use {
    crate::{crc::Crc, parse},
    core::fmt,
};

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

pub enum ParseError<P: parse::State<u8>> {
    WrongFirstHeaderByte { expected: u8, actual: u8 },
    WrongSecondHeaderByte { expected: u8, actual: u8 },
    WrongThirdHeaderByte { expected: u8, actual: u8 },
    WrongReservedByte { expected: u8, actual: u8 },
    WrongId { expected: u8, actual: u8 },
    WrongLength { expected: u16, actual: u16 },
    WrongInstruction { expected: u8, actual: u8 },
    InvalidSoftwareError(InvalidSoftwareError),
    InstructionSpecific(P::Error),
}

impl<P: parse::State<u8>> fmt::Display for ParseError<P> {
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
            Self::InstructionSpecific(ref e) => fmt::Display::fmt(e, f),
            Self::InvalidSoftwareError(ref e) => fmt::Display::fmt(e, f),
        }
    }
}

pub struct WithHardwareErrorStatus<Output> {
    output: Output,
    crc: u16,
    hardware_error: bool,
}

pub enum WithoutCrc<P: parse::State<u8, SideEffect = ()>, const ID: u8> {
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
    Parameters {
        state: P,
        crc: Crc,
        hardware_error: bool,
    },
}

impl<P: parse::State<u8, SideEffect = ()>, const ID: u8> WithoutCrc<P, ID> {
    #[inline]
    const fn crc_init() -> Crc {
        let mut crc = Crc::new();
        crc.push(0xFF);
        crc.push(0xFF);
        crc.push(0xFD);
        crc.push(0x00);
        crc.push(ID);
        {
            let [lo, hi] = const { (core::mem::size_of::<P>() as u16 + 4).to_le_bytes() };
            crc.push(lo);
            crc.push(hi);
        }
        crc.push(0x55);
        crc
    }
}

impl<P: parse::State<u8, SideEffect = ()>, const ID: u8> parse::State<u8> for WithoutCrc<P, ID> {
    type WithoutAnyInput = !;
    type Output = Result<WithHardwareErrorStatus<P::Output>, SoftwareError>;
    type SideEffect = ();
    type Error = ParseError<P>;

    #[inline(always)]
    fn init() -> parse::Status<Self::WithoutAnyInput, Self> {
        parse::Status::Incomplete(Self::Header1)
    }

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
                    return Err(ParseError::$err {
                        expected: $byte,
                        actual: input,
                    });
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
                        return Err(ParseError::WrongInstruction {
                            expected: 0x55,
                            actual: input,
                        });
                    }
                }
                Self::Error { length } => {
                    if let Some(software_error) =
                        SoftwareError::check(input).map_err(ParseError::InvalidSoftwareError)?
                    {
                        return Ok(parse::Status::Complete(Err(software_error)));
                    }
                    // THEN, after we know it's not just short because of a mistunderstood packet,
                    // check to make sure that the length of the packet matches our expectation:
                    if length as usize != const { core::mem::size_of::<P::Output>() + 4 } {
                        return Err(ParseError::WrongLength {
                            actual: length,
                            expected: const { (core::mem::size_of::<P::Output>() + 4) as _ },
                        });
                    }
                    let mut crc = const { WithoutCrc::<P, ID>::crc_init() };
                    let () = crc.push(input);
                    Self::Parameters {
                        state: match P::init() {
                            parse::Status::Complete(_) => todo!(),
                            parse::Status::Incomplete(p) => p,
                        },
                        crc,
                        hardware_error: (input & 0x80) != 0,
                    }
                }
                Self::Parameters {
                    state,
                    mut crc,
                    hardware_error,
                } => {
                    let () = crc.push(input);
                    match state.push(input).map_err(ParseError::InstructionSpecific)? {
                        parse::Status::Complete(output) => {
                            return Ok(parse::Status::Complete(Ok(WithHardwareErrorStatus {
                                output,
                                crc: crc.collapse(),
                                hardware_error,
                            })))
                        }
                        parse::Status::Incomplete((state, ())) => Self::Parameters {
                            state,
                            crc,
                            hardware_error,
                        },
                    }
                }
            },
            (),
        )))
    }
}

pub enum Error<P: parse::State<u8>> {
    Parsing(ParseError<P>),
    Crc { expected: u16, actual: u16 },
    Software(SoftwareError),
    Hardware(P::Output),
}

impl<P: parse::State<u8>> fmt::Display for Error<P> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Parsing(ref e) => write!(f, "Parsing error: {e}"),
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

pub enum WithCrc<P: parse::State<u8, SideEffect = ()>, const ID: u8> {
    BeforeCrc {
        state: WithoutCrc<P, ID>,
    },
    FirstCrcByte {
        payload: WithHardwareErrorStatus<P::Output>,
    },
    SecondCrcByte {
        first_crc_byte: u8,
        payload: WithHardwareErrorStatus<P::Output>,
    },
}

impl<P: parse::State<u8, SideEffect = ()>, const ID: u8> parse::State<u8> for WithCrc<P, ID> {
    type WithoutAnyInput = !;
    type Output = P::Output;
    type SideEffect = ();
    type Error = Error<P>;

    #[inline(always)]
    fn init() -> parse::Status<Self::WithoutAnyInput, Self> {
        let parse::Status::Incomplete(state) = <WithoutCrc<P, ID> as parse::State<u8>>::init();
        parse::Status::Incomplete(Self::BeforeCrc { state })
    }

    #[inline(always)]
    fn push(
        self,
        input: u8,
    ) -> Result<parse::Status<Self::Output, (Self, Self::SideEffect)>, Error<P>> {
        Ok(parse::Status::Incomplete((
            match self {
                Self::BeforeCrc { state } => match state.push(input).map_err(Error::Parsing)? {
                    parse::Status::Complete(result) => Self::FirstCrcByte {
                        payload: result.map_err(Error::Software)?,
                    },
                    parse::Status::Incomplete((new_state, ())) => {
                        Self::BeforeCrc { state: new_state }
                    }
                },
                Self::FirstCrcByte { payload } => Self::SecondCrcByte {
                    first_crc_byte: input,
                    payload,
                },
                Self::SecondCrcByte {
                    first_crc_byte,
                    payload,
                } => {
                    return Ok(parse::Status::Complete({
                        let WithHardwareErrorStatus {
                            output,
                            crc: actual_crc,
                            hardware_error,
                        } = payload;
                        let expected_crc = u16::from_le_bytes([first_crc_byte, input]);
                        if actual_crc != expected_crc {
                            return Err(Error::Crc {
                                expected: expected_crc,
                                actual: actual_crc,
                            });
                        }
                        if hardware_error {
                            return Err(Error::Hardware(output));
                        }
                        output
                    }))
                }
            },
            (),
        )))
    }
}

/*
#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{instruction, stream, test_util},
        core::pin::pin,
        quickcheck::{Arbitrary, RandomSource, TestResult},
        quickcheck_macros::quickcheck,
        strum::VariantArray,
    };

    impl Arbitrary for SoftwareError {
        #[inline]
        fn arbitrary(g: &mut RandomSource) -> Self {
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
        let future = crate::packet::parse::<Ping, 0x01>(&mut s);
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
*/
