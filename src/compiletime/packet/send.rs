use crate::{
    compiletime::instruction::Instruction,
    constants::{C16, C8},
    crc::Crc,
};

#[repr(C, packed)]
pub struct WithoutCrc<Insn: Instruction, const ID: u8>
where
    // [(); { Insn::SEND_BYTES + 3 } as usize]:,
    [(); { core::mem::size_of::<Insn::Send>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    pub header: (C8<0xFF>, C8<0xFF>, C8<0xFD>),
    pub reserved: C8<0x00>,
    pub id: C8<ID>,
    pub length: C16<{ core::mem::size_of::<Insn::Send>() as u16 + 3 }>,
    pub instruction: C8<{ Insn::BYTE }>,
    pub parameters: Insn::Send,
}

impl<Insn: Instruction, const ID: u8> WithoutCrc<Insn, ID>
where
    // [(); { Insn::SEND_BYTES + 3 } as usize]:,
    [(); { core::mem::size_of::<Insn::Send>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    #[inline(always)]
    pub const fn new(parameters: Insn::Send) -> Self {
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
    pub const fn crc_init() -> Crc {
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
    // [(); { Insn::SEND_BYTES + 3 } as usize]:,
    [(); { core::mem::size_of::<Insn::Send>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    pub without_crc: WithoutCrc<Insn, ID>,
    pub crc: [u8; 2],
}
