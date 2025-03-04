use crate::stream::Stream;

#[expect(async_fn_in_trait, reason = "fuck off")]
pub trait Parse<Input>: Sized {
    type Output;
    type Error;

    async fn parse<S: Stream<Item = Input>>(s: &mut S) -> Result<Self::Output, Self::Error>;
}

impl Parse<u8> for u16 {
    type Output = Self;
    type Error = !;

    #[inline(always)]
    async fn parse<S: Stream<Item = u8>>(s: &mut S) -> Result<Self::Output, Self::Error> {
        let lo = s.next().await;
        let hi = s.next().await;
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
    async fn parse<S: Stream<Item = Input>>(s: &mut S) -> Result<Self::Output, Self::Error> {
        Ok((A::parse(s).await?, B::parse(s).await?, C::parse(s).await?))
    }
}
