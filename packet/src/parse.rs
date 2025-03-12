use core::{convert::Infallible, marker::PhantomData, mem::MaybeUninit};

pub trait MaybeParse<Input, Output>: Sized {
    type Parser: State<Input, Output = Output>;
    const INIT: Status<Output, Self::Parser>;
}

pub struct DontRun {
    _uninstantiable: Infallible,
}
impl<Input> MaybeParse<Input, ()> for DontRun {
    type Parser = Infallible;
    const INIT: Status<(), Self::Parser> = Status::Complete(());
}

pub struct Run<S> {
    _uninstantiable: Infallible,
    _phantom: PhantomData<S>,
}
impl<Input, S: State<Input>> MaybeParse<Input, S::Output> for Run<S> {
    type Parser = S;
    const INIT: Status<S::Output, Self::Parser> = Status::Incomplete(S::INIT);
}

pub enum Status<Output, SideEffect> {
    Complete(Output),
    Incomplete(SideEffect),
}

pub trait State<Input>: Sized {
    type Output;
    type SideEffect;
    type Error: defmt::Format;

    const INIT: Self;

    #[expect(clippy::type_complexity, reason = "grow up")]
    fn push(
        self,
        input: Input,
    ) -> Result<Status<Self::Output, (Self, Self::SideEffect)>, Self::Error>;
}

impl<Input> State<Input> for Infallible {
    type Output = ();
    type SideEffect = Infallible;
    type Error = Infallible;
    const INIT: Self = unsafe { core::hint::unreachable_unchecked() };
    #[inline(always)]
    fn push(self, _: Input) -> Result<Status<(), (Infallible, Infallible)>, Infallible> {
        unsafe { core::hint::unreachable_unchecked() }
    }
}

pub struct ParseU16 {
    first_byte: Option<u8>,
}

impl State<u8> for ParseU16 {
    type Output = u16;
    type SideEffect = ();
    type Error = Infallible;

    const INIT: Self = Self { first_byte: None };

    #[inline]
    fn push(
        mut self,
        input: u8,
    ) -> Result<Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
        Ok(if let Some(first_byte) = self.first_byte {
            Status::Complete(u16::from_le_bytes([first_byte, input]))
        } else {
            self.first_byte = Some(input);
            Status::Incomplete((self, ()))
        })
    }
}

pub struct ItemResult<Input, S: State<Input>>(S, PhantomData<Input>);

#[derive(defmt::Format)]
pub enum ItemResultError<ItemError: defmt::Format, ParseError: defmt::Format> {
    Item(ItemError),
    Parsing(ParseError),
}

/*
impl<ItemError: defmt::Format, ParseError: defmt::Format> defmt::Format
    for ItemResultError<ItemError, ParseError>
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Item(ref e) => write!(f, "Error reported in input stream: {e}"),
            Self::Parsing(ref e) => write!(f, "Error while parsing: {e}"),
        }
    }
}
*/

impl<Input, S: State<Input>, ItemError: defmt::Format> State<Result<Input, ItemError>>
    for ItemResult<Input, S>
{
    type Output = S::Output;
    type SideEffect = S::SideEffect;
    type Error = ItemResultError<ItemError, S::Error>;

    const INIT: Self = Self(S::INIT, PhantomData);

    #[inline]
    fn push(
        self,
        input: Result<Input, ItemError>,
    ) -> Result<Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
        let Self(state, PhantomData) = self;
        Ok(
            match state
                .push(input.map_err(ItemResultError::Item)?)
                .map_err(ItemResultError::Parsing)?
            {
                Status::Complete(output) => Status::Complete(output),
                Status::Incomplete((recurse, side_effect)) => {
                    Status::Incomplete((Self(recurse, PhantomData), side_effect))
                }
            },
        )
    }
}

pub struct ByteArray<const N: usize> {
    index: usize,
    buffer: [MaybeUninit<u8>; N],
}

impl<const N: usize> State<u8> for ByteArray<N> {
    type Output = [u8; N];
    type SideEffect = ();
    type Error = Infallible;

    const INIT: Self = Self {
        index: 0,
        buffer: [MaybeUninit::uninit(); N],
    };

    #[inline]
    fn push(
        mut self,
        input: u8,
    ) -> Result<Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
        let Some(uninit) = self.buffer.get_mut(self.index) else {
            return Ok(Status::Complete({
                let ptr: *const _ = &self.buffer;
                let cast: *const Self::Output = ptr.cast();
                unsafe { cast.read() }
            }));
        };
        uninit.write(input);
        self.index += 1;
        Ok(Status::Incomplete((self, ())))
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{
            parse::State,
            stream::{self, Stream},
        },
        core::{pin::pin, task},
        quickcheck::TestResult,
        quickcheck_macros::quickcheck,
    };

    #[quickcheck]
    fn parse_u16(i: u16) -> TestResult {
        let little_endian = i.to_le_bytes();
        let mut s = stream::WithLog(stream::Loop::new(&little_endian));
        let mut state = ParseU16::INIT;
        loop {
            let input = match pin!(s.next())
                .poll(&mut const { task::Context::from_waker(task::Waker::noop()) })
            {
                task::Poll::Ready(ready) => ready,
                task::Poll::Pending => panic!("Future pending"),
            };
            state = match state.push(input).unwrap() {
                Status::Incomplete((updated, ())) => updated,
                Status::Complete(roundtrip) => {
                    return if roundtrip == i {
                        TestResult::passed()
                    } else {
                        TestResult::error(format!(
                            "{i:02X?} -> {little_endian:02X?} -> {roundtrip:02X?} =/= {i:02X?}"
                        ))
                    }
                }
            };
        }
    }
}
