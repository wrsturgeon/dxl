use {
    crate::{
        compiletime::instruction::Instruction,
        constants::{WrongByte, C16, C8},
        crc::Crc,
        parse::Parse,
    },
    core::marker::PhantomData,
};

pub enum Error<Insn: Instruction> {
    Parsing(ParseOrCrcError<Insn>),
    Software(SoftwareError),
    Hardware,
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(test, derive(strum_macros::VariantArray))]
pub enum SoftwareError {
    ResultFail = 0x01,
    InstructionError = 0x02,
    CrcError = 0x03,
    DataRangeError = 0x04,
    DataLengthError = 0x05,
    DataLimitError = 0x06,
    AccessError = 0x07,
}

#[derive(Debug, PartialEq)]
pub struct InvalidSoftwareError {
    byte_without_msb: u8,
}

impl SoftwareError {
    #[inline]
    pub fn check(byte: u8) -> Result<Option<Self>, InvalidSoftwareError> {
        match byte & 0x7F {
            0x00 => Ok(None),
            0x01 => Ok(Some(Self::ResultFail)),
            0x02 => Ok(Some(Self::InstructionError)),
            0x03 => Ok(Some(Self::CrcError)),
            0x04 => Ok(Some(Self::DataRangeError)),
            0x05 => Ok(Some(Self::DataLengthError)),
            0x06 => Ok(Some(Self::DataLimitError)),
            0x07 => Ok(Some(Self::AccessError)),
            byte_without_msb => Err(InvalidSoftwareError { byte_without_msb }),
        }
    }
}

pub enum ParseError<Insn: Instruction> {
    WrongHeader(WrongByte),
    WrongReservedByte(WrongByte),
    WrongId(WrongByte),
    WrongLength(WrongByte),
    WrongInstruction(WrongByte),
    InstructionSpecific(<Insn::Recv as Parse<u8>>::Error),
    InvalidSoftwareError(InvalidSoftwareError),
}

pub enum ParseOrCrcError<Insn: Instruction> {
    Crc { expected: u16, actual: u16 },
    Parse(ParseError<Insn>),
}

pub struct WithErrorCode<Insn: Instruction> {
    software_error: Option<SoftwareError>,
    hardware_error: bool,
    parameters: <Insn::Recv as Parse<u8>>::Output,
    expected_crc: u16,
}

// #[repr(C, packed)]
pub struct WithoutCrc<Insn: Instruction, const ID: u8>(PhantomData<Insn>)
where
    [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:;
/*
{
    header: (C8<0xFF>, C8<0xFF>, C8<0xFD>),
    reserved: C8<0x00>,
    id: C8<ID>,
    length: C16<{ core::mem::size_of::<Insn::Recv>() as u16 + 4 }>,
    instruction: C8<0x55>,
    error: u8,
    parameters: Insn::Recv,
}
*/

impl<Insn: Instruction, const ID: u8> WithoutCrc<Insn, ID>
where
    [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:,
{
    #[inline]
    const fn crc_init() -> Crc {
        let mut crc = Crc::new();
        crc.push(0xFF);
        crc.push(0xFF);
        crc.push(0xFD);
        crc.push(0x00);
        crc.push(ID);
        {
            let [lo, hi] = const { (core::mem::size_of::<Insn::Recv>() as u16 + 4).to_le_bytes() };
            crc.push(lo);
            crc.push(hi);
        }
        crc.push(0x55);
        crc
    }

    /*
    #[inline]
    fn crc(&self) -> u16 {
        let mut crc = const { Self::crc_init() };
        crc.push(self.error);
        let ptr: *const u8 = self as *const _ as *const u8;
        for i in (core::mem::size_of::<Self>() - core::mem::size_of::<Insn>())
            ..(core::mem::size_of::<Self>())
        {
            crc.push(unsafe { *ptr.byte_offset(i as isize) })
        }
        crc.collapse()
    }
    */
}

impl<Insn: Instruction, const ID: u8> Parse<u8> for WithoutCrc<Insn, ID>
where
    [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) & 0xFF) as u8 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) >> 8) as u8 } as usize]:,
{
    type Output = WithErrorCode<Insn>;
    type Error = ParseError<Insn>;

    #[inline]
    async fn parse<Callback: FnMut(u8), F: Future<Output = u8>, Next: FnMut() -> F>(
        next: &mut Next,
        callback: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
        let ((), (), ()) = <(C8<0xFF>, C8<0xFF>, C8<0xFD>) as Parse<u8>>::parse(next, callback)
            .await
            .map_err(Self::Error::WrongHeader)?;
        let () = <C8<0x00> as Parse<u8>>::parse(next, callback)
            .await
            .map_err(Self::Error::WrongReservedByte)?;
        let () = <C8<ID> as Parse<u8>>::parse(next, callback)
            .await
            .map_err(Self::Error::WrongId)?;
        let () = <C16<{ core::mem::size_of::<Insn::Recv>() as u16 + 4 }> as Parse<u8>>::parse(
            next, callback,
        )
        .await
        .map_err(Self::Error::WrongLength)?;
        let () = <C8<0x55> as Parse<u8>>::parse(next, callback)
            .await
            .map_err(Self::Error::WrongInstruction)?;
        let mut crc_state = const { Self::crc_init() };
        let (software_error, hardware_error) = {
            let byte: u8 = next().await;
            callback(byte);
            crc_state.push(byte);
            (
                SoftwareError::check(byte).map_err(Self::Error::InvalidSoftwareError)?,
                (byte & 0x80) != 0,
            )
        };
        let parameters = <Insn::Recv as Parse<u8>>::parse(next, &mut |byte| crc_state.push(byte))
            .await
            .map_err(Self::Error::InstructionSpecific)?;
        let expected_crc = crc_state.collapse();
        Ok(Self::Output {
            software_error,
            hardware_error,
            parameters,
            expected_crc,
        })
    }
}

// #[repr(C, packed)]
pub struct WithCrc<Insn: Instruction, const ID: u8>(PhantomData<Insn>)
where
    [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) & 0xFF) as u8 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) >> 8) as u8 } as usize]:;
/*
{
    without_crc: WithoutCrc<Insn, ID>,
    crc: [u8; 2],
}
*/

impl<Insn: Instruction, const ID: u8> Parse<u8> for WithCrc<Insn, ID>
where
    [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) & 0xFF) as u8 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) >> 8) as u8 } as usize]:,
{
    type Output = <Insn::Recv as Parse<u8>>::Output;
    type Error = Error<Insn>;

    #[inline]
    async fn parse<Callback: FnMut(u8), F: Future<Output = u8>, Next: FnMut() -> F>(
        next: &mut Next,
        callback: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
        let WithErrorCode {
            software_error,
            hardware_error,
            parameters,
            expected_crc,
        } = WithoutCrc::<Insn, ID>::parse(next, callback)
            .await
            .map_err(|e| Error::Parsing(ParseOrCrcError::Parse(e)))?;
        let Ok(actual_crc) = u16::parse(next, callback).await;
        if actual_crc != expected_crc {
            return Err(Error::Parsing(ParseOrCrcError::Crc {
                expected: expected_crc,
                actual: actual_crc,
            }));
        }
        if let Some(error) = software_error {
            return Err(Error::Software(error));
        }
        if hardware_error {
            return Err(Error::Hardware);
        }
        Ok(parameters)
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        quickcheck::{Arbitrary, Gen, TestResult},
        quickcheck_macros::quickcheck,
        strum::VariantArray,
    };

    impl Arbitrary for SoftwareError {
        #[inline]
        fn arbitrary(g: &mut Gen) -> Self {
            let i = usize::arbitrary(g) % const { Self::VARIANTS.len() };
            Self::VARIANTS[i]
        }

        #[inline]
        fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
            let i = Self::VARIANTS
                .binary_search(self)
                .expect("Invalid enum variant");
            Box::new(i.shrink().filter_map(|j| Self::VARIANTS.get(j).map(|&e| e)))
        }
    }

    #[quickcheck]
    fn byte_software_error_roundtrip(byte: u8) -> TestResult {
        let Ok(Some(software_error)) = SoftwareError::check(byte) else {
            return TestResult::discard();
        };
        let roundtrip = software_error as u8;
        let byte_without_msb = byte & 0x7F;
        if roundtrip == byte_without_msb {
            TestResult::passed()
        } else {
            TestResult::error(format!("Invalid software-error byte logic: {byte:#?} -> {software_error:#?} -> {roundtrip:#?} =/= {byte_without_msb:#?}"))
        }
    }

    #[quickcheck]
    fn software_error_byte_roundtrip(software_error: SoftwareError) -> TestResult {
        let byte_without_msb = software_error as u8;
        let result = SoftwareError::check(byte_without_msb);
        if result == Ok(Some(software_error)) {
            TestResult::passed()
        } else {
            TestResult::error(format!("Invalid software-error byte logic: {software_error:#?} -> {byte_without_msb:#?} -> {result:#?} =/= Ok(Some({software_error:#?}))"))
        }
    }
}
