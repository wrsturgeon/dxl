use crate::{constants::C16, control_table, parse, recv, Instruction};

// const _ASSERT_ZST_UNIT: () = assert_eq!(core::mem::size_of::<()>(), 0);
// const _ASSERT_ZST_PING: () = assert_eq!(core::mem::size_of::<Ping>(), 0);
// const _ASSERT_ZST_ACTION: () = assert_eq!(core::mem::size_of::<Action>(), 0);
// const _ASSERT_ZST_FACTORY_RESET: () = assert_eq!(core::mem::size_of::<FactoryReset>(), 0);
// const _ASSERT_ZST_REBOOT: () = assert_eq!(core::mem::size_of::<Reboot>(), 0);

#[repr(C, packed)]
#[derive(Debug, Default)]
pub struct Ping;
impl Ping {
    #[inline(always)]
    pub const fn new() -> Self {
        Self
    }
}
impl Instruction for Ping {
    const BYTE: u8 = 0x01;
    type Recv = recv::Ping;
    type Parser = recv::ParsePing;
}

#[repr(C, packed)]
#[derive(Debug, Default)]
pub struct Read<Address: control_table::Item>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    address: C16<{ Address::ADDRESS as u16 }>,
}
impl<Address: control_table::Item> Read<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            address: C16::new(),
        }
    }
}
impl<Address: control_table::Item> Instruction for Read<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    const BYTE: u8 = 0x02;
    type Recv = recv::Read<Address>;
    type Parser = recv::ParseRead<Address>;
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct Write<Address: control_table::Item>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    address: C16<{ Address::ADDRESS as u16 }>,
    bytes: [u8; Address::BYTES as usize],
}
impl<Address: control_table::Item> Write<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    #[inline(always)]
    pub const fn new(bytes: [u8; Address::BYTES as usize]) -> Self {
        Self {
            address: C16::new(),
            bytes,
        }
    }
}
impl<Address: control_table::Item> Instruction for Write<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    const BYTE: u8 = 0x03;
    type Recv = ();
    type Parser = parse::ParseUnit;
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct RegWrite<Address: control_table::Item>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    address: C16<{ Address::ADDRESS as u16 }>,
    bytes: [u8; Address::BYTES as usize],
}
impl<Address: control_table::Item> RegWrite<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    #[inline(always)]
    pub const fn new(bytes: [u8; Address::BYTES as usize]) -> Self {
        Self {
            address: C16::new(),
            bytes,
        }
    }
}
impl<Address: control_table::Item> Instruction for RegWrite<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    const BYTE: u8 = 0x04;
    type Recv = ();
    type Parser = parse::ParseUnit;
}

#[repr(C, packed)]
#[derive(Debug, Default)]
pub struct Action;
impl Action {
    #[inline(always)]
    pub const fn new() -> Self {
        Self
    }
}
impl Instruction for Action {
    const BYTE: u8 = 0x05;
    type Recv = ();
    type Parser = parse::ParseUnit;
}

#[repr(C, packed)]
#[derive(Debug, Default)]
pub struct FactoryReset;
impl FactoryReset {
    #[inline(always)]
    pub const fn new() -> Self {
        Self
    }
}
impl Instruction for FactoryReset {
    const BYTE: u8 = 0x06;
    type Recv = ();
    type Parser = parse::ParseUnit;
}

#[repr(C, packed)]
#[derive(Debug, Default)]
pub struct Reboot;
impl Reboot {
    #[inline(always)]
    pub const fn new() -> Self {
        Self
    }
}
impl Instruction for Reboot {
    const BYTE: u8 = 0x08;
    type Recv = ();
    type Parser = parse::ParseUnit;
}
