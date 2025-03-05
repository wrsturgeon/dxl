#![cfg_attr(not(test), no_std)]
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to precompute packets"
)]
#![feature(generic_const_exprs, never_type)]

mod constants;
pub mod control_table;
mod crc;
pub mod instruction;
pub mod packet;
mod parse;
pub mod stream;

#[cfg(test)]
mod test_util;
