use crate::{
    compiletime::instruction::Instruction,
    constants::{C16, C8},
    crc::{self, Crc},
};

#[repr(C, packed)]
pub struct WithoutCrc<Insn: Instruction, const ID: u8>
where
    [(); { Insn::SEND_BYTES + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    header: (C8<0xFF>, C8<0xFF>, C8<0xFD>),
    reserved: C8<0x00>,
    id: C8<ID>,
    length: C16<{ Insn::SEND_BYTES + 3 }>,
    instruction: C8<{ Insn::BYTE }>,
    parameters: Insn,
}

impl<Insn: Instruction, const ID: u8> WithoutCrc<Insn, ID>
where
    [(); { Insn::SEND_BYTES + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    #[inline(always)]
    pub const fn new(parameters: Insn) -> Self {
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
            let [lo, hi] = (Insn::SEND_BYTES + 3).to_le_bytes();
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
    [(); { Insn::SEND_BYTES + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    without_crc: WithoutCrc<Insn, ID>,
    crc: [u8; 2],
}

impl<Insn: Instruction, const ID: u8> WithCrc<Insn, ID>
where
    [(); { Insn::SEND_BYTES + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    #[inline]
    pub const fn precompute(parameters: Insn) -> Self {
        let without_crc = WithoutCrc::new(parameters);
        let mut crc_state = const { WithoutCrc::<Insn, ID>::crc_init() };
        crc::recurse_over_bytes(&mut crc_state, {
            let ptr = {
                let init_ptr = &without_crc as *const _ as *const u8;
                let offset = const {
                    (core::mem::size_of::<WithoutCrc<Insn, ID>>() - core::mem::size_of::<Insn>())
                        as isize
                };
                unsafe { init_ptr.byte_offset(offset) }
            };
            let size = const { core::mem::size_of::<Insn>() };
            unsafe { core::slice::from_raw_parts(ptr, size) }
        });
        let crc = crc_state.collapse().to_le_bytes();
        Self { without_crc, crc }
    }

    #[inline]
    pub fn at_runtime(parameters: Insn) -> Self {
        let without_crc = WithoutCrc::new(parameters);
        let mut crc = const { WithoutCrc::<Insn, ID>::crc_init() };
        let ptr: *const u8 = &without_crc as *const _ as *const u8;
        for i in (core::mem::size_of::<WithoutCrc<Insn, ID>>() - core::mem::size_of::<Insn>())
            ..(core::mem::size_of::<WithoutCrc<Insn, ID>>())
        {
            crc.push(unsafe { *ptr.byte_offset(i as isize) })
        }
        Self {
            without_crc,
            crc: crc.collapse().to_le_bytes(),
        }
    }
}
