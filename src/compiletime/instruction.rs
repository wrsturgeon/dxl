use crate::{compiletime::control_table, constants::C16};

// #[const_trait]
pub trait Instruction {
    const BYTE: u8;
    const SEND_BYTES: u16;
    const RECV_BYTES: u16;

    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize];
}

#[repr(C, packed)]
pub struct Ping;
impl Instruction for Ping {
    const BYTE: u8 = 0x01;
    const SEND_BYTES: u16 = 0;
    const RECV_BYTES: u16 = 3;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

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
impl<Address: control_table::Item> Instruction for Read<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
{
    const BYTE: u8 = 0x02;
    const SEND_BYTES: u16 = 2;
    const RECV_BYTES: u16 = Address::BYTES;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
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
impl<Address: control_table::Item> Instruction for Write<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    const BYTE: u8 = 0x03;
    const SEND_BYTES: u16 = 2 + Address::BYTES;
    const RECV_BYTES: u16 = 0;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
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
impl<Address: control_table::Item> Instruction for RegWrite<Address>
where
    [(); { Address::ADDRESS as u16 } as usize]:,
    [(); Address::BYTES as usize]:,
{
    const BYTE: u8 = 0x04;
    const SEND_BYTES: u16 = 2 + Address::BYTES;
    const RECV_BYTES: u16 = 0;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

#[repr(C, packed)]
pub struct Action;
impl Instruction for Action {
    const BYTE: u8 = 0x05;
    const SEND_BYTES: u16 = 0;
    const RECV_BYTES: u16 = 0;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

#[repr(C, packed)]
pub struct FactoryReset;
impl Instruction for FactoryReset {
    const BYTE: u8 = 0x06;
    const SEND_BYTES: u16 = 0;
    const RECV_BYTES: u16 = 0;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

#[repr(C, packed)]
pub struct Reboot;
impl Instruction for Reboot {
    const BYTE: u8 = 0x08;
    const SEND_BYTES: u16 = 0;
    const RECV_BYTES: u16 = 0;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

/*
#[repr(C, packed)]
pub struct Clear(TODO);
impl /* const */ Instruction for Clear {
    const BYTE: u8 = 0x10;
    const SEND_BYTES: u16 = 5;
    const RECV_BYTES: u16 = 0;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

#[repr(C, packed)]
pub struct Backup(TODO);
impl /* const */ Instruction for Backup {
    const BYTE: u8 = 0x20;
    const SEND_BYTES: u16 = 5;
    const RECV_BYTES: u16 = 0;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

#[repr(C, packed)]
pub struct SyncRead;
impl /* const */ Instruction for SyncRead {
    const BYTE: u8 = 0x82;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

#[repr(C, packed)]
pub struct SyncWrite;
impl /* const */ Instruction for SyncWrite {
    const BYTE: u8 = 0x83;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

#[repr(C, packed)]
pub struct FastSyncRead;
impl /* const */ Instruction for FastSyncRead {
    const BYTE: u8 = 0x8A;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

#[repr(C, packed)]
pub struct BulkRead;
impl /* const */ Instruction for BulkRead {
    const BYTE: u8 = 0x92;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

#[repr(C, packed)]
pub struct BulkWrite;
impl /* const */ Instruction for BulkWrite {
    const BYTE: u8 = 0x93;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}

#[repr(C, packed)]
pub struct FastBulkRead;
impl /* const */ Instruction for FastBulkRead {
    const BYTE: u8 = 0x9A;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;

    // #[inline(always)]
    // fn into_bytes(self) -> [u8; Self::SEND_BYTES as usize] { [] }
}
*/
