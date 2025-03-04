use crate::crc::Crc;

#[expect(async_fn_in_trait, reason = "fuck off")]
pub trait Stream {
    type Item;
    async fn next(&mut self) -> Self::Item;
}

impl<S: Stream> Stream for &mut S {
    type Item = S::Item;

    #[inline(always)]
    async fn next(&mut self) -> Self::Item {
        S::next(self).await
    }
}

pub struct WithCrc<'crc, S: Stream<Item = u8>> {
    pub crc: &'crc mut Crc,
    pub internal: S,
}

impl<S: Stream<Item = u8>> Stream for WithCrc<'_, S> {
    type Item = u8;

    #[inline]
    async fn next(&mut self) -> Self::Item {
        let byte = self.internal.next().await;
        self.crc.push(byte);
        byte
    }
}
