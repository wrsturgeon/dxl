#![cfg_attr(not(test), no_std)]
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to construct Dynamixel packets on the stack"
)]
#![feature(generic_const_exprs, never_type)]

pub mod constants;
pub mod control_table;
mod crc;
pub mod packet;
pub mod parse;
pub mod recv;
pub mod send;
pub mod stream;

#[cfg(test)]
mod test_util;

pub trait Instruction: Sized {
    const BYTE: u8;

    type Recv: core::fmt::Debug;
    type Parser: parse::State<u8, Output = Self::Recv>;
}
