use {
    embassy_rp::uart::{self, Uart},
    embassy_time::{with_timeout, TimeoutError},
};

#[derive(defmt::Format)]
pub enum RecvError {
    TimedOut(TimeoutError),
    Uart(uart::Error),
}

/*
impl defmt::Format for RecvError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TimedOut(ref e) => {
                write!(f, "Timed out while waiting for a serial response: {e}")
            }
            Self::Uart(ref e) => write!(
                f,
                "UART error while trying to receive a serial response: {e:?}"
            ),
        }
    }
}
*/

pub(crate) struct RxStream<'lock, 'uart, HardwareUart: uart::Instance> {
    uart: &'lock mut Uart<'uart, HardwareUart, uart::Async>,
}

impl<'lock, 'uart, HardwareUart: uart::Instance> RxStream<'lock, 'uart, HardwareUart> {
    #[inline(always)]
    pub const fn new(uart: &'lock mut Uart<'uart, HardwareUart, uart::Async>) -> Self {
        Self { uart }
    }

    #[inline]
    pub async fn next_without_timeout(&mut self) -> Result<u8, uart::Error> {
        let mut byte: u8 = 0;
        let ptr = {
            let single: *mut u8 = &mut byte;
            let multiple: *mut [u8; 1] = single.cast();
            unsafe { &mut *multiple }
        };
        loop {
            match self.uart.read(ptr).await {
                Ok(()) => return Ok(byte),
                Err(uart::Error::Break) => defmt::warn!("UART break"),
                Err(e) => return Err(e),
            }
        }
    }
}

impl<'lock, 'uart, HardwareUart: uart::Instance> crate::Stream
    for RxStream<'lock, 'uart, HardwareUart>
{
    type Item = Result<u8, RecvError>;

    #[inline(always)]
    async fn next(&mut self) -> Self::Item {
        match with_timeout(crate::TIMEOUT_RECV, self.next_without_timeout()).await {
            Ok(Ok(ok)) => Ok(ok),
            Ok(Err(e)) => Err(RecvError::Uart(e)),
            Err(e) => Err(RecvError::TimedOut(e)),
        }
    }
}

/*
impl<'lock, 'uart, HardwareUart: uart::Instance> Drop for RxStream<'lock, 'uart, HardwareUart> {
    #[inline]
    fn drop(&mut self) {
        use core::{pin::pin, task};

        loop {
            match pin!(self.next_without_timeout())
                .poll(&mut task::Context::from_waker(task::Waker::noop()))
            {
                task::Poll::Ready(Ok(byte)) => {
                    defmt::error!("Extraneous byte: `x{=u8:X}`", byte);
                }
                task::Poll::Ready(Err(e)) => {
                    defmt::error!("Error while clearing an RX stream: {}", e)
                }
                task::Poll::Pending =>
                    return,
            }
        }
    }
}
*/
