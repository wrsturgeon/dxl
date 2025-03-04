#[expect(async_fn_in_trait, reason = "fuck off")]
pub trait Parse<Input>: Sized {
    type Output;
    type Error;

    async fn parse<Callback: FnMut(u8), F: Future<Output = Input>, Next: FnMut() -> F>(
        next: &mut Next,
        callback: &mut Callback,
    ) -> Result<Self::Output, Self::Error>;
}

impl Parse<u8> for u16 {
    type Output = Self;
    type Error = !;

    #[inline(always)]
    async fn parse<Callback: FnMut(u8), F: Future<Output = u8>, Next: FnMut() -> F>(
        next: &mut Next,
        callback: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
        let lo = next().await;
        callback(lo);
        let hi = next().await;
        callback(hi);
        Ok(Self::from_le_bytes([lo, hi]))
    }
}

impl<
        Input,
        E,
        A: Parse<Input, Error = E>,
        B: Parse<Input, Error = E>,
        C: Parse<Input, Error = E>,
    > Parse<Input> for (A, B, C)
{
    type Output = (A::Output, B::Output, C::Output);
    type Error = E;

    #[inline]
    async fn parse<Callback: FnMut(u8), F: Future<Output = Input>, Next: FnMut() -> F>(
        next: &mut Next,
        callback: &mut Callback,
    ) -> Result<Self::Output, Self::Error> {
        Ok((
            A::parse(next, callback).await?,
            B::parse(next, callback).await?,
            C::parse(next, callback).await?,
        ))
    }
}
