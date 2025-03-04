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

#[cfg(test)]
pub struct WithLog<S: Stream>(pub S);

#[cfg(test)]
impl<S: Stream<Item: core::fmt::Debug>> Stream for WithLog<S> {
    type Item = S::Item;

    #[inline]
    async fn next(&mut self) -> Self::Item {
        let item = self.0.next().await;
        println!("Stream log: {item:02X?}");
        item
    }
}

#[cfg(test)]
pub struct Loop<'slice, Item: Clone> {
    index: usize,
    slice: &'slice [Item],
    start: std::time::Instant,
}

#[cfg(test)]
impl<'slice, Item: Clone> Loop<'slice, Item> {
    #[inline]
    pub fn new(slice: &'slice [Item]) -> Self {
        Self {
            index: 0,
            slice,
            start: std::time::Instant::now(),
        }
    }
}

#[cfg(test)]
impl<Item: Clone> Stream for Loop<'_, Item> {
    type Item = Item;

    #[inline]
    async fn next(&mut self) -> Self::Item {
        loop {
            if self.start.elapsed() > core::time::Duration::from_millis(1) {
                panic!("`stream::Loop` timed out");
            }
            let Some(item) = self.slice.get(self.index) else {
                self.index = 0;
                continue;
            };
            self.index += 1;
            return item.clone();
        }
    }
}
