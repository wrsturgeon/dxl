pub mod recv;
pub mod send;

#[inline]
pub const fn new<Insn: crate::Instruction, const ID: u8>(
    instruction: Insn,
) -> send::WithCrc<Insn, ID>
where
    [(); { core::mem::size_of::<Insn>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    let without_crc = send::WithoutCrc::new(instruction);
    let crc = {
        let mut crc_state = const { send::WithoutCrc::<Insn, ID>::crc_init() };
        let () = crc_state.recurse_over_bytes({
            let ptr = {
                let init_ptr = &without_crc as *const _ as *const u8;
                let offset = const {
                    (core::mem::size_of::<send::WithoutCrc<Insn, ID>>()
                        - core::mem::size_of::<Insn>()) as isize
                };
                unsafe { init_ptr.byte_offset(offset) }
            };
            unsafe { core::slice::from_raw_parts(ptr, const { core::mem::size_of::<Insn>() }) }
        });
        crc_state.collapse().to_le_bytes()
    };
    send::WithCrc { without_crc, crc }
}
