use {
    crate::{parse::Parse, stream::Stream},
    core::fmt,
};

#[repr(transparent)]
pub struct C8<const N: u8>(u8);

impl<const N: u8> C8<N> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self(N)
    }

    #[inline(always)]
    pub const fn get(&self) -> u8 {
        self.0
    }
}

impl<const N: u8> Default for C8<N> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: u8> Parse<u8> for C8<N> {
    type Output = ();
    type Error = WrongByte;

    #[inline(always)]
    async fn parse<S: Stream<Item = u8>>(s: &mut S) -> Result<Self::Output, Self::Error> {
        let actual = s.next().await;
        if actual == N {
            Ok(())
        } else {
            Err(WrongByte {
                actual,
                expected: N,
            })
        }
    }
}

#[repr(C, packed)]
pub struct C16<const N: u16> {
    little_endian: [u8; 2],
}

impl<const N: u16> C16<N> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            little_endian: N.to_le_bytes(),
        }
    }

    #[inline(always)]
    pub const fn get(&self) -> u16 {
        u16::from_le_bytes(self.little_endian)
    }
}

impl<const N: u16> Default for C16<N> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: u16> Parse<u8> for C16<N>
where
    [(); { (N & 0xFF) as u8 } as usize]:,
    [(); { (N >> 8) as u8 } as usize]:,
{
    type Output = ();
    type Error = WrongByte;

    #[inline(always)]
    async fn parse<S: Stream<Item = u8>>(s: &mut S) -> Result<Self::Output, Self::Error> {
        let () = <C8<{ (N & 0xFF) as u8 }> as Parse<u8>>::parse(s).await?;
        let () = <C8<{ (N >> 8) as u8 }> as Parse<u8>>::parse(s).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct WrongByte {
    pub expected: u8,
    pub actual: u8,
}

impl fmt::Display for WrongByte {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            ref expected,
            ref actual,
        } = *self;
        write!(f, "Expected `{expected:02X?}` but received `{actual:02X?}`")
    }
}
