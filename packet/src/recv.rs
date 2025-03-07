use crate::{control_table, parse};

// const _ASSERT_ZST_UNIT: () = assert_eq!(core::mem::size_of::<()>(), 0);

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
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
    type WithoutAnyInput = !;
    type Output = Ping;
    type SideEffect = ();
    type Error = !;
    #[inline(always)]
    fn init() -> parse::Status<Self::WithoutAnyInput, Self> {
        parse::Status::Incomplete(Self {
            model_number_lo: None,
            model_number_hi: None,
        })
    }
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

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
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
    type WithoutAnyInput = !;
    type Output = Read<Address>;
    type SideEffect = ();
    type Error = !;
    #[inline(always)]
    fn init() -> parse::Status<Self::WithoutAnyInput, Self> {
        let parse::Status::Incomplete(init) = parse::ByteArray::init();
        parse::Status::Incomplete(Self(init))
    }
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
