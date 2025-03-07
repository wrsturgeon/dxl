#![cfg_attr(not(test), no_std)]
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to construct Dynamixel packets on the stack"
)]
#![feature(generic_const_exprs)]

pub mod actuator;
pub mod bus;
pub mod comm;
pub mod mutex;

pub enum Error<C: comm::Comm> {
    Send(<C as comm::Comm>::SendError),
    Recv(<C as comm::Comm>::RecvError),
    Packet(dxl_packet::packet::Error),
}
