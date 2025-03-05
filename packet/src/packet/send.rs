use crate::{
    constants::{C16, C8},
    crc::Crc,
    instruction::Instruction,
};

#[repr(C, packed)]
pub(super) struct WithoutCrc<Insn: Instruction, const ID: u8>
where
    [(); { core::mem::size_of::<Insn::Send>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    pub(super) header: (C8<0xFF>, C8<0xFF>, C8<0xFD>),
    pub(super) reserved: C8<0x00>,
    pub(super) id: C8<ID>,
    pub(super) length: C16<{ core::mem::size_of::<Insn::Send>() as u16 + 3 }>,
    pub(super) instruction: C8<{ Insn::BYTE }>,
    pub(super) parameters: Insn::Send,
}

impl<Insn: Instruction, const ID: u8> WithoutCrc<Insn, ID>
where
    [(); { core::mem::size_of::<Insn::Send>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    #[inline(always)]
    pub(super) const fn new(parameters: Insn::Send) -> Self {
        Self {
            header: (C8::new(), C8::new(), C8::new()),
            reserved: C8::new(),
            id: C8::new(),
            length: C16::new(),
            instruction: C8::new(),
            parameters,
        }
    }

    #[inline(always)]
    pub(super) const fn crc_init() -> Crc {
        let mut crc = Crc::new();
        crc.push(0xFF);
        crc.push(0xFF);
        crc.push(0xFD);
        crc.push(0x00);
        crc.push(ID);
        {
            let [lo, hi] = const { (core::mem::size_of::<Insn::Send>() as u16 + 3).to_le_bytes() };
            crc.push(lo);
            crc.push(hi);
        }
        crc.push(Insn::BYTE);
        crc
    }
}

#[repr(C, packed)]
pub struct WithCrc<Insn: Instruction, const ID: u8>
where
    [(); { core::mem::size_of::<Insn::Send>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    pub(super) without_crc: WithoutCrc<Insn, ID>,
    pub(super) crc: [u8; 2],
}

impl<Insn: Instruction, const ID: u8> WithCrc<Insn, ID>
where
    [(); { core::mem::size_of::<Insn::Send>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    #[inline]
    pub const fn as_buffer(&self) -> &[u8] {
        let ptr = self as *const Self as *const u8;
        let size = const { core::mem::size_of::<Self>() };
        unsafe { core::slice::from_raw_parts(ptr, size) }
    }
}
