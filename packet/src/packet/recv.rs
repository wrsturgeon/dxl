use {
    crate::{Instruction, New, crc::Crc, parse, recv},
    core::fmt,
};

#[repr(u8)]
#[non_exhaustive]
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

impl defmt::Format for SoftwareError {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::ResultFail => defmt::write!(f, "Actuator could not process the packet"),
            Self::InstructionError => defmt::write!(
                f,
                "Either the actuator did not recognize the instruction byte or it received `Action` without `RegWrite`"
            ),
            Self::CrcError => defmt::write!(
                f,
                "Actuator disagrees about CRC calculation (likely a corrupted packet)"
            ),
            Self::DataRangeError => defmt::write!(
                f,
                "Data to be written is too long to fit in the specified range of memory"
            ),
            Self::DataLengthError => defmt::write!(
                f,
                "Data to be written is too short to fit in the specified range of memory"
            ),
            Self::DataLimitError => defmt::write!(f, "Data out of range"),
            Self::AccessError => defmt::write!(
                f,
                "Couldn't write (either tried to write to EEPROM with torque enabled, tried to write to read-only memory, or tried to read from write-only memory)"
            ),
        }
    }
}

impl fmt::Display for SoftwareError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::ResultFail => write!(f, "Actuator could not process the packet"),
            Self::InstructionError => write!(
                f,
                "Either the actuator did not recognize the instruction byte or it received `Action` without `RegWrite`"
            ),
            Self::CrcError => write!(
                f,
                "Actuator disagrees about CRC calculation (likely a corrupted packet)"
            ),
            Self::DataRangeError => write!(
                f,
                "Data to be written is too long to fit in the specified range of memory"
            ),
            Self::DataLengthError => write!(
                f,
                "Data to be written is too short to fit in the specified range of memory"
            ),
            Self::DataLimitError => write!(f, "Data out of range"),
            Self::AccessError => write!(
                f,
                "Couldn't write (either tried to write to EEPROM with torque enabled, tried to write to read-only memory, or tried to read from write-only memory)"
            ),
        }
    }
}

#[derive(defmt::Format)]
#[cfg_attr(test, derive(Debug, PartialEq))]
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

#[cfg_attr(test, derive(Debug, PartialEq))]
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

#[cfg_attr(test, derive(Debug, PartialEq))]
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
    InvalidId { id: u8 },
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
            Self::InvalidId { id } => defmt::write!(
                f,
                "Invalid ID: {} (must be between {} and {}, inclusive)",
                id,
                crate::MIN_ID,
                crate::MAX_ID,
            ),
            Self::WrongLength(ref e) => defmt::write!(f, "Wrong length: {}", e),
            Self::WrongInstruction(ref e) => defmt::write!(f, "Wrong instruction: {}", e),
            Self::InvalidSoftwareError(ref e) => defmt::write!(f, "Invalid software error: {}", e),
            Self::InstructionSpecific(ref e) => {
                defmt::write!(f, "Instruction-specific error: {}", e)
            }
        }
    }
}

#[derive(defmt::Format)]
pub struct WithHardwareErrorStatus<Output> {
    id: u8,
    output: Output,
    expected_crc: u16,
    hardware_error: bool,
}

pub enum WithoutCrc<Insn: Instruction> {
    Header1,
    Header2,
    Header3,
    Reserved,
    Id,
    LengthLo {
        id: u8,
        crc_state: Crc,
    },
    LengthHi {
        id: u8,
        crc_state: Crc,
        length_lo: u8,
    },
    Instruction {
        id: u8,
        crc_state: Crc,
        length: u16,
    },
    Error {
        id: u8,
        crc_state: Crc,
        length: u16,
    },
    SoftwareError {
        id: u8,
        crc_state: Crc,
        hardware_error: bool,
        software_error: SoftwareError,
        length: u16,
        count: u16,
    },
    Parameters {
        id: u8,
        state: <<<Insn as Instruction>::Recv as recv::Receive>::Parser as parse::MaybeParse<
            u8,
            <Insn as Instruction>::Recv,
        >>::Parser,
        crc_state: Crc,
        hardware_error: bool,
    },
}

impl<Insn: Instruction> WithoutCrc<Insn> {
    #[inline]
    const fn crc_init() -> Crc {
        let mut crc = Crc::new();
        let () = crc.push(0xFF);
        let () = crc.push(0xFF);
        let () = crc.push(0xFD);
        let () = crc.push(0x00);
        crc
    }
}

impl<Insn: Instruction> New for WithoutCrc<Insn> {
    type Config = ();

    #[inline(always)]
    fn new((): ()) -> Self {
        Self::Header1
    }
}

impl<Insn: Instruction> parse::State<u8> for WithoutCrc<Insn> {
    type Output = WithHardwareErrorStatus<Result<Insn::Recv, SoftwareError>>;
    type SideEffect = ();
    type Error = ParseError<
        <<<<Insn as Instruction>::Recv as recv::Receive>::Parser as parse::MaybeParse<
            u8,
            <Insn as Instruction>::Recv,
        >>::Parser as parse::State<u8>>::Error,
    >;

    #[inline]
    #[expect(clippy::too_many_lines, reason = "Lots of cases in a single match.")]
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
                Self::Id => {
                    if !(crate::MIN_ID..=crate::MAX_ID).contains(&input) {
                        return Err(ParseError::InvalidId { id: input });
                    }
                    let mut crc_state = const { WithoutCrc::<Insn>::crc_init() };
                    let () = crc_state.push(input);
                    Self::LengthLo {
                        id: input,
                        crc_state,
                    }
                }
                Self::LengthLo { id, mut crc_state } => {
                    let () = crc_state.push(
                        const { ((<Insn::Recv as crate::recv::Receive>::BYTES + 4) as u16) as u8 },
                    );
                    Self::LengthHi {
                        id,
                        crc_state,
                        length_lo: input,
                    }
                }
                Self::LengthHi {
                    id,
                    mut crc_state,
                    length_lo,
                } => {
                    let () =
                        crc_state.push(
                            const {
                                (((<Insn::Recv as crate::recv::Receive>::BYTES + 4) as u16) >> 8)
                                    as u8
                            },
                        );
                    Self::Instruction {
                        id,
                        crc_state,
                        length: u16::from_le_bytes([length_lo, input]),
                    }
                }
                Self::Instruction {
                    id,
                    mut crc_state,
                    length,
                } => {
                    if input == 0x55 {
                        let () = crc_state.push(0x55);
                        Self::Error {
                            id,
                            crc_state,
                            length,
                        }
                    } else {
                        return Err(ParseError::WrongInstruction(Mismatch8 {
                            expected: 0x55,
                            actual: input,
                        }));
                    }
                }
                Self::Error {
                    id,
                    mut crc_state,
                    length,
                } => {
                    let () = crc_state.push(input);
                    let hardware_error = (input & 0x80) != 0;
                    if let Some(software_error) =
                        SoftwareError::check(input).map_err(ParseError::InvalidSoftwareError)?
                    {
                        if length <= 4 {
                            return Ok(parse::Status::Complete(WithHardwareErrorStatus {
                                id,
                                output: Err(software_error),
                                expected_crc: crc_state.collapse(),
                                hardware_error,
                            }));
                        }
                        Self::SoftwareError {
                            id,
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
                        match <<<Insn as Instruction>::Recv as recv::Receive>::Parser as parse::MaybeParse<u8, _>>::init() {
                        parse::Status::Complete(output) => {
                            return Ok(parse::Status::Complete(WithHardwareErrorStatus {
                                id,
                                output: Ok(output),
                                expected_crc: crc_state.collapse(),
                                hardware_error,
                            }))
                        }
                        parse::Status::Incomplete(state) => Self::Parameters {
                            id,
                            state,
                            crc_state,
                            hardware_error,
                        },
                    }
                    }
                }
                Self::SoftwareError {
                    id,
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
                            id,
                            output: Err(software_error),
                            expected_crc: crc_state.collapse(),
                            hardware_error,
                        }));
                    } else {
                        Self::SoftwareError {
                            id,
                            crc_state,
                            hardware_error,
                            software_error,
                            length,
                            count,
                        }
                    }
                }
                Self::Parameters {
                    id,
                    state,
                    mut crc_state,
                    hardware_error,
                } => {
                    let () = crc_state.push(input);
                    match state.push(input).map_err(ParseError::InstructionSpecific)? {
                        parse::Status::Complete(output) => {
                            return Ok(parse::Status::Complete(WithHardwareErrorStatus {
                                id,
                                output: Ok(output),
                                expected_crc: crc_state.collapse(),
                                hardware_error,
                            }));
                        }
                        parse::Status::Incomplete((state, _)) => Self::Parameters {
                            id,
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

#[derive(defmt::Format)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct WithId<Output> {
    pub id: u8,
    pub output: Output,
}

pub enum WithCrc<Insn: Instruction> {
    BeforeCrc {
        state: WithoutCrc<Insn>,
    },
    FirstCrcByte {
        payload: WithHardwareErrorStatus<Result<Insn::Recv, SoftwareError>>,
    },
    SecondCrcByte {
        first_crc_byte: u8,
        payload: WithHardwareErrorStatus<Result<Insn::Recv, SoftwareError>>,
    },
}

impl<Insn: Instruction> New for WithCrc<Insn> {
    type Config = <WithoutCrc<Insn> as New>::Config;

    #[inline(always)]
    fn new(config: Self::Config) -> Self {
        Self::BeforeCrc {
            state: <WithoutCrc<Insn> as New>::new(config),
        }
    }
}

impl<Insn: Instruction> parse::State<u8> for WithCrc<Insn> {
    type Output = WithId<Insn::Recv>;
    type SideEffect = ();
    type Error = Error<
        Self::Output,
        <<<<Insn as Instruction>::Recv as recv::Receive>::Parser as parse::MaybeParse<
            u8,
            <Insn as Instruction>::Recv,
        >>::Parser as parse::State<u8>>::Error,
    >;

    #[inline(always)]
    fn push(
        self,
        input: u8,
    ) -> Result<parse::Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
        Ok(parse::Status::Incomplete((
            match self {
                Self::BeforeCrc { state } => match state.push(input).map_err(Error::Parsing)? {
                    parse::Status::Complete(payload) => Self::FirstCrcByte { payload },
                    parse::Status::Incomplete((new_state, _)) => {
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
                    let WithHardwareErrorStatus {
                        id,
                        output,
                        expected_crc,
                        hardware_error,
                    } = payload;
                    {
                        let actual_crc = u16::from_le_bytes([first_crc_byte, input]);
                        if actual_crc != expected_crc {
                            return Err(Error::Crc(Mismatch16 {
                                expected: expected_crc,
                                actual: actual_crc,
                            }));
                        }
                    }
                    let ok = output.map_err(Error::Software)?;
                    let with_id = WithId { id, output: ok };
                    return if hardware_error {
                        Err(Error::Hardware(with_id))
                    } else {
                        Ok(parse::Status::Complete(with_id))
                    };
                }
            },
            (),
        )))
    }
}

#[cfg_attr(test, derive(Debug))]
pub enum PersistentError<Output> {
    Software(SoftwareError),
    Hardware(Output),
}

impl<X> PersistentError<X> {
    #[inline]
    pub fn map<Y, F: FnOnce(X) -> Y>(self, f: F) -> PersistentError<Y> {
        match self {
            Self::Software(e) => PersistentError::Software(e),
            Self::Hardware(e) => PersistentError::Hardware(f(e)),
        }
    }
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

pub struct PersistentConfig {
    pub expected_id: u8,
}

pub struct Persistent<Insn: Instruction> {
    expected_id: u8,
    parser: WithCrc<Insn>,
}

impl<Insn: Instruction> New for Persistent<Insn> {
    type Config = PersistentConfig;

    #[inline(always)]
    fn new(PersistentConfig { expected_id }: Self::Config) -> Self {
        Self {
            expected_id,
            parser: <WithCrc<Insn> as New>::new(()),
        }
    }
}

impl<Insn: Instruction> parse::State<u8> for Persistent<Insn> {
    type Output = Insn::Recv;
    type SideEffect = ();
    type Error = PersistentError<Self::Output>;

    #[inline(always)]
    fn push(
        self,
        input: u8,
    ) -> Result<parse::Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
        let Self {
            expected_id,
            parser,
        } = self;
        match parser.push(input) {
            Ok(parse::Status::Complete(WithId {
                id: actual_id,
                output,
            })) => Ok(if actual_id == expected_id {
                parse::Status::Complete(output)
            } else {
                defmt::warn!(
                    "Wrong ID (expected {} but found {}); trying again...",
                    expected_id,
                    actual_id
                );
                parse::Status::Incomplete((Self::new(PersistentConfig { expected_id }), ()))
            }),
            Ok(parse::Status::Incomplete((incomplete, ()))) => Ok(parse::Status::Incomplete((
                Self {
                    expected_id,
                    parser: incomplete,
                },
                (),
            ))),
            Err(Error::Parsing(e)) => {
                defmt::warn!("Parsing error ({}); trying again...", e);
                Ok(parse::Status::Incomplete((
                    Self::new(PersistentConfig { expected_id }),
                    (),
                )))
            }
            Err(Error::Crc(e)) => {
                defmt::warn!("CRC error ({}); trying again...", e);
                Ok(parse::Status::Incomplete((
                    Self::new(PersistentConfig { expected_id }),
                    (),
                )))
            }
            Err(Error::Software(e)) => Err(PersistentError::Software(e)),
            Err(Error::Hardware(WithId {
                id: actual_id,
                output,
            })) => {
                if actual_id == expected_id {
                    Err(PersistentError::Hardware(output))
                } else {
                    defmt::warn!(
                        "Wrong ID (expected {} but found {}); trying again...",
                        expected_id,
                        actual_id
                    );
                    Ok(parse::Status::Incomplete((
                        Self::new(PersistentConfig { expected_id }),
                        (),
                    )))
                }
            }
        }
    }
}
