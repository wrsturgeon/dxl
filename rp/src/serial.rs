use {
    embassy_futures::yield_now,
    embassy_rp::uart::{self, Uart},
    embassy_time::{with_timeout, TimeoutError},
};

#[derive(defmt::Format)]
pub enum RecvError {
    TimedOut(TimeoutError),
    Uart(uart::Error),
}

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
            let () = yield_now().await;
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
