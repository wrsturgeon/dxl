mod recv;
pub mod send;

pub enum Error {
    Software(recv::SoftwareError),
    Hardware,
}

impl core::fmt::Display for Error {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Software(ref e) => core::fmt::Display::fmt(e, f),
            Self::Hardware => write!(
                f,
                "Hardware error reported (details require a separate request)"
            ),
        }
    }
}

#[inline]
pub const fn new<Insn: crate::instruction::Instruction, const ID: u8>(
    parameters: Insn::Send,
) -> send::WithCrc<Insn, ID>
where
    [(); { core::mem::size_of::<Insn::Send>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    let without_crc = send::WithoutCrc::new(parameters);
    let crc = {
        let mut crc_state = const { send::WithoutCrc::<Insn, ID>::crc_init() };
        crate::crc::recurse_over_bytes(&mut crc_state, {
            let ptr = {
                let init_ptr = &without_crc as *const _ as *const u8;
                let offset = const {
                    (core::mem::size_of::<send::WithoutCrc<Insn, ID>>()
                        - core::mem::size_of::<Insn::Send>()) as isize
                };
                unsafe { init_ptr.byte_offset(offset) }
            };
            unsafe {
                core::slice::from_raw_parts(ptr, const { core::mem::size_of::<Insn::Send>() })
            }
        });
        crc_state.collapse().to_le_bytes()
    };
    send::WithCrc { without_crc, crc }
}

#[inline(always)]
pub async fn parse<Insn: crate::instruction::Instruction, const ID: u8>(
    s: &mut impl crate::stream::Stream<Item = u8>,
) -> Result<<recv::WithCrc<Insn, ID> as crate::parse::Parse<u8>>::Output, Error>
where
    [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) & 0xFF) as u8 } as usize]:,
    [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) >> 8) as u8 } as usize]:,
{
    loop {
        return match <recv::WithCrc<Insn, ID> as crate::parse::Parse<u8>>::parse(s).await {
            Ok(ok) => Ok(ok),
            Err(recv::Error::Parsing(e)) => {
                log::error!("Parsing error: {e}; trying again...");
                continue;
            }
            Err(recv::Error::Software(e)) => Err(Error::Software(e)),
            Err(recv::Error::Hardware) => Err(Error::Hardware),
        };
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::{
            instruction,
            stream::{self, Stream},
            test_util,
        },
        core::pin::pin,
        quickcheck_macros::quickcheck,
    };

    #[quickcheck]
    fn parse_ping_in_media_res(displacement: u8) {
        const EXPECTED: instruction::recv::Ping = instruction::recv::Ping {
            model_number: 1030,
            firmware_version: 38,
        };
        let status_packet = [
            0xFF, 0xFF, 0xFD, 0x00, 0x01, 0x07, 0x00, 0x55, 0x00, 0x06, 0x04, 0x26, 0x65, 0x5D,
        ];
        let mut s = {
            let mut s = stream::Loop::new(&status_packet);
            for _ in 0..displacement {
                let _: u8 = test_util::trivial_future(pin!(s.next()));
            }
            stream::WithLog(s)
        };
        let future = parse::<instruction::Ping, 0x01>(&mut s);
        let actual = match test_util::trivial_future(pin!(future)) {
            Ok(ok) => ok,
            Err(e) => panic!("{e}"),
        };
        assert_eq!(
            actual, EXPECTED,
            "Expected `{EXPECTED:02X?}` but got `{actual:02X?}`",
        );
    }
}
