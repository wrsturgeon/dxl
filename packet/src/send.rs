#![expect(
    clippy::new_without_default,
    reason = "extra binary space, inconsistent across instructions"
)]

use core::marker::PhantomData;

use crate::{Instruction, control_table, recv};

#[repr(C, packed)]
#[derive(defmt::Format)]
pub struct Ping;
impl Ping {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
impl Instruction for Ping {
    const BYTE: u8 = 0x01;
    const GERUND: &str = "Pinging";
    type Recv = recv::Ping;
}

#[repr(C, packed)]
#[derive(defmt::Format)]
pub struct Read<Address: control_table::Item>
where
    [(); Address::BYTES as usize]:,
{
    address: [u8; 2],
    length: [u8; 2],
    _phantom: PhantomData<Address>,
}
impl<Address: control_table::Item> Read<Address>
where
    [(); Address::BYTES as usize]:,
{
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            address: (Address::ADDRESS as u16).to_le_bytes(),
            length: Address::BYTES.to_le_bytes(),
            _phantom: PhantomData,
        }
    }
}
impl<Address: control_table::Item> Instruction for Read<Address>
where
    [(); Address::BYTES as usize]:,
{
    const BYTE: u8 = 0x02;
    const GERUND: &str = "Reading";
    type Recv = recv::Read<{ Address::BYTES as usize }>;
}

#[repr(C, packed)]
pub struct Write<Address: control_table::Item, const BYTES: usize> {
    address: [u8; 2],
    bytes: [u8; BYTES],
    _phantom: PhantomData<Address>,
}
impl<Address: control_table::Item, const BYTES: usize> Write<Address, BYTES> {
    #[inline]
    #[must_use]
    pub const fn new(bytes: [u8; BYTES]) -> Self {
        Self {
            address: (Address::ADDRESS as u16).to_le_bytes(),
            bytes,
            _phantom: PhantomData,
        }
    }
}
impl<Address: control_table::Item, const BYTES: usize> Instruction for Write<Address, BYTES> {
    const BYTE: u8 = 0x03;
    const GERUND: &str = "Writing";
    type Recv = ();
}
impl<Address: control_table::Item, const BYTES: usize> defmt::Format for Write<Address, BYTES> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Write {{ address: {}, bytes: [ ", Address::DESCRIPTION);
        let byte: *const u8 = (&raw const self.bytes).cast();
        for i in 0..Address::BYTES {
            defmt::write!(f, "x{=u8:X}, ", unsafe { byte.add(usize::from(i)).read() });
        }
        let () = defmt::write!(f, "] }}");
    }
}

#[repr(C, packed)]
pub struct RegWrite<Address: control_table::Item>
where
    [(); Address::BYTES as usize]:,
{
    address: [u8; 2],
    bytes: [u8; Address::BYTES as usize],
}
impl<Address: control_table::Item> RegWrite<Address>
where
    [(); Address::BYTES as usize]:,
{
    #[inline]
    #[must_use]
    pub const fn new(bytes: [u8; Address::BYTES as usize]) -> Self {
        Self {
            address: (Address::ADDRESS as u16).to_le_bytes(),
            bytes,
        }
    }
}
impl<Address: control_table::Item> Instruction for RegWrite<Address>
where
    [(); Address::BYTES as usize]:,
{
    const BYTE: u8 = 0x04;
    const GERUND: &str = "Register-writing";
    type Recv = ();
}
impl<Address: control_table::Item> defmt::Format for RegWrite<Address>
where
    [(); Address::BYTES as usize]:,
{
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(
            f,
            "RegWrite {{ address: {}, bytes: [ ",
            Address::DESCRIPTION
        );
        let byte: *const u8 = (&raw const self.bytes).cast();
        for i in 0..Address::BYTES {
            defmt::write!(f, "x{=u8:X}, ", unsafe { byte.add(usize::from(i)).read() });
        }
        let () = defmt::write!(f, "] }}");
    }
}

#[repr(C, packed)]
#[derive(defmt::Format)]
pub struct Action;
impl Action {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
impl Instruction for Action {
    const BYTE: u8 = 0x05;
    const GERUND: &str = "Sending action";
    type Recv = ();
}

#[repr(C, packed)]
#[derive(defmt::Format)]
pub struct FactoryReset;
impl FactoryReset {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
impl Instruction for FactoryReset {
    const BYTE: u8 = 0x06;
    const GERUND: &str = "Factory-resetting";
    type Recv = ();
}

#[repr(C, packed)]
#[derive(defmt::Format)]
pub struct Reboot;
impl Reboot {
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}
impl Instruction for Reboot {
    const BYTE: u8 = 0x08;
    const GERUND: &str = "Rebooting";
    type Recv = ();
}
