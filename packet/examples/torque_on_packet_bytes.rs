#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to precompute packets"
)]
#![feature(const_trait_impl, generic_const_exprs)]

use dxl_packet::{control_table, packet, send};

type Insn = send::Write<control_table::TorqueEnable>;
const ID: u8 = 1;

const PACKET: packet::send::WithCrc<Insn> = packet::new(ID, send::Write::new([1]));

fn main() {
    println!("{:02X?}", PACKET.as_buffer());
}
