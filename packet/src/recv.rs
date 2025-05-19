pub trait Receive: Sized + defmt::Format {
    const BYTES: usize;
    type Parser: crate::parse::MaybeParse<u8, Self>;
}

impl Receive for () {
    const BYTES: usize = 0;
    type Parser = crate::parse::DontRun;
}

#[derive(defmt::Format)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Ping {
    pub model_number: u16,
    pub firmware_version: u8,
}

impl Receive for Ping {
    const BYTES: usize = 3;
    type Parser = crate::parse::Run<parse::Ping>;
}

#[derive(defmt::Format)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Read<const BYTES: usize> {
    pub bytes: [u8; BYTES],
}

impl<const BYTES: usize> Receive for Read<BYTES> {
    const BYTES: usize = BYTES;
    type Parser = crate::parse::Run<parse::Read<BYTES>>;
}

pub type Write = ();
pub type RegWrite = ();
pub type Action = ();
pub type FactoryReset = ();
pub type Reboot = ();

#[non_exhaustive]
#[cfg_attr(test, derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd,))]
pub struct HardwareErrorStatus {
    input_voltage: bool,
    overheat: bool,
    electric_shock: bool,
    overload: bool,
    unrecognized: bool,
}

impl defmt::Format for HardwareErrorStatus {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        let mut so_far = false;
        if self.input_voltage {
            so_far = true;
            defmt::write!(f, "input voltage out of safe range");
        }
        if self.overheat {
            if so_far {
                defmt::write!(f, " AND ");
            }
            so_far = true;
            defmt::write!(f, "overheating");
        }
        if self.electric_shock {
            if so_far {
                defmt::write!(f, " AND ");
            }
            so_far = true;
            defmt::write!(f, "electric shock");
        }
        if self.overload {
            if so_far {
                defmt::write!(f, " AND ");
            }
            so_far = true;
            defmt::write!(f, "overload");
        }
        if self.unrecognized {
            if so_far {
                defmt::write!(f, " AND ");
            }
            so_far = true;
            defmt::write!(
                f,
                "an unrecognized error (INTERNAL ERROR: update the protocol?)"
            );
        }
        if !so_far {
            defmt::write!(f, "[INTERNAL ERROR: no hardware errors]");
        }
    }
}

impl HardwareErrorStatus {
    #[inline]
    pub fn parse_byte(mut byte: u8) -> Self {
        let mut build = Self {
            input_voltage: false,
            overheat: false,
            electric_shock: false,
            overload: false,
            unrecognized: false,
        };
        if (byte & 0b1) != 0 {
            build.input_voltage = true;
            byte &= !0b1;
        }
        if (byte & 0b100) != 0 {
            build.overheat = true;
            byte &= !0b100;
        }
        if (byte & 0b10000) != 0 {
            build.electric_shock = true;
            byte &= !0b10000;
        }
        if (byte & 0b100000) != 0 {
            build.overload = true;
            byte &= !0b100000;
        }
        if byte != 0 {
            build.unrecognized = true;
        }
        build
    }
}

mod parse {
    use {
        crate::{New, parse},
        core::convert::Infallible,
    };

    pub struct Ping {
        model_number_lo: Option<u8>,
        model_number_hi: Option<u8>,
    }
    impl New for Ping {
        type Config = ();
        #[inline(always)]
        fn new((): ()) -> Self {
            Self {
                model_number_lo: None,
                model_number_hi: None,
            }
        }
    }
    impl parse::State<u8> for Ping {
        type Output = super::Ping;
        type SideEffect = ();
        type Error = Infallible;
        #[inline(always)]
        fn push(
            mut self,
            input: u8,
        ) -> Result<parse::Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
            Ok(if let Some(lo) = self.model_number_lo {
                if let Some(hi) = self.model_number_hi {
                    parse::Status::Complete(Self::Output {
                        model_number: u16::from_le_bytes([lo, hi]),
                        firmware_version: input,
                    })
                } else {
                    self.model_number_hi = Some(input);
                    parse::Status::Incomplete((self, ()))
                }
            } else {
                self.model_number_lo = Some(input);
                parse::Status::Incomplete((self, ()))
            })
        }
    }

    pub struct Read<const BYTES: usize>(parse::ByteArray<BYTES>);
    impl<const BYTES: usize> New for Read<BYTES> {
        type Config = <parse::ByteArray<BYTES> as New>::Config;
        #[inline(always)]
        fn new(config: Self::Config) -> Self {
            Self(parse::ByteArray::new(config))
        }
    }
    impl<const BYTES: usize> parse::State<u8> for Read<BYTES> {
        type Output = super::Read<BYTES>;
        type SideEffect = ();
        type Error = Infallible;
        #[inline(always)]
        fn push(
            self,
            input: u8,
        ) -> Result<parse::Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
            let Self(internal) = self;
            let Ok(status) = internal.push(input);
            Ok(match status {
                parse::Status::Incomplete((updated, ())) => {
                    parse::Status::Incomplete((Self(updated), ()))
                }
                parse::Status::Complete(bytes) => parse::Status::Complete(Self::Output { bytes }),
            })
        }
    }
}
