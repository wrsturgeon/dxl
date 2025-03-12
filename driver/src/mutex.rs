use core::ops::DerefMut;

#[expect(async_fn_in_trait, reason = "fuck off")]
pub trait Mutex {
    type Item;
    type Error: defmt::Format;
    fn new(item: Self::Item) -> Self;
    async fn lock(&self) -> Result<impl DerefMut<Target = Self::Item>, Self::Error>;
}
