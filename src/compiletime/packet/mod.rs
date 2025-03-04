pub mod recv;
pub mod send;

#[inline]
pub const fn new<Insn: crate::compiletime::instruction::Instruction, const ID: u8>(
    parameters: Insn::Send,
) -> send::WithCrc<Insn, ID>
where
    [(); { core::mem::size_of::<Insn::Send>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    let without_crc = send::WithoutCrc::new(parameters);
    let crc = {
        let mut crc_state = const { send::WithoutCrc::<Insn, ID>::crc_init() };
        crate::crc::recurse_over_bytes(&mut crc_state, {
            let ptr = {
                let init_ptr = &without_crc as *const _ as *const u8;
                let offset = const {
                    (core::mem::size_of::<send::WithoutCrc<Insn, ID>>()
                        - core::mem::size_of::<Insn::Send>()) as isize
                };
                unsafe { init_ptr.byte_offset(offset) }
            };
            unsafe {
                core::slice::from_raw_parts(ptr, const { core::mem::size_of::<Insn::Send>() })
            }
        });
        crc_state.collapse().to_le_bytes()
    };
    send::WithCrc { without_crc, crc }
}

#[inline(always)]
pub async fn parse<
    Insn: crate::compiletime::instruction::Instruction,
    const ID: u8,
    F: Future<Output = u8>,
    Next: FnMut() -> F,
>(
    next: &mut Next,
) -> Result<
    <recv::WithCrc<Insn, ID> as crate::parse::Parse<u8>>::Output,
    <recv::WithCrc<Insn, ID> as crate::parse::Parse<u8>>::Error,
>
where
    [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) & 0xFF) as u8 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) >> 8) as u8 } as usize]:,
{
    <recv::WithCrc<Insn, ID> as crate::parse::Parse<u8>>::parse(next, &mut |_| {}).await
}
