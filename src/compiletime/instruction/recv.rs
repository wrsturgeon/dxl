#![expect(clippy::new_without_default, reason = "would be inconsistent")]

use {
    crate::{compiletime::control_table, parse::Parse, stream::Stream},
    core::marker::PhantomData,
};

pub struct Ping {
    pub model_number: u16,
    pub firmware_version: u8,
}
impl Parse<u8> for Ping {
    type Output = Self;
    type Error = !;
    #[inline(always)]
    async fn parse<S: Stream<Item = u8>>(s: &mut S) -> Result<Self::Output, Self::Error> {
        let model_number_lo = s.next().await;
        let model_number_hi = s.next().await;
        let firmware_version = s.next().await;
        Ok(Self {
            model_number: u16::from_le_bytes([model_number_lo, model_number_hi]),
            firmware_version,
        })
    }
}

pub struct Read<Address: control_table::Item>
where
    [(); Address::BYTES as usize]:,
{
    pub bytes: [u8; Address::BYTES as usize],
}
impl<Address: control_table::Item> Parse<u8> for Read<Address>
where
    [(); Address::BYTES as usize]:,
{
    type Output = Self;
    type Error = !;
    #[inline(always)]
    async fn parse<S: Stream<Item = u8>>(s: &mut S) -> Result<Self::Output, Self::Error> {
        Ok(Self {
            bytes: [s.next().await; Address::BYTES as usize],
        })
    }
}

pub struct Write<Address: control_table::Item>(PhantomData<Address>);
impl<Address: control_table::Item> Write<Address> {
    #[inline(always)]
    pub fn new() -> Self {
        Self(PhantomData)
    }
}
impl<Address: control_table::Item> Parse<u8> for Write<Address> {
    type Output = Self;
    type Error = !;
    #[inline(always)]
    async fn parse<S: Stream<Item = u8>>(_: &mut S) -> Result<Self::Output, Self::Error> {
        Ok(Self::new())
    }
}

pub struct RegWrite<Address: control_table::Item>(PhantomData<Address>);
impl<Address: control_table::Item> RegWrite<Address> {
    #[inline(always)]
    pub fn new() -> Self {
        Self(PhantomData)
    }
}
impl<Address: control_table::Item> Parse<u8> for RegWrite<Address> {
    type Output = Self;
    type Error = !;
    #[inline(always)]
    async fn parse<S: Stream<Item = u8>>(_: &mut S) -> Result<Self::Output, Self::Error> {
        Ok(Self::new())
    }
}

pub struct Action;
impl Parse<u8> for Action {
    type Output = Self;
    type Error = !;
    #[inline(always)]
    async fn parse<S: Stream<Item = u8>>(_: &mut S) -> Result<Self::Output, Self::Error> {
        Ok(Self)
    }
}

pub struct FactoryReset;
impl Parse<u8> for FactoryReset {
    type Output = Self;
    type Error = !;
    #[inline(always)]
    async fn parse<S: Stream<Item = u8>>(_: &mut S) -> Result<Self::Output, Self::Error> {
        Ok(Self)
    }
}

pub struct Reboot;
impl Parse<u8> for Reboot {
    type Output = Self;
    type Error = !;
    #[inline(always)]
    async fn parse<S: Stream<Item = u8>>(_: &mut S) -> Result<Self::Output, Self::Error> {
        Ok(Self)
    }
}
