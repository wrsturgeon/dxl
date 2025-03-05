#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to precompute packets"
)]
#![feature(generic_const_exprs)]

use dxl_packet::{
    control_table,
    instruction::{self, Instruction},
    packet::{self, send},
};

type Insn = instruction::Write<control_table::TorqueEnable>;
const ID: u8 = 1;

const PACKET: send::WithCrc<Insn, ID> =
    packet::new::<Insn, ID>(<Insn as Instruction>::Send::new([1]));

fn main() {
    println!("{:02X?}", PACKET.as_buffer());
}
