use crate::control_table;

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
pub struct Read<Address: control_table::Item>
where
    [(); Address::BYTES as usize]:,
{
    pub bytes: [u8; Address::BYTES as usize],
}

impl<Address: control_table::Item> Receive for Read<Address>
where
    [(); Address::BYTES as usize]:,
{
    const BYTES: usize = Address::BYTES as usize;
    type Parser = crate::parse::Run<parse::Read<Address>>;
}

pub type Write = ();
pub type RegWrite = ();
pub type Action = ();
pub type FactoryReset = ();
pub type Reboot = ();

mod parse {
    use {
        crate::{control_table, parse},
        core::convert::Infallible,
    };

    pub struct Ping {
        model_number_lo: Option<u8>,
        model_number_hi: Option<u8>,
    }
    impl parse::State<u8> for Ping {
        type Output = super::Ping;
        type SideEffect = ();
        type Error = Infallible;
        const INIT: Self = Self {
            model_number_lo: None,
            model_number_hi: None,
        };
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

    pub struct Read<Address: control_table::Item>(parse::ByteArray<{ Address::BYTES as usize }>)
    where
        [(); Address::BYTES as usize]:;
    impl<Address: control_table::Item> parse::State<u8> for Read<Address>
    where
        [(); Address::BYTES as usize]:,
    {
        type Output = super::Read<Address>;
        type SideEffect = ();
        type Error = Infallible;
        const INIT: Self = Self(parse::ByteArray::INIT);
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

/*
pub(crate) mod packed {
use {core::convert::Infallible, crate::{control_table, parse}};

#[repr(C, packed)]
pub struct Ping {
    pub model_number: u16,
    pub firmware_version: u8,
}
pub struct ParsePing {
    model_number_lo: Option<u8>,
    model_number_hi: Option<u8>,
}
impl parse::State<u8> for ParsePing {
    type Output = Ping;
    type SideEffect = ();
    type Error = Infallible;
    const INIT: Self = Self {
        model_number_lo: None,
        model_number_hi: None,
    };
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

#[repr(C, packed)]
pub struct Read<Address: control_table::Item>
where
    [(); Address::BYTES as usize]:,
{
    pub bytes: [u8; Address::BYTES as usize],
}
pub struct ParseRead<Address: control_table::Item>(parse::ByteArray<{ Address::BYTES as usize }>)
where
    [(); Address::BYTES as usize]:;
impl<Address: control_table::Item> parse::State<u8> for ParseRead<Address>
where
    [(); Address::BYTES as usize]:,
{
    type Output = Read<Address>;
    type SideEffect = ();
    type Error = Infallible;
    const INIT: Self = Self(parse::ByteArray::INIT);
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

pub type Write = ();
pub type RegWrite = ();
pub type Action = ();
pub type FactoryReset = ();
pub type Reboot = ();

}
*/
