#![cfg_attr(not(test), no_std)]
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to construct Dynamixel packets on the stack"
)]
#![feature(generic_const_exprs)]

pub mod constants;
pub mod control_table;
pub mod crc;
pub mod packet;
pub mod parse;
pub mod recv;
pub mod send;
pub mod stream;

pub trait Instruction: Sized + defmt::Format {
    const BYTE: u8;

    type Recv: recv::Receive;
    // type ParseState: parse::State<u8, Output = Self::Recv>;
    // type Parser: parse::MaybeParse<u8, Self::ParseState>;
}
