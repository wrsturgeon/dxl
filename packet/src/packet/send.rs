use crate::{Instruction, crc::Crc};

#[repr(C, packed)]
pub(crate) struct WithoutCrc<Insn: Instruction> {
    header: (u8, u8, u8),
    reserved: u8,
    id: u8,
    length: [u8; 2],
    instruction: u8,
    parameters: Insn,
}

impl<Insn: Instruction> WithoutCrc<Insn> {
    #[inline]
    pub(crate) const fn new(id: u8, parameters: Insn) -> Self {
        Self {
            header: const { (0xFF, 0xFF, 0xFD) },
            reserved: 0x00,
            id,
            length: const { core::mem::size_of::<Insn>() as u16 + 3 }.to_le_bytes(),
            instruction: Insn::BYTE,
            parameters,
        }
    }

    #[inline]
    pub(crate) const fn crc_init() -> Crc {
        let mut crc = Crc::new();
        crc.push(0xFF);
        crc.push(0xFF);
        crc.push(0xFD);
        crc.push(0x00);
        crc
    }
}

#[repr(C, packed)]
pub struct WithCrc<Insn: Instruction> {
    pub(crate) without_crc: WithoutCrc<Insn>,
    pub(crate) crc: [u8; 2],
}

impl<Insn: Instruction> WithCrc<Insn> {
    #[inline]
    pub const fn as_buffer(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        let size = const { core::mem::size_of::<Self>() };
        unsafe { core::slice::from_raw_parts(ptr, size) }
    }
}
