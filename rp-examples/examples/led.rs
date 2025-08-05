#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    defmt_rtt as _,
    dxl_driver::bus::Bus,
    dxl_rp::{Actuator, Comm, Mutex},
    embassy_executor::Spawner,
    embassy_rp::{
        bind_interrupts,
        peripherals::{UART1, USB},
        uart, usb,
    },
    embassy_time::{Duration, Timer},
    panic_probe as _,
};

bind_interrupts!(struct Irqs {
    UART1_IRQ => uart::InterruptHandler<UART1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const BAUD: u32 = 1_000_000;

async fn persistent_actuator_init<'tx_en, 'uart, 'bus>(
    id: u8,
    description: &'static str,
    dxl_bus: &'bus Mutex<Bus<Comm<'tx_en, 'uart, UART1>>>,
) -> Actuator<'tx_en, 'uart, 'bus, UART1> {
    defmt::debug!(
        "Running `persistent_actuator_init` for \"{}\"...",
        description
    );
    loop {
        defmt::debug!("Calling `init_at_position` for \"{}\"...", description);
        match Actuator::init_at_position(dxl_bus, id, description, 0.5, 0.001).await {
            Ok(ok) => {
                defmt::debug!("`init_at_position` succeeded for \"{}\"", description);
                return ok;
            }
            Err(e) => defmt::error!(
                "Error initializing Dynamixel ID {} (\"{}\"): {}; retrying...",
                id,
                description,
                e
            ),
        }
        defmt::debug!(
            "Waiting a second before trying again with \"{}\"...",
            description
        );
        let () = Timer::after(Duration::from_secs(1)).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    {
        // USB background task:
        #[embassy_executor::task]
        pub async fn task(driver: usb::Driver<'static, USB>) {
            embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
        }
        let () = match spawner.spawn(task(usb::Driver::new(p.USB, Irqs))) {
            Ok(()) => defmt::info!("Spawned USB task"),
            Err(e) => defmt::panic!("Error spawning USB task: {}", e),
        };
    }
}
