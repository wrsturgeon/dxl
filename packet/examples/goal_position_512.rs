#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to precompute packets"
)]
#![feature(generic_const_exprs)]

use dxl_packet::{control_table, packet, send};

type Insn = send::Write<control_table::GoalPosition>;
const ID: u8 = 1;

const PACKET: packet::send::WithCrc<Insn> =
    packet::new(ID, send::Write::new(512_u32.to_le_bytes()));

fn main() {
    println!("{:02X?}", PACKET.as_buffer());
}
