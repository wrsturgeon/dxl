#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to precompute packets"
)]
#![feature(generic_const_exprs)]

use dxl::compiletime::{
    control_table,
    instruction::{self, Instruction},
    packet,
};

type Insn = instruction::Write<control_table::TorqueEnable>;
const ID: u8 = 1;

const PACKET: packet::send::WithCrc<Insn, ID> =
    packet::new::<Insn, ID>(<Insn as Instruction>::Send::new([1]));

fn main() {
    println!("{:02X?}", unsafe {
        core::slice::from_raw_parts(
            &PACKET as *const _ as *const u8,
            core::mem::size_of::<packet::send::WithCrc<Insn, ID>>(),
        )
    });
}
