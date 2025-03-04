#![expect(clippy::new_without_default, reason = "would be inconsistent")]

use crate::{compiletime::control_table, constants::C16};

pub struct Ping;

#[repr(C, packed)]
pub struct Read<Address: control_table::Item>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
{
    address: C16<{ Address::ADDRESS as u16 }>,
}
impl<Address: control_table::Item> Read<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
{
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            address: C16::new(),
        }
    }
}

#[repr(C, packed)]
pub struct Write<Address: control_table::Item>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    address: C16<{ Address::ADDRESS as u16 }>,
    value: [u8; Address::BYTES as usize],
}
impl<Address: control_table::Item> Write<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    #[inline(always)]
    pub const fn new(value: [u8; Address::BYTES as usize]) -> Self {
        Self {
            address: C16::new(),
            value,
        }
    }
}

#[repr(C, packed)]
pub struct RegWrite<Address: control_table::Item>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    address: C16<{ Address::ADDRESS as u16 }>,
    value: [u8; Address::BYTES as usize],
}
impl<Address: control_table::Item> RegWrite<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    #[inline(always)]
    pub const fn new(value: [u8; Address::BYTES as usize]) -> Self {
        Self {
            address: C16::new(),
            value,
        }
    }
}

pub struct Action;

pub struct FactoryReset;

pub struct Reboot;
