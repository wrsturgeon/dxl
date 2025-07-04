use {
    core::fmt,
    embassy_futures::yield_now,
    embassy_rp::uart::{self, Uart},
    embassy_time::{TimeoutError, with_timeout},
};

#[derive(defmt::Format)]
pub enum RecvError {
    TimedOut(TimeoutError),
    Uart(uart::Error),
}

impl fmt::Display for RecvError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TimedOut(embassy_time::TimeoutError) => write!(f, "Serial read timed out"),
            Self::Uart(e) => match e {
                uart::Error::Break => write!(f, "UART break"),
                uart::Error::Overrun => write!(f, "UART overrun"),
                uart::Error::Parity => write!(f, "UART parity mismatch"),
                uart::Error::Framing => write!(f, "UART frame missing stop bit"),
                _ => write!(f, "UART error (unrecognized)"),
            },
        }
    }
}

pub(crate) struct RxStream<'lock, 'uart, HardwareUart: uart::Instance> {
    uart: &'lock mut Uart<'uart, HardwareUart, uart::Async>,
}

impl<'lock, 'uart, HardwareUart: uart::Instance> RxStream<'lock, 'uart, HardwareUart> {
    #[inline(always)]
    pub const fn new(uart: &'lock mut Uart<'uart, HardwareUart, uart::Async>) -> Self {
        Self { uart }
    }
}

impl<'lock, 'uart, HardwareUart: uart::Instance> crate::Stream
    for RxStream<'lock, 'uart, HardwareUart>
{
    type Item = Result<u8, RecvError>;

    #[inline(always)]
    async fn next(&mut self) -> Self::Item {
        let mut byte: u8 = 0;
        let ptr = {
            let single: *mut u8 = &mut byte;
            let multiple: *mut [u8; 1] = single.cast();
            unsafe { &mut *multiple }
        };
        loop {
            match with_timeout(crate::TIMEOUT_RECV, self.uart.read(ptr))
                .await
                .map_err(RecvError::TimedOut)?
            {
                Ok(()) => return Ok(byte),
                Err(uart::Error::Break) => defmt::warn!("UART break"),
                Err(e) => return Err(RecvError::Uart(e)),
            }
            let () = yield_now().await;
        }
    }
}
