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
