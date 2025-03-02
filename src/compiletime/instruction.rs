use {crate::compiletime::control_table, core::marker::PhantomData};

pub trait Instruction {
    const BYTE: u8;
    const SEND_BYTES: u16;
    const RECV_BYTES: u16;
}

pub struct Ping;
impl Instruction for Ping {
    const BYTE: u8 = 0x01;
    const SEND_BYTES: u16 = 0;
    const RECV_BYTES: u16 = 3;
}

pub struct Read<Address: control_table::Item>(PhantomData<Address>);
impl<Address: control_table::Item> Instruction for Read<Address> {
    const BYTE: u8 = 0x02;
    const SEND_BYTES: u16 = 2;
    const RECV_BYTES: u16 = Address::BYTES;
}

pub struct Write<Address: control_table::Item>(PhantomData<Address>);
impl<Address: control_table::Item> Instruction for Write<Address> {
    const BYTE: u8 = 0x03;
    const SEND_BYTES: u16 = Address::BYTES;
    const RECV_BYTES: u16 = 0;
}

pub struct RegWrite<Address: control_table::Item>(PhantomData<Address>);
impl<Address: control_table::Item> Instruction for RegWrite<Address> {
    const BYTE: u8 = 0x04;
    const SEND_BYTES: u16 = Address::BYTES;
    const RECV_BYTES: u16 = 0;
}

pub struct Action;
impl Instruction for Action {
    const BYTE: u8 = 0x05;
    const SEND_BYTES: u16 = 0;
    const RECV_BYTES: u16 = 0;
}

pub struct FactoryReset;
impl Instruction for FactoryReset {
    const BYTE: u8 = 0x06;
    const SEND_BYTES: u16 = 0;
    const RECV_BYTES: u16 = 0;
}

pub struct Reboot;
impl Instruction for Reboot {
    const BYTE: u8 = 0x08;
    const SEND_BYTES: u16 = 0;
    const RECV_BYTES: u16 = 0;
}

pub struct Clear;
impl Instruction for Clear {
    const BYTE: u8 = 0x10;
    const SEND_BYTES: u16 = 5;
    const RECV_BYTES: u16 = 0;
}

pub struct Backup;
impl Instruction for Backup {
    const BYTE: u8 = 0x20;
    const SEND_BYTES: u16 = 5;
    const RECV_BYTES: u16 = 0;
}

/*
pub struct SyncRead;
impl Instruction for SyncRead {
    const BYTE: u8 = 0x82;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;
}

pub struct SyncWrite;
impl Instruction for SyncWrite {
    const BYTE: u8 = 0x83;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;
}

pub struct FastSyncRead;
impl Instruction for FastSyncRead {
    const BYTE: u8 = 0x8A;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;
}

pub struct BulkRead;
impl Instruction for BulkRead {
    const BYTE: u8 = 0x92;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;
}

pub struct BulkWrite;
impl Instruction for BulkWrite {
    const BYTE: u8 = 0x93;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;
}

pub struct FastBulkRead;
impl Instruction for FastBulkRead {
    const BYTE: u8 = 0x9A;
    const SEND_BYTES: u16 = ;
    const RECV_BYTES: u16 = ;
}
*/
