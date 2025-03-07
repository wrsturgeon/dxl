use core::fmt;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub(crate) struct C8<const N: u8>(u8);

impl<const N: u8> C8<N> {
    #[inline(always)]
    pub(crate) const fn new() -> Self {
        Self(N)
    }
}

impl<const N: u8> fmt::Debug for C8<N> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&N, f)
    }
}

impl<const N: u8> Default for C8<N> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub(crate) struct C16<const N: u16> {
    little_endian: [u8; 2],
}

impl<const N: u16> C16<N> {
    #[inline(always)]
    pub(crate) const fn new() -> Self {
        Self {
            little_endian: N.to_le_bytes(),
        }
    }
}

impl<const N: u16> fmt::Debug for C16<N> {
    #[inline(always)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&N, f)
    }
}

impl<const N: u16> Default for C16<N> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
