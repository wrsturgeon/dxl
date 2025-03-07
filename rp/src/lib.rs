#![no_std]
#![no_main]
#![feature(generic_const_exprs, never_type)]
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to construct Dynamixel packets on the stack"
)]

mod pull_high;
mod rx_stream;

use {
    core::ops::DerefMut,
    dxl_packet::stream::Stream,
    embassy_futures::yield_now,
    embassy_rp::{
        gpio,
        uart::{self, Uart},
    },
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    embassy_time::{with_timeout, Duration, TimeoutError},
    pull_high::PullHigh,
};

const TIMEOUT_SEND: Duration = Duration::from_millis(1);
const TIMEOUT_RECV: Duration = Duration::from_millis(1);

pub enum Error {
    Write(uart::Error),
    WriteTimeout(TimeoutError),
}

pub struct Comm<'tx_en, 'uart, HardwareUart: uart::Instance> {
    tx_enable: gpio::Output<'tx_en>,
    uart: Uart<'uart, HardwareUart, uart::Async>,
}

impl<'tx_en, 'uart, HardwareUart: uart::Instance> dxl_driver::comm::Comm
    for Comm<'tx_en, 'uart, HardwareUart>
{
    type Error = Error;

    #[inline]
    async fn comm(
        &mut self,
        bytes: &[u8],
    ) -> Result<impl Stream<Item = u8> + 'static, <Self as dxl_driver::comm::Comm>::Error> {
        let () = with_timeout(TIMEOUT_SEND, async {
            // Block incoming transmission ONLY WITHIN THIS SCOPE to allow outgoing transmission:
            let _enable_tx = PullHigh::new(&mut self.tx_enable);
            // Asynchronously ask hardware to transmit this buffer:
            let () = self.uart.write(bytes).await?;
            // Wait until it actually starts transmitting:
            while !self.uart.busy() {
                let () = yield_now().await;
            }
            // Then wait until it finishes:
            while self.uart.busy() {
                let () = yield_now().await;
            }
            // Then lower the `tx_enable` pin by dropping `_enable_tx`:
            Ok(())
        })
        .await
        .map_err(Error::WriteTimeout)?
        .map_err(Error::Write)?;
        Ok(RxStream::new())
    }
}

pub struct Mutex<Item>(embassy_sync::mutex::Mutex<CriticalSectionRawMutex, Item>);

impl<Item> dxl_driver::mutex::Mutex for Mutex<Item> {
    type Item = Item;

    #[inline(always)]
    async fn lock(&self) -> impl DerefMut<Target = Self::Item> {
        self.0.lock().await
    }
}

pub type Actuator<'tx_en, 'uart, 'bus, const ID: u8, HardwareUart: uart::Instance> =
    dxl_driver::actuator::Actuator<
        'bus,
        ID,
        Comm<'tx_en, 'uart, HardwareUart>,
        Mutex<Comm<'tx_en, 'uart, HardwareUart>>,
    >;
