use crate::{
    constants::{C16, C8},
    crc::Crc,
    Instruction,
};

#[repr(C, packed)]
pub(crate) struct WithoutCrc<Insn: Instruction>
where
    [(); { core::mem::size_of::<Insn>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    header: (C8<0xFF>, C8<0xFF>, C8<0xFD>),
    reserved: C8<0x00>,
    id: u8,
    length: C16<{ core::mem::size_of::<Insn>() as u16 + 3 }>,
    instruction: C8<{ Insn::BYTE }>,
    parameters: Insn,
}

impl<Insn: Instruction> WithoutCrc<Insn>
where
    [(); { core::mem::size_of::<Insn>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    #[inline(always)]
    pub(crate) const fn new(id: u8, parameters: Insn) -> Self {
        Self {
            header: (C8::new(), C8::new(), C8::new()),
            reserved: C8::new(),
            id,
            length: C16::new(),
            instruction: C8::new(),
            parameters,
        }
    }

    #[inline(always)]
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
pub struct WithCrc<Insn: Instruction>
where
    [(); { core::mem::size_of::<Insn>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    pub(crate) without_crc: WithoutCrc<Insn>,
    pub(crate) crc: [u8; 2],
}

impl<Insn: Instruction> WithCrc<Insn>
where
    [(); { core::mem::size_of::<Insn>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    #[inline]
    pub const fn as_buffer(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        let size = const { core::mem::size_of::<Self>() };
        unsafe { core::slice::from_raw_parts(ptr, size) }
    }
}
