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
    const GERUND: &'static str;

    type Recv: recv::Receive;
}

pub trait New {
    type Config;
    fn new(config: Self::Config) -> Self;
}

impl New for () {
    type Config = ();
    #[inline(always)]
    fn new((): ()) -> Self {}
}
