#![expect(clippy::new_without_default, reason = "would be inconsistent")]

use {
    crate::{compiletime::control_table, parse::Parse},
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
    async fn parse<Callback: FnMut(u8), F: Future<Output = u8>, Next: FnMut() -> F>(
        next: &mut Next,
        callback: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
        let model_number_lo = next().await;
        callback(model_number_lo);
        let model_number_hi = next().await;
        callback(model_number_hi);
        let firmware_version = next().await;
        callback(firmware_version);
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
    async fn parse<Callback: FnMut(u8), F: Future<Output = u8>, Next: FnMut() -> F>(
        next: &mut Next,
        callback: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
        let mut buffer = [const { core::mem::MaybeUninit::uninit() }; Address::BYTES as usize];
        for uninit in &mut buffer {
            let byte = next().await;
            uninit.write(byte);
            callback(byte);
        }
        let ptr: *const [core::mem::MaybeUninit<u8>; Address::BYTES as usize] = &buffer;
        let cast: *const [u8; Address::BYTES as usize] = ptr.cast();
        Ok(Self {
            bytes: unsafe { cast.read() },
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
    async fn parse<Callback: FnMut(u8), F: Future<Output = u8>, Next: FnMut() -> F>(
        _: &mut Next,
        _: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
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
    async fn parse<Callback: FnMut(u8), F: Future<Output = u8>, Next: FnMut() -> F>(
        _: &mut Next,
        _: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
        Ok(Self::new())
    }
}

pub struct Action;
impl Parse<u8> for Action {
    type Output = Self;
    type Error = !;
    #[inline(always)]
    async fn parse<Callback: FnMut(u8), F: Future<Output = u8>, Next: FnMut() -> F>(
        _: &mut Next,
        _: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
        Ok(Self)
    }
}

pub struct FactoryReset;
impl Parse<u8> for FactoryReset {
    type Output = Self;
    type Error = !;
    #[inline(always)]
    async fn parse<Callback: FnMut(u8), F: Future<Output = u8>, Next: FnMut() -> F>(
        _: &mut Next,
        _: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
        Ok(Self)
    }
}

pub struct Reboot;
impl Parse<u8> for Reboot {
    type Output = Self;
    type Error = !;
    #[inline(always)]
    async fn parse<Callback: FnMut(u8), F: Future<Output = u8>, Next: FnMut() -> F>(
        _: &mut Next,
        _: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
        Ok(Self)
    }
}
