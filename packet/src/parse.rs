use {
    // crate::stream::Stream,
    core::{fmt, marker::PhantomData, mem::MaybeUninit},
};

pub trait Parse<Input> {
    type State: State<Input, Output = Self>;
}

pub enum Status<Output, SideEffect> {
    Complete(Output),
    Incomplete(SideEffect),
}

pub trait State<Input>: Sized {
    type WithoutAnyInput;
    type Output;
    type SideEffect;
    type Error: fmt::Display;

    fn init() -> Status<Self::WithoutAnyInput, Self>;

    #[expect(clippy::type_complexity, reason = "grow up")]
    fn push(
        self,
        input: Input,
    ) -> Result<Status<Self::Output, (Self, Self::SideEffect)>, Self::Error>;
}

pub struct ParseUnit {
    _uninstantiable: !,
}

impl Parse<u8> for () {
    type State = ParseUnit;
}

impl State<u8> for ParseUnit {
    type WithoutAnyInput = ();
    type Output = ();
    type SideEffect = !;
    type Error = !;

    #[inline(always)]
    fn init() -> Status<Self::WithoutAnyInput, Self> {
        Status::Complete(())
    }

    #[inline(always)]
    fn push(self, _: u8) -> Result<Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
        unsafe { core::hint::unreachable_unchecked() }
    }
}

pub struct ParseU16 {
    first_byte: Option<u8>,
}

impl Parse<u8> for u16 {
    type State = ParseU16;
}

impl State<u8> for ParseU16 {
    type WithoutAnyInput = !;
    type Output = u16;
    type SideEffect = ();
    type Error = !;

    #[inline(always)]
    fn init() -> Status<Self::WithoutAnyInput, Self> {
        Status::Incomplete(Self { first_byte: None })
    }

    #[inline]
    fn push(
        self,
        input: u8,
    ) -> Result<Status<Self::Output, (Self, Self::SideEffect)>, Self::Error> {
        Ok(if let Some(first_byte) = self.first_byte {
            Status::Complete(u16::from_le_bytes([first_byte, input]))
        } else {
            Status::Incomplete((self, ()))
        })
    }
}

pub struct ItemResult<Input, S: State<Input>>(S, PhantomData<Input>);

pub enum ItemResultError<ItemError: fmt::Display, ParseError: fmt::Display> {
    Item(ItemError),
    Parsing(ParseError),
}

impl<ItemError: fmt::Display, ParseError: fmt::Display> fmt::Display
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

impl<Input, S: State<Input>, ItemError: fmt::Display> State<Result<Input, ItemError>>
    for ItemResult<Input, S>
{
    type WithoutAnyInput = S::WithoutAnyInput;
    type Output = S::Output;
    type SideEffect = S::SideEffect;
    type Error = ItemResultError<ItemError, S::Error>;

    #[inline(always)]
    fn init() -> Status<Self::WithoutAnyInput, Self> {
        let init = match S::init() {
            Status::Complete(complete) => return Status::Complete(complete),
            Status::Incomplete(s) => s,
        };
        Status::Incomplete(Self(init, PhantomData))
    }

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

impl<const N: usize> Parse<u8> for [u8; N] {
    type State = ByteArray<N>;
}

impl<const N: usize> State<u8> for ByteArray<N> {
    type WithoutAnyInput = !;
    type Output = [u8; N];
    type SideEffect = ();
    type Error = !;

    #[inline(always)]
    fn init() -> Status<Self::WithoutAnyInput, Self> {
        Status::Incomplete(Self {
            index: 0,
            buffer: [MaybeUninit::uninit(); N],
        })
    }

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

/*
#[inline]
async fn parse<Input, Output: Parse<Input, State: State<Input, WithoutAnyInput = !>>>(
    stream: &mut impl Stream<Item = Input>,
) -> Result<
    <<Output as Parse<Input>>::State as State<Input>>::Output,
    <<Output as Parse<Input>>::State as State<Input>>::Error,
> {
    let Status::Incomplete(mut state) = Output::State::init();
    loop {
        state = match state.push(stream.next().await)? {
            Status::Complete(output) => return Ok(output),
            Status::Incomplete((updated, _)) => updated,
        };
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
        let future = parse::<_, u16>(&mut s);
        let roundtrip: u16 = match test_util::trivial_future(pin!(future)) {
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
*/
