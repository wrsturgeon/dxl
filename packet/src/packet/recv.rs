use crate::{crc::Crc, parse, recv, Instruction};

#[repr(u8)]
#[non_exhaustive]
#[derive(defmt::Format)]
#[cfg_attr(
    test,
    derive(
        Clone,
        Copy,
        Debug,
        Eq,
        Ord,
        PartialEq,
        PartialOrd,
        strum_macros::VariantArray
    )
)]
pub enum SoftwareError {
    ResultFail = 0x01,
    InstructionError = 0x02,
    CrcError = 0x03,
    DataRangeError = 0x04,
    DataLengthError = 0x05,
    DataLimitError = 0x06,
    AccessError = 0x07,
}

/*
impl defmt::Format for SoftwareError {
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
*/

#[derive(defmt::Format)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct InvalidSoftwareError {
    byte_without_msb: u8,
}

/*
impl defmt::Format for InvalidSoftwareError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            ref byte_without_msb,
        } = *self;
        write!(f, "Invalid software error: `{byte_without_msb:02X?}`")
    }
}
*/

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

pub struct Mismatch8 {
    expected: u8,
    actual: u8,
}

impl defmt::Format for Mismatch8 {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        let Self {
            ref expected,
            ref actual,
        } = *self;
        defmt::write!(f, "Expected `x{:X}` but received `x{:X}`", expected, actual)
    }
}

pub struct Mismatch16 {
    expected: u16,
    actual: u16,
}

impl defmt::Format for Mismatch16 {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        let Self {
            ref expected,
            ref actual,
        } = *self;
        defmt::write!(f, "Expected `x{:X}` but received `x{:X}`", expected, actual)
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum ParseError<InstructionSpecific: defmt::Format> {
    WrongFirstHeaderByte(Mismatch8),
    WrongSecondHeaderByte(Mismatch8),
    WrongThirdHeaderByte(Mismatch8),
    WrongReservedByte(Mismatch8),
    WrongId(Mismatch8),
    WrongLength(Mismatch16),
    WrongInstruction(Mismatch8),
    InvalidSoftwareError(InvalidSoftwareError),
    InstructionSpecific(InstructionSpecific),
}

impl<InstructionSpecific: defmt::Format> defmt::Format for ParseError<InstructionSpecific> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::WrongFirstHeaderByte(ref e) => defmt::write!(f, "Wrong first header byte: {}", e),
            Self::WrongSecondHeaderByte(ref e) => {
                defmt::write!(f, "Wrong second header byte: {}", e)
            }
            Self::WrongThirdHeaderByte(ref e) => defmt::write!(f, "Wrong third header byte: {}", e),
            Self::WrongReservedByte(ref e) => defmt::write!(f, "Wrong reserved byte: {}", e),
            Self::WrongId(ref e) => defmt::write!(f, "Wrong ID: {}", e),
            Self::WrongLength(ref e) => defmt::write!(f, "Wrong length: {}", e),
            Self::WrongInstruction(ref e) => defmt::write!(f, "Wrong instruction: {}", e),
            Self::InvalidSoftwareError(ref e) => defmt::write!(f, "Invalid software error: {}", e),
            Self::InstructionSpecific(ref e) => {
                defmt::write!(f, "Instruction-specific error: {}", e)
            }
        }
    }
}

/*
impl<InstructionSpecific: defmt::Format> defmt::Format for ParseError<InstructionSpecific> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::WrongFirstHeaderByte { ref expected, ref actual } => write!(f, "Wrong first header byte: expected `0x{expected:02X?}` but received `0x{actual:02X?}`"),
            Self::WrongSecondHeaderByte { ref expected, ref actual } => write!(f, "Wrong second header byte: expected `0x{expected:02X?}` but received `0x{actual:02X?}`"),
            Self::WrongThirdHeaderByte { ref expected, ref actual } => write!(f, "Wrong third header byte: expected `0x{expected:02X?}` but received `0x{actual:02X?}`"),
            Self::WrongReservedByte { ref expected, ref actual } => write!(f, "Wrong reserved byte: expected `0x{expected:02X?}` but received `0x{actual:02X?}`"),
            Self::WrongId { ref expected, ref actual } => write!(f, "Wrong ID: expected `0x{expected}` but received `0x{actual}`"),
            Self::WrongLength { ref expected, ref actual } => write!(f, "Wrong length: expected `0x{expected}` but received `0x{actual}`"),
            Self::WrongInstruction { ref expected, ref actual } => write!(f, "Wrong instruction: expected `0x{expected:02X?}` but received `0x{actual:02X?}`"),
            Self::InstructionSpecific(ref e) => defmt::Format::fmt(e, f),
            Self::InvalidSoftwareError(ref e) => defmt::Format::fmt(e, f),
        }
    }
}
*/

#[derive(defmt::Format)]
pub struct WithHardwareErrorStatus<Output> {
    output: Output,
    expected_crc: u16,
    hardware_error: bool,
}

pub enum WithoutCrc<Insn: Instruction, const ID: u8> {
    Header1,
    Header2,
    Header3,
    Reserved,
    Id,
    LengthLo,
    LengthHi {
        length_lo: u8,
    },
    Instruction {
        length: u16,
    },
    Error {
        length: u16,
    },
    SoftwareError {
        crc_state: Crc,
        hardware_error: bool,
        software_error: SoftwareError,
        length: u16,
        count: u16,
    },
    Parameters {
        state: <<<Insn as Instruction>::Recv as recv::Receive>::Parser as parse::MaybeParse<
            u8,
            <Insn as Instruction>::Recv,
        >>::Parser,
        crc_state: Crc,
        hardware_error: bool,
    },
}

impl<Insn: Instruction, const ID: u8> WithoutCrc<Insn, ID> {
    #[inline]
    const fn crc_init() -> Crc {
        let mut crc = Crc::new();
        crc.push(0xFF);
        crc.push(0xFF);
        crc.push(0xFD);
        crc.push(0x00);
        crc.push(ID);
        {
            let [lo, hi] =
                const { ((<Insn::Recv as crate::recv::Receive>::BYTES + 4) as u16).to_le_bytes() };
            crc.push(lo);
            crc.push(hi);
        }
        crc.push(0x55);
        crc
    }
}

impl<Insn: Instruction, const ID: u8> parse::State<u8> for WithoutCrc<Insn, ID> {
    type Output = WithHardwareErrorStatus<Result<Insn::Recv, SoftwareError>>;
    type SideEffect = ();
    type Error = ParseError<
        <<<<Insn as Instruction>::Recv as recv::Receive>::Parser as parse::MaybeParse<
            u8,
            <Insn as Instruction>::Recv,
        >>::Parser as parse::State<u8>>::Error,
    >;

    const INIT: Self = Self::Header1;

    #[inline]
    fn push(
        self,
        input: u8,
    ) -> Result<
        parse::Status<Self::Output, (Self, Self::SideEffect)>,
        <Self as parse::State<u8>>::Error,
    > {
        macro_rules! expect {
            ($byte:expr, $err:ident, $next:ident) => {
                if input == $byte {
                    Self::$next
                } else {
                    return Err(ParseError::$err(Mismatch8 {
                        expected: $byte,
                        actual: input,
                    }));
                }
            };
        }

        Ok(parse::Status::Incomplete((
            match self {
                Self::Header1 => expect!(0xFF, WrongFirstHeaderByte, Header2),
                Self::Header2 => expect!(0xFF, WrongSecondHeaderByte, Header3),
                Self::Header3 => expect!(0xFD, WrongThirdHeaderByte, Reserved),
                Self::Reserved => expect!(0x00, WrongReservedByte, Id),
                Self::Id => expect!(ID, WrongId, LengthLo),
                Self::LengthLo => Self::LengthHi { length_lo: input },
                Self::LengthHi { length_lo } => Self::Instruction {
                    length: u16::from_le_bytes([length_lo, input]),
                },
                Self::Instruction { length } => {
                    if input == 0x55 {
                        Self::Error { length }
                    } else {
                        return Err(ParseError::WrongInstruction(Mismatch8 {
                            expected: 0x55,
                            actual: input,
                        }));
                    }
                }
                Self::Error { length } => {
                    let mut crc_state = const { WithoutCrc::<Insn, ID>::crc_init() };
                    let () = crc_state.push(input);
                    let hardware_error = (input & 0x80) != 0;
                    if let Some(software_error) =
                        SoftwareError::check(input).map_err(ParseError::InvalidSoftwareError)?
                    {
                        if length <= 4 {
                            return Ok(parse::Status::Complete(WithHardwareErrorStatus {
                                output: Err(software_error),
                                expected_crc: crc_state.collapse(),
                                hardware_error,
                            }));
                        }
                        Self::SoftwareError {
                            crc_state,
                            software_error,
                            hardware_error,
                            length: length - 4,
                            count: 0,
                        }
                    } else {
                        // THEN, after we know it's not just short because of a misunderstood packet,
                        // check to make sure that the length of the packet matches our expectation:
                        if length
                            != const { (<Insn::Recv as crate::recv::Receive>::BYTES + 4) as u16 }
                        {
                            let mismatch = Mismatch16 {
                                actual: length,
                                expected: const {
                                    (<Insn::Recv as crate::recv::Receive>::BYTES + 4) as u16
                                },
                            };
                            return Err(ParseError::WrongLength(mismatch));
                        }
                        match <<<Insn as Instruction>::Recv as recv::Receive>::Parser as parse::MaybeParse<u8, _>>::INIT {
                        parse::Status::Complete(output) => {
                            return Ok(parse::Status::Complete(WithHardwareErrorStatus {
                                output: Ok(output),
                                expected_crc: crc_state.collapse(),
                                hardware_error,
                            }))
                        }
                        parse::Status::Incomplete(state) => Self::Parameters {
                            state,
                            crc_state,
                            hardware_error,
                        },
                    }
                    }
                }
                Self::SoftwareError {
                    mut crc_state,
                    hardware_error,
                    software_error,
                    length,
                    mut count,
                } => {
                    let () = crc_state.push(input);
                    count += 1;
                    if count == length {
                        return Ok(parse::Status::Complete(WithHardwareErrorStatus {
                            output: Err(software_error),
                            expected_crc: crc_state.collapse(),
                            hardware_error,
                        }));
                    } else {
                        Self::SoftwareError {
                            crc_state,
                            hardware_error,
                            software_error,
                            length,
                            count,
                        }
                    }
                }
                Self::Parameters {
                    state,
                    mut crc_state,
                    hardware_error,
                } => {
                    let () = crc_state.push(input);
                    match state.push(input).map_err(ParseError::InstructionSpecific)? {
                        parse::Status::Complete(output) => {
                            return Ok(parse::Status::Complete(WithHardwareErrorStatus {
                                output: Ok(output),
                                expected_crc: crc_state.collapse(),
                                hardware_error,
                            }))
                        }
                        parse::Status::Incomplete((state, _)) => Self::Parameters {
                            state,
                            crc_state,
                            hardware_error,
                        },
                    }
                }
            },
            (),
        )))
    }
}

impl<Insn: Instruction, const ID: u8> defmt::Format for WithoutCrc<Insn, ID> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::Header1 => defmt::write!(f, "Waiting for the first header byte"),
            Self::Header2 => defmt::write!(f, "Waiting for the second header byte"),
            Self::Header3 => defmt::write!(f, "Waiting for the third header byte"),
            Self::Reserved => defmt::write!(f, "Waiting for the reserved byte"),
            Self::Id => defmt::write!(f, "Waiting for the ID"),
            Self::LengthLo => defmt::write!(f, "Waiting for the first length byte"),
            Self::LengthHi { length_lo } => defmt::write!(
                f,
                "Waiting for the second length byte (already received the first: `x{:X}`)",
                length_lo
            ),
            Self::Instruction { length } => defmt::write!(
                f,
                "Waiting for the instruction byte (already saw length of {})",
                length
            ),
            Self::Error { length } => defmt::write!(
                f,
                "Waiting for the error byte (already saw length of {})",
                length
            ),
            Self::SoftwareError { ref crc_state, ref software_error, hardware_error, length, count } => defmt::write!(f, "Waiting to discard instruction-specific parameter #{}/{}, since we received a software error: {} (crc_state = {:X}, hardware_error: {:X})", count, length, software_error, crc_state, hardware_error,),
            Self::Parameters {
                state: _,
                ref crc_state,
                hardware_error,
            } => defmt::write!(
                f,
                "Parsing instruction-specific parameters (crc_state = {:X}, hardware_error = {:X})",
                crc_state,
                hardware_error,
            ),
        }
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub enum Error<Output, E: defmt::Format> {
    Parsing(ParseError<E>),
    Crc(Mismatch16),
    Software(SoftwareError),
    Hardware(Output),
}

impl<Output, E: defmt::Format> defmt::Format for Error<Output, E> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::Parsing(ref e) => defmt::write!(f, "Parsing error: {}", e),
            Self::Crc(ref e) => defmt::write!(f, "CRC error: {}", e),
            Self::Software(ref e) => defmt::write!(f, "Software error: {}", e),
            Self::Hardware(_) => {
                defmt::write!(f, "Hardware error (details require a separate request)",)
            }
        }
    }
}

/*
impl<Output, E: defmt::Format> defmt::Format for Error<Output, E> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Parsing(ref e) => defmt::write!(f, "Parsing error: {e=ParseError:}"),
            Self::Crc { expected, actual } => write!(
                f,
                "CRC mismatch: expected `0x{expected:02X?}` but received `0x{actual:02X?}`",
            ),
            Self::Software(ref e) => write!(f, "Software error reported: {e}"),
            Self::Hardware(_) => write!(
                f,
                "Hardware error reported (details require a separate request)",
            ),
        }
    }
}
*/

pub enum WithCrc<Insn: Instruction, const ID: u8> {
    BeforeCrc {
        state: WithoutCrc<Insn, ID>,
    },
    FirstCrcByte {
        payload: WithHardwareErrorStatus<Result<Insn::Recv, SoftwareError>>,
    },
    SecondCrcByte {
        first_crc_byte: u8,
        payload: WithHardwareErrorStatus<Result<Insn::Recv, SoftwareError>>,
    },
}

impl<Insn: Instruction, const ID: u8> parse::State<u8> for WithCrc<Insn, ID> {
    type Output = Insn::Recv;
    type SideEffect = ();
    type Error = Error<
        Self::Output,
        <<<<Insn as Instruction>::Recv as recv::Receive>::Parser as parse::MaybeParse<
            u8,
            <Insn as Instruction>::Recv,
        >>::Parser as parse::State<u8>>::Error,
    >;

    const INIT: Self = Self::BeforeCrc {
        state: WithoutCrc::<Insn, ID>::INIT,
    };

    #[inline(always)]
    fn push(
        self,
        input: u8,
    ) -> Result<parse::Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
        Ok(parse::Status::Incomplete((
            match self {
                Self::BeforeCrc { state } => match state.push(input).map_err(Error::Parsing)? {
                    parse::Status::Complete(payload) => {
                        /*
                        defmt::debug!(
                            "Finished parsing packet body into {}; waiting for CRC...",
                            payload
                        );
                        */
                        Self::FirstCrcByte { payload }
                    }
                    parse::Status::Incomplete((new_state, _)) => {
                        Self::BeforeCrc { state: new_state }
                    }
                },
                Self::FirstCrcByte { payload } => {
                    // defmt::debug!("First CRC byte: `x{:X}`", input);
                    Self::SecondCrcByte {
                        first_crc_byte: input,
                        payload,
                    }
                }
                Self::SecondCrcByte {
                    first_crc_byte,
                    payload,
                } => {
                    // defmt::debug!("Second CRC byte: `x{:X}`", input);
                    let WithHardwareErrorStatus {
                        output,
                        expected_crc,
                        hardware_error,
                    } = payload;
                    {
                        let actual_crc = u16::from_le_bytes([first_crc_byte, input]);
                        // defmt::debug!("Full CRC: `x{:X}`", actual_crc);
                        if actual_crc != expected_crc {
                            return Err(Error::Crc(Mismatch16 {
                                expected: expected_crc,
                                actual: actual_crc,
                            }));
                        }
                    }
                    let ok = output.map_err(Error::Software)?;
                    return if hardware_error {
                        Err(Error::Hardware(ok))
                    } else {
                        Ok(parse::Status::Complete(ok))
                    };
                }
            },
            (),
        )))
    }
}

impl<Insn: Instruction, const ID: u8> defmt::Format for WithCrc<Insn, ID> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::BeforeCrc { ref state } => defmt::Format::format(state, f),
            Self::FirstCrcByte { ref payload } => defmt::write!(f, "Waiting for the first CRC byte (already parsed body into {})", payload),
            Self::SecondCrcByte { ref payload, first_crc_byte } => defmt::write!(f, "Waiting for the second CRC byte (already parsed body into {} and received the first: `0x{:X}`)", payload, first_crc_byte),
        }
    }
}

#[cfg_attr(test, derive(Debug))]
pub enum PersistentError<Output> {
    Software(SoftwareError),
    Hardware(Output),
}

impl<Output> defmt::Format for PersistentError<Output> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::Software(ref e) => defmt::write!(f, "Software({})", e),
            Self::Hardware(_) => defmt::write!(f, "Hardware(..)"),
        }
    }
}

#[derive(defmt::Format)]
pub struct Persistent<Insn: Instruction, const ID: u8>(pub WithCrc<Insn, ID>);

impl<Insn: Instruction, const ID: u8> parse::State<u8> for Persistent<Insn, ID> {
    type Output = Insn::Recv;
    type SideEffect = ();
    type Error = PersistentError<Self::Output>;

    const INIT: Self = Self(WithCrc::INIT);

    #[inline(always)]
    fn push(
        self,
        input: u8,
    ) -> Result<parse::Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
        match self.0.push(input) {
            Ok(parse::Status::Complete(complete)) => Ok(parse::Status::Complete(complete)),
            Ok(parse::Status::Incomplete((incomplete, ()))) => {
                Ok(parse::Status::Incomplete((Self(incomplete), ())))
            }
            Err(Error::Parsing(e)) => {
                defmt::warn!("Parsing error ({}); trying again...", e);
                Ok(parse::Status::Incomplete((Self::INIT, ())))
            }
            Err(Error::Crc(e)) => {
                defmt::warn!("CRC error ({}); trying again...", e);
                Ok(parse::Status::Incomplete((Self::INIT, ())))
            }
            Err(Error::Software(e)) => Err(PersistentError::Software(e)),
            Err(Error::Hardware(e)) => Err(PersistentError::Hardware(e)),
        }
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{
            parse::State,
            recv,
            stream::{self, Stream},
        },
        core::{pin::pin, task},
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
        const EXPECTED: recv::Ping = recv::Ping {
            model_number: 1030,
            firmware_version: 38,
        };
        let status_packet = [
            0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = WithCrc::<crate::send::Ping, 1>::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input).unwrap() {
                parse::Status::Incomplete((updated, ())) => updated,
                parse::Status::Complete(actual) => {
                    assert_eq!(
                        actual, EXPECTED,
                        "Expected `{EXPECTED:02X?}` but got `{actual:02X?}`",
                    );
                    return;
                }
            }
        }
    }

    #[test]
    fn parse_ping_wrong_header_1() {
        let status_packet = [
            0xFE, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = WithCrc::<crate::send::Ping, 1>::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input) {
                Ok(parse::Status::Incomplete((updated, ()))) => updated,
                Ok(parse::Status::Complete(actual)) => {
                    panic!("Expected an error but received `{actual:02X?}`")
                }
                Err(e) => {
                    assert_eq!(
                        e,
                        Error::Parsing(ParseError::WrongFirstHeaderByte {
                            expected: 0xFF,
                            actual: 0xFE
                        })
                    );
                    return;
                }
            }
        }
    }

    #[test]
    fn parse_ping_wrong_header_2() {
        let status_packet = [
            0xFF, 0xFE, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = WithCrc::<crate::send::Ping, 1>::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input) {
                Ok(parse::Status::Incomplete((updated, ()))) => updated,
                Ok(parse::Status::Complete(actual)) => {
                    panic!("Expected an error but received `{actual:02X?}`")
                }
                Err(e) => {
                    assert_eq!(
                        e,
                        Error::Parsing(ParseError::WrongSecondHeaderByte {
                            expected: 0xFF,
                            actual: 0xFE,
                        })
                    );
                    return;
                }
            }
        }
    }

    #[test]
    fn parse_ping_wrong_header_3() {
        let status_packet = [
            0xFF, 0xFF, 0xFF, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = WithCrc::<crate::send::Ping, 1>::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input) {
                Ok(parse::Status::Incomplete((updated, ()))) => updated,
                Ok(parse::Status::Complete(actual)) => {
                    panic!("Expected an error but received `{actual:02X?}`")
                }
                Err(e) => {
                    assert_eq!(
                        e,
                        Error::Parsing(ParseError::WrongThirdHeaderByte {
                            expected: 0xFD,
                            actual: 0xFF,
                        })
                    );
                    return;
                }
            }
        }
    }

    #[test]
    fn parse_ping_wrong_reserved() {
        let status_packet = [
            0xFF, 0xFF, 0xFD, 0x01, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = WithCrc::<crate::send::Ping, 1>::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input) {
                Ok(parse::Status::Incomplete((updated, ()))) => updated,
                Ok(parse::Status::Complete(actual)) => {
                    panic!("Expected an error but received `{actual:02X?}`")
                }
                Err(e) => {
                    assert_eq!(
                        e,
                        Error::Parsing(ParseError::WrongReservedByte {
                            expected: 0x00,
                            actual: 0x01
                        })
                    );
                    return;
                }
            }
        }
    }

    #[test]
    fn parse_ping_wrong_id() {
        let status_packet = [
            0xFF, 0xFF, 0xFD, 0x00, 0x02, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = WithCrc::<crate::send::Ping, 1>::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input) {
                Ok(parse::Status::Incomplete((updated, ()))) => updated,
                Ok(parse::Status::Complete(actual)) => {
                    panic!("Expected an error but received `{actual:02X?}`")
                }
                Err(e) => {
                    assert_eq!(
                        e,
                        Error::Parsing(ParseError::WrongId {
                            expected: 0x01,
                            actual: 0x02,
                        })
                    );
                    return;
                }
            }
        }
    }

    #[test]
    fn parse_ping_wrong_length_1() {
        let status_packet = [
            0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x08, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = WithCrc::<crate::send::Ping, 1>::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input) {
                Ok(parse::Status::Incomplete((updated, ()))) => updated,
                Ok(parse::Status::Complete(actual)) => {
                    panic!("Expected an error but received `{actual:02X?}`")
                }
                Err(e) => {
                    assert_eq!(
                        e,
                        Error::Parsing(ParseError::WrongLength {
                            expected: 7,
                            actual: 8
                        })
                    );
                    return;
                }
            }
        }
    }

    #[test]
    fn parse_ping_wrong_length_2() {
        let status_packet = [
            0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x01, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = WithCrc::<crate::send::Ping, 1>::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input) {
                Ok(parse::Status::Incomplete((updated, ()))) => updated,
                Ok(parse::Status::Complete(actual)) => {
                    panic!("Expected an error but received `{actual:02X?}`")
                }
                Err(e) => {
                    assert_eq!(
                        e,
                        Error::Parsing(ParseError::WrongLength {
                            expected: 7,
                            actual: 263
                        })
                    );
                    return;
                }
            }
        }
    }

    #[test]
    fn parse_ping_wrong_insn() {
        let status_packet = [
            0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x56, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = WithCrc::<crate::send::Ping, 1>::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input) {
                Ok(parse::Status::Incomplete((updated, ()))) => updated,
                Ok(parse::Status::Complete(actual)) => {
                    panic!("Expected an error but received `{actual:02X?}`")
                }
                Err(e) => {
                    assert_eq!(
                        e,
                        Error::Parsing(ParseError::WrongInstruction {
                            expected: 0x55,
                            actual: 0x56
                        })
                    );
                    return;
                }
            }
        }
    }

    #[test]
    fn parse_ping_wrong_crc() {
        let status_packet = [
            0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5E,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = WithCrc::<crate::send::Ping, 1>::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input) {
                Ok(parse::Status::Incomplete((updated, ()))) => updated,
                Ok(parse::Status::Complete(actual)) => {
                    panic!("Expected an error but received `{actual:02X?}`")
                }
                Err(e) => {
                    assert_eq!(
                        e,
                        Error::Crc {
                            expected: 0x5D65,
                            actual: 0x5E65
                        }
                    );
                    return;
                }
            }
        }
    }

    #[quickcheck]
    fn parse_ping_persistent(offset: u8) {
        const EXPECTED: recv::Ping = recv::Ping {
            model_number: 1030,
            firmware_version: 38,
        };
        let status_packet = [
            0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = stream::WithLog(stream::Loop::new(&status_packet));
        let mut state = Persistent::<crate::send::Ping, 1>::INIT;
        for _ in 0..offset {
            let _: u8 = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
        }
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input).unwrap() {
                parse::Status::Incomplete((updated, ())) => updated,
                parse::Status::Complete(actual) => {
                    assert_eq!(
                        actual, EXPECTED,
                        "Expected `{EXPECTED:02X?}` but got `{actual:02X?}`",
                    );
                    return;
                }
            }
        }
    }
}
