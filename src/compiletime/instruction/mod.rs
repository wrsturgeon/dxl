pub mod recv;
pub mod send;

use {
    crate::{compiletime::control_table, parse::Parse},
    core::{fmt, marker::PhantomData},
};

pub trait Instruction {
    const BYTE: u8;
    // const SEND_BYTES: u16;
    // const RECV_BYTES: u16;

    type Send;
    type Recv: fmt::Debug + Parse<u8>;

    // TODO: SEE IF WE CAN USE SIZES OF THESE TYPES INSTEAD OF CONSTANTS
}

pub struct Ping;
impl Instruction for Ping {
    const BYTE: u8 = 0x01;
    type Send = send::Ping;
    type Recv = recv::Ping;
}

pub struct Read<Address: control_table::Item>(PhantomData<Address>)
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); { Address::BYTES } as usize]:;
impl<Address: control_table::Item> Instruction for Read<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); { Address::BYTES } as usize]:,
{
    const BYTE: u8 = 0x02;
    type Send = send::Read<Address>;
    type Recv = recv::Read<Address>;
}

pub struct Write<Address: control_table::Item>(PhantomData<Address>)
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); { Address::BYTES } as usize]:;
impl<Address: control_table::Item> Instruction for Write<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); { Address::BYTES } as usize]:,
{
    const BYTE: u8 = 0x03;
    type Send = send::Write<Address>;
    type Recv = recv::Write<Address>;
}

pub struct RegWrite<Address: control_table::Item>(PhantomData<Address>)
where
    [(); { Address::BYTES } as usize]:,
    [(); { Address::ADDRESS as u16 } as usize]:;
impl<Address: control_table::Item> Instruction for RegWrite<Address>
where
    [(); { Address::BYTES } as usize]:,
    [(); { Address::ADDRESS as u16 } as usize]:,
{
    const BYTE: u8 = 0x04;
    type Send = send::RegWrite<Address>;
    type Recv = recv::RegWrite<Address>;
}

pub struct Action;
impl Instruction for Action {
    const BYTE: u8 = 0x05;
    type Send = send::Action;
    type Recv = recv::Action;
}

pub struct FactoryReset;
impl Instruction for FactoryReset {
    const BYTE: u8 = 0x06;
    type Send = send::FactoryReset;
    type Recv = recv::FactoryReset;
}

pub struct Reboot;
impl Instruction for Reboot {
    const BYTE: u8 = 0x08;
    type Send = send::Reboot;
    type Recv = recv::Reboot;
}
