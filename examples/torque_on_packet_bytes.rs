#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to precompute packets"
)]
#![feature(generic_const_exprs)]

use dxl::compiletime::{control_table, instruction, packet::send::WithCrc};

type Packet = WithCrc<instruction::Write<control_table::TorqueEnable>, 1>;
const PACKET: Packet = Packet::precompute(instruction::Write::new([1]));

fn main() {
    println!("{:02X?}", unsafe {
        core::slice::from_raw_parts(
            &PACKET as *const _ as *const u8,
            core::mem::size_of::<Packet>(),
        )
    });
}
