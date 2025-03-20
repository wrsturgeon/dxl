#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    defmt_rtt as _,
    dxl_driver::mutex::Mutex as _,
    dxl_packet::recv,
    dxl_rp::serial,
    embassy_executor::Spawner,
    embassy_rp::{
        uart,
        bind_interrupts,
        peripherals::UART0,
    },
    panic_probe as _,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
    // PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    UART0_IRQ => uart::InterruptHandler<UART0>;
    // USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const BAUD: u32 = 1_000_000;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    static NET_RESOURCES: StaticCell<embassy_net::StackResources<3>> = StaticCell::new();

    let p = embassy_rp::init(Default::default());

    let dxl_bus = dxl_rp::bus(
        BAUD, p.PIN_13, p.UART0, p.PIN_16, p.PIN_17, Irqs, p.DMA_CH1, p.DMA_CH2,
    );

    for id in 0..=252 {
        defmt::debug!("Pinging {}...", id);
        'retry: loop {
            if let Ok(mut bus) = dxl_bus.lock().await {
                match bus.ping(id).await {
                    Ok(recv::Ping { model_number, firmware_version }) => {
                        defmt::info!("    --> ID {} responded! Model number {}, firmware version {}", id, model_number, firmware_version);
                        break 'retry;
                    }
                    Err(dxl_driver::bus::Error::Io(dxl_driver::IoError::Recv(serial::RecvError::TimedOut(_)))) => {
                        defmt::debug!("    --> timed out");
                        break 'retry;
                    }
                    Err(e) => {
                        defmt::debug!("    --> unexpected error ({}); retrying...", e);
                    }
                }
            }
        }
    }

    defmt::info!("Finished. Halting.");
}
