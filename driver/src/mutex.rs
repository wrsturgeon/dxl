use core::ops::DerefMut;

#[expect(async_fn_in_trait, reason = "fuck off")]
pub trait Mutex {
    type Item;
    async fn lock(&self) -> impl DerefMut<Target = Self::Item>;
}
