#[repr(transparent)]
pub(crate) struct C8<const N: u8>(u8);

impl<const N: u8> C8<N> {
    #[inline(always)]
    pub(crate) const fn new() -> Self {
        Self(N)
    }
}

impl<const N: u8> defmt::Format for C8<N> {
    #[inline(always)]
    fn format(&self, f: defmt::Formatter) {
        <u8 as defmt::Format>::format(&self.0, f)
    }
}

#[repr(transparent)]
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
impl<const N: u16> defmt::Format for C16<N> {
    #[inline(always)]
    fn format(&self, f: defmt::Formatter) {
        <u16 as defmt::Format>::format(&u16::from_le_bytes(self.little_endian), f)
    }
}
