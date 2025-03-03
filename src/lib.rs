#![cfg_attr(not(test), no_std)]
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to precompute packets"
)]
#![feature(
    // const_trait_impl,
    generic_const_exprs,
)]

pub mod compiletime;
pub mod constants;
pub mod crc;
pub mod runtime;
