use {
    embassy_rp::{
        dma, gpio, interrupt,
        uart::{self, Async, Uart},
        Peripheral,
    },
    embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex},
};

pub struct Bus<'uart, 'tx_enable, HardwareUart: uart::Instance>(
    Mutex<CriticalSectionRawMutex, Devices<'uart, 'tx_enable, HardwareUart>>,
);

impl<'uart, 'tx_enable, HardwareUart: uart::Instance> Bus<'uart, 'tx_enable, HardwareUart> {
    #[inline(always)]
    pub fn new(
        baud_rate: u32,
        tx_enable_pin: impl Peripheral<P = impl gpio::Pin> + 'tx_enable,
        hardware_uart: impl Peripheral<P = HardwareUart> + 'uart,
        tx_pin: impl Peripheral<P = impl uart::TxPin<HardwareUart>> + 'uart,
        rx_pin: impl Peripheral<P = impl uart::RxPin<HardwareUart>> + 'uart,
        interrupts: impl interrupt::typelevel::Binding<
            HardwareUart::Interrupt,
            uart::InterruptHandler<HardwareUart>,
        >,
        tx_dma: impl Peripheral<P = impl dma::Channel> + 'uart,
        rx_dma: impl Peripheral<P = impl dma::Channel> + 'uart,
    ) -> Self {
        Self(Mutex::new(Devices::new(
            baud_rate,
            tx_enable_pin,
            hardware_uart,
            tx_pin,
            rx_pin,
            interrupts,
            tx_dma,
            rx_dma,
        )))
    }
}

struct Devices<'uart, 'tx_enable, HardwareUart: uart::Instance> {
    uart: Uart<'uart, HardwareUart, Async>,
    tx_enable: gpio::Output<'tx_enable>,
}

impl<'uart, 'tx_enable, HardwareUart: uart::Instance> Devices<'uart, 'tx_enable, HardwareUart> {
    #[inline]
    pub fn new(
        baud_rate: u32,
        tx_enable_pin: impl Peripheral<P = impl gpio::Pin> + 'tx_enable,
        hardware_uart: impl Peripheral<P = HardwareUart> + 'uart,
        tx_pin: impl Peripheral<P = impl uart::TxPin<HardwareUart>> + 'uart,
        rx_pin: impl Peripheral<P = impl uart::RxPin<HardwareUart>> + 'uart,
        interrupts: impl interrupt::typelevel::Binding<
            HardwareUart::Interrupt,
            uart::InterruptHandler<HardwareUart>,
        >,
        tx_dma: impl Peripheral<P = impl dma::Channel> + 'uart,
        rx_dma: impl Peripheral<P = impl dma::Channel> + 'uart,
    ) -> Self {
        let tx_enable = gpio::Output::new(tx_enable_pin, gpio::Level::Low);
        let uart = Uart::new(hardware_uart, tx_pin, rx_pin, interrupts, tx_dma, rx_dma, {
            let mut cfg = uart::Config::default();
            cfg.baudrate = baud_rate;
            cfg.data_bits = uart::DataBits::DataBits8;
            cfg.parity = uart::Parity::ParityNone;
            cfg.stop_bits = uart::StopBits::STOP1;
            cfg
        });
        Self { uart, tx_enable }
    }
}
