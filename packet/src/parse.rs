use {crate::stream::Stream, core::fmt};

#[expect(async_fn_in_trait, reason = "fuck off")]
pub trait Parse<Input>: Sized {
    type Output;
    type Error: fmt::Display;

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
        E: fmt::Display,
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

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{stream, test_util},
        core::pin::pin,
        quickcheck::TestResult,
        quickcheck_macros::quickcheck,
    };

    #[quickcheck]
    fn parse_u16(i: u16) -> TestResult {
        let little_endian = i.to_le_bytes();
        let mut s = stream::WithLog(stream::Loop::new(&little_endian));
        let future = u16::parse(&mut s);
        let roundtrip = match test_util::trivial_future(pin!(future)) {
            Err(e) => return TestResult::error(format!("{e}")),
            Ok(ok) => ok,
        };
        if roundtrip == i {
            TestResult::passed()
        } else {
            TestResult::error(format!(
                "{i:02X?} -> {little_endian:02X?} -> {roundtrip:02X?} =/= {i:02X?}"
            ))
        }
    }
}
