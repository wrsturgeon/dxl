use {
    crate::{
        constants::{WrongByte, C16, C8},
        crc::Crc,
        instruction::Instruction,
        parse::Parse,
        stream::{self, Stream},
    },
    core::{fmt, marker::PhantomData},
};

pub enum Error<Insn: Instruction> {
    Parsing(ParseOrCrcError<Insn>),
    Software(SoftwareError),
    Hardware,
}

impl<Insn: Instruction> fmt::Display for Error<Insn> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Parsing(ref e) => fmt::Display::fmt(e, f),
            Self::Software(ref e) => fmt::Display::fmt(e, f),
            Self::Hardware => write!(
                f,
                "Hardware error reported (details require a separate request)"
            ),
        }
    }
}

#[repr(u8)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

impl fmt::Display for SoftwareError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
Self::ResultFail => write!(f, "Actuator could not process the packet"),
Self::InstructionError => write!(f, "Either the actuator did not recognize the instruction byte or it received `Action` without `RegWrite`"),
Self::CrcError => write!(f, "Actuator disagrees about CRC calculation (likely a corrupted packet)"),
Self::DataRangeError => write!(f, "Data to be written is too long to fit in the specified range of memory"),
Self::DataLengthError => write!(f, "Data to be written is too short to fit in the specified range of memory"),
Self::DataLimitError => write!(f, "Data out of range"),
Self::AccessError => write!(f, "Couldn't write (either tried to write to EEPROM with torque enabled, tried to write to read-only memory, or tried to read from write-only memory)"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct InvalidSoftwareError {
    byte_without_msb: u8,
}

impl fmt::Display for InvalidSoftwareError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            ref byte_without_msb,
        } = *self;
        write!(f, "Invalid software error: `{byte_without_msb:02X?}`")
    }
}

impl SoftwareError {
    #[inline]
    fn check(byte: u8) -> Result<Option<Self>, InvalidSoftwareError> {
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
    WrongReserved(WrongByte),
    WrongId(WrongByte),
    WrongLength(WrongByte),
    WrongInstruction(WrongByte),
    InstructionSpecific(<Insn::Recv as Parse<u8>>::Error),
    InvalidSoftwareError(InvalidSoftwareError),
}

impl<Insn: Instruction> fmt::Display for ParseError<Insn> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::WrongHeader(ref e) => write!(f, "Wrong header byte: {e}"),
            Self::WrongReserved(ref e) => write!(f, "Wrong reserved byte: {e}"),
            Self::WrongId(ref e) => write!(f, "Wrong ID: {e}"),
            Self::WrongLength(ref e) => write!(f, "Wrong length: {e}"),
            Self::WrongInstruction(ref e) => write!(f, "Wrong instruction: {e}"),
            Self::InstructionSpecific(ref e) => fmt::Display::fmt(e, f),
            Self::InvalidSoftwareError(ref e) => fmt::Display::fmt(e, f),
        }
    }
}

pub enum ParseOrCrcError<Insn: Instruction> {
    Crc { expected: u16, actual: u16 },
    Parse(ParseError<Insn>),
}

impl<Insn: Instruction> fmt::Display for ParseOrCrcError<Insn> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Parse(ref e) => fmt::Display::fmt(e, f),
            Self::Crc { expected, actual } => write!(
                f,
                "Expected CRC to be `{expected:02X?}` but received `{actual:02X?}`"
            ),
        }
    }
}

struct WithErrorCode<Insn: Instruction> {
    software_error: Option<SoftwareError>,
    hardware_error: bool,
    parameters: <Insn::Recv as Parse<u8>>::Output,
    expected_crc: u16,
}

struct WithoutCrc<Insn: Instruction, const ID: u8>(PhantomData<Insn>)
where
    [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:;

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
    async fn parse<S: Stream<Item = u8>>(s: &mut S) -> Result<Self::Output, Self::Error> {
        let ((), (), ()) = <(C8<0xFF>, C8<0xFF>, C8<0xFD>) as Parse<u8>>::parse(s)
            .await
            .map_err(Self::Error::WrongHeader)?;
        let () = <C8<0x00> as Parse<u8>>::parse(s)
            .await
            .map_err(Self::Error::WrongReserved)?;
        let () = <C8<ID> as Parse<u8>>::parse(s)
            .await
            .map_err(Self::Error::WrongId)?;
        let () = <C16<{ core::mem::size_of::<Insn::Recv>() as u16 + 4 }> as Parse<u8>>::parse(s)
            .await
            .map_err(Self::Error::WrongLength)?;
        let () = <C8<0x55> as Parse<u8>>::parse(s)
            .await
            .map_err(Self::Error::WrongInstruction)?;
        let mut crc_state = const { Self::crc_init() };
        let (software_error, hardware_error) = {
            let byte: u8 = s.next().await;
            crc_state.push(byte);
            (
                SoftwareError::check(byte).map_err(Self::Error::InvalidSoftwareError)?,
                (byte & 0x80) != 0,
            )
        };
        let parameters = <Insn::Recv as Parse<u8>>::parse(&mut stream::WithCrc {
            internal: s,
            crc: &mut crc_state,
        })
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

pub struct WithCrc<Insn: Instruction, const ID: u8>(PhantomData<Insn>)
where
    [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) & 0xFF) as u8 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) >> 8) as u8 } as usize]:;

impl<Insn: Instruction, const ID: u8> Parse<u8> for WithCrc<Insn, ID>
where
    [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) & 0xFF) as u8 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) >> 8) as u8 } as usize]:,
{
    type Output = <Insn::Recv as Parse<u8>>::Output;
    type Error = Error<Insn>;

    #[inline]
    async fn parse<S: Stream<Item = u8>>(s: &mut S) -> Result<Self::Output, Self::Error> {
        let WithErrorCode {
            software_error,
            hardware_error,
            parameters,
            expected_crc,
        } = WithoutCrc::<Insn, ID>::parse(s)
            .await
            .map_err(|e| Error::Parsing(ParseOrCrcError::Parse(e)))?;
        let Ok(actual_crc) = u16::parse(s).await;
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
        crate::{instruction, stream, test_util},
        core::pin::pin,
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

    #[test]
    fn parse_ping() {
        const EXPECTED: instruction::recv::Ping = instruction::recv::Ping {
            model_number: 1030,
            firmware_version: 38,
        };
        let status_packet = [
            0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let future = crate::packet::parse::<instruction::Ping, 0x01>(&mut s);
        let actual = match test_util::trivial_future(pin!(future)) {
            Ok(ok) => ok,
            Err(e) => panic!("{e}"),
        };
        assert_eq!(
            actual, EXPECTED,
            "Expected `{EXPECTED:02X?}` but got `{actual:02X?}`",
        );
    }
}
