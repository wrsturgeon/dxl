pub mod recv;
pub mod send;

#[inline]
pub const fn new<Insn: crate::Instruction>(
    id: u8,
    instruction: Insn,
) -> send::WithCrc<Insn>
where
    [(); { core::mem::size_of::<Insn>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    let without_crc = send::WithoutCrc::new(id, instruction);
    let crc = {
        let mut crc_state = const { send::WithoutCrc::<Insn>::crc_init() };
        let () = crc_state.push(id);
        let () = crc_state.push(
            const { ((<Insn::Recv as crate::recv::Receive>::BYTES + 4) as u16) as u8 },
        );
        let () =
            crc_state.push(
                const {
                    (((<Insn::Recv as crate::recv::Receive>::BYTES + 4) as u16) >> 8)
                        as u8
                },
            );
        let () = crc_state.push(const { Insn::BYTE });
        let () = crc_state.recurse_over_bytes({
            let ptr = {
                let init_ptr = &without_crc as *const _ as *const u8;
                let offset = const {
                    (core::mem::size_of::<send::WithoutCrc<Insn>>()
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
