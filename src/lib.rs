#![cfg_attr(not(test), no_std)]
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to precompute packets"
)]
#![feature(generic_const_exprs, never_type)]

pub mod compiletime;
pub mod constants;
pub mod crc;
pub mod parse;
pub mod runtime;
pub mod stream;
