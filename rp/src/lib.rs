#![no_std]
#![no_main]
#![feature(generic_const_exprs)]
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to construct Dynamixel packets on the stack"
)]

mod pull_high;
pub mod serial;

use {
    core::ops::DerefMut,
    dxl_driver::bus::Bus,
    dxl_packet::stream::Stream,
    embassy_futures::yield_now,
    embassy_rp::{
        dma, gpio, interrupt,
        uart::{self, Uart},
        Peripheral,
    },
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    embassy_time::{with_timeout, Duration, Instant, TimeoutError},
    pull_high::PullHigh,
};

const TIMEOUT_LOCK: Duration = Duration::from_millis(100);
const TIMEOUT_SEND: Duration = Duration::from_millis(100);
const TIMEOUT_RECV: Duration = Duration::from_millis(100);

#[inline]
#[expect(
    clippy::too_many_arguments,
    reason = "using a `struct` requires ridiculous generics"
)]
pub fn bus<'tx_en, 'uart, HardwareUart: uart::Instance>(
    baud_rate: u32,
    tx_enable_pin: impl Peripheral<P = impl gpio::Pin> + 'tx_en,
    uart: impl Peripheral<P = HardwareUart> + 'uart,
    tx: impl Peripheral<P = impl uart::TxPin<HardwareUart>> + 'uart,
    rx: impl Peripheral<P = impl uart::RxPin<HardwareUart>> + 'uart,
    irq: impl interrupt::typelevel::Binding<
        HardwareUart::Interrupt,
        uart::InterruptHandler<HardwareUart>,
    >,
    tx_dma: impl Peripheral<P = impl dma::Channel> + 'uart,
    rx_dma: impl Peripheral<P = impl dma::Channel> + 'uart,
) -> Mutex<Bus<Comm<'tx_en, 'uart, HardwareUart>>> {
    let comm = Comm::new(baud_rate, tx_enable_pin, uart, tx, rx, irq, tx_dma, rx_dma);
    let bus = Bus::new(comm);
    dxl_driver::mutex::Mutex::new(bus)
}

#[derive(defmt::Format)]
pub enum Error {
    Write(uart::Error),
    WriteTimeout(TimeoutError),
}

pub struct Comm<'tx_en, 'uart, HardwareUart: uart::Instance> {
    tx_enable: gpio::Output<'tx_en>,
    uart: Uart<'uart, HardwareUart, uart::Async>,
}

impl<'tx_en, 'uart, HardwareUart: uart::Instance> Comm<'tx_en, 'uart, HardwareUart> {
    #[inline]
    #[expect(
        clippy::too_many_arguments,
        reason = "using a `struct` requires ridiculous generics"
    )]
    pub fn new(
        baud_rate: u32,
        tx_enable_pin: impl Peripheral<P = impl gpio::Pin> + 'tx_en,
        uart: impl Peripheral<P = HardwareUart> + 'uart,
        tx: impl Peripheral<P = impl uart::TxPin<HardwareUart>> + 'uart,
        rx: impl Peripheral<P = impl uart::RxPin<HardwareUart>> + 'uart,
        irq: impl interrupt::typelevel::Binding<
            HardwareUart::Interrupt,
            uart::InterruptHandler<HardwareUart>,
        >,
        tx_dma: impl Peripheral<P = impl dma::Channel> + 'uart,
        rx_dma: impl Peripheral<P = impl dma::Channel> + 'uart,
    ) -> Self {
        Self {
            tx_enable: gpio::Output::new(tx_enable_pin, gpio::Level::Low),
            uart: Uart::new(uart, tx, rx, irq, tx_dma, rx_dma, {
                let mut cfg = uart::Config::default();
                cfg.baudrate = baud_rate;
                cfg.data_bits = uart::DataBits::DataBits8;
                cfg.stop_bits = uart::StopBits::STOP1;
                cfg.parity = uart::Parity::ParityNone;
                cfg
            }),
        }
    }
}

impl<'tx_en, 'uart, HardwareUart: uart::Instance> dxl_driver::comm::Comm
    for Comm<'tx_en, 'uart, HardwareUart>
{
    type SendError = Error;
    type RecvError = serial::RecvError;

    #[inline]
    async fn comm<'rx>(
        &'rx mut self,
        bytes: &[u8],
    ) -> Result<impl 'rx + Stream<Item = Result<u8, Self::RecvError>>, Self::SendError> {
        // Block incoming transmission ONLY WITHIN THIS SCOPE to allow outgoing transmission:
        let enable_tx = PullHigh::new(&mut self.tx_enable);
        let () = with_timeout(TIMEOUT_SEND, async {
            // Asynchronously ask hardware to transmit this buffer:
            let () = self.uart.write(bytes).await?;
            // Wait until it actually starts transmitting:
            while !self.uart.busy() {
                // let () = embassy_futures::yield_now().await;
            }
            // Then wait until it finishes:
            while self.uart.busy() {
                // let () = embassy_futures::yield_now().await;
            }
            Ok(())
        })
        .await
        .map_err(Error::WriteTimeout)?
        .map_err(Error::Write)?;
        // Then lower the `tx_enable` pin by dropping `_enable_tx`:
        // NOTE: I'm pretty sure this could be implicit, but this couldn't hurt.
        drop(enable_tx);
        Ok(serial::RxStream::new(&mut self.uart))
    }

    #[inline(always)]
    async fn yield_to_other_tasks() {
        let () = yield_now().await;
    }
}

pub struct Mutex<Item>(embassy_sync::mutex::Mutex<CriticalSectionRawMutex, Item>);

impl<Item> dxl_driver::mutex::Mutex for Mutex<Item> {
    type Item = Item;
    type Error = TimeoutError;

    #[inline(always)]
    fn new(item: Item) -> Self {
        Self(embassy_sync::mutex::Mutex::new(item))
    }

    #[inline(always)]
    async fn lock(&self) -> Result<impl DerefMut<Target = Self::Item>, Self::Error> {
        let start = Instant::now();
        // with_timeout(TIMEOUT_LOCK, self.0.lock()).await
        loop {
            if let Ok(ok) = self.0.try_lock() {
                return Ok(ok);
            }
            let () = yield_now().await;
            if start.elapsed() > TIMEOUT_LOCK {
                return Err(TimeoutError);
            }
        }
    }
}

pub type Actuator<'tx_en, 'uart, 'bus, const ID: u8, HardwareUart> = dxl_driver::actuator::Actuator<
    'bus,
    ID,
    Comm<'tx_en, 'uart, HardwareUart>,
    Mutex<Bus<Comm<'tx_en, 'uart, HardwareUart>>>,
>;
