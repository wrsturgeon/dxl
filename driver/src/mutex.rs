use core::ops::DerefMut;

#[expect(async_fn_in_trait, reason = "fuck off")]
pub trait Mutex {
    type Item;
    type Error: defmt::Format;
    fn new(item: Self::Item) -> Self;
    async fn lock(&self) -> Result<impl DerefMut<Target = Self::Item>, Self::Error>;

    #[inline]
    async fn lock_persistent(&self) -> impl DerefMut<Target = Self::Item> {
        loop {
            match self.lock().await {
                Ok(ok) => return ok,
                Err(e) => defmt::error!("Error acquiring a mutex lock: {}", e),
            }
        }
    }
}
