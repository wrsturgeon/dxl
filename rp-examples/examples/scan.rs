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
        bind_interrupts,
        peripherals::{UART1, USB},
        uart, usb,
    },
    panic_probe as _,
};

bind_interrupts!(struct Irqs {
    UART1_IRQ => uart::InterruptHandler<UART1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const BAUD_RATES: &[u32] = &[
    9_600, 57_600, 115_200, 1_000_000, 2_000_000, 3_000_000, 4_000_000, 4_500_000,
];

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

    if BAUD_RATES.is_empty() {
        log::error!("No baud rates provided!");
        defmt::panic!("No baud rates provided!");
    }

    let dxl_bus_mutex = dxl_rp::bus(
        BAUD_RATES[0],
        p.PIN_7,
        p.UART1,
        p.PIN_8,
        p.PIN_9,
        Irqs,
        p.DMA_CH1,
        p.DMA_CH2,
    );
    let mut dxl_bus = match dxl_bus_mutex.lock().await {
        Err(e) => {
            log::error!("Couldn't acquire the Dynamixel bus mutex lock");
            defmt::panic!("Couldn't acquire the Dynamixel bus mutex lock: {}", e);
        }
        Ok(ok) => ok,
    };

    loop {
        for &baud in BAUD_RATES {
            log::info!("");
            defmt::info!("");
            log::info!("{} baud:", baud);
            defmt::info!("{} baud:", baud);

            let () = dxl_bus.set_baud(baud);

            for id in dxl_packet::MIN_ID..=dxl_packet::MAX_ID {
                log::debug!("Pinging {}...", id);
                defmt::debug!("Pinging {}...", id);
                'retry: loop {
                    match dxl_bus.ping(id).await {
                        Ok(recv::Ping {
                            model_number,
                            firmware_version,
                        }) => {
                            log::info!(
                                "    --> ID {} responded! Model number {}, firmware version {}",
                                id,
                                model_number,
                                firmware_version,
                            );
                            defmt::info!(
                                "    --> ID {} responded! Model number {}, firmware version {}",
                                id,
                                model_number,
                                firmware_version,
                            );
                            break 'retry;
                        }
                        Err(dxl_driver::bus::Error::Io(dxl_driver::IoError::Recv(
                            serial::RecvError::TimedOut(_),
                        ))) => {
                            log::debug!("    --> timed out");
                            defmt::debug!("    --> timed out");
                            break 'retry;
                        }
                        Err(e) => {
                            log::info!("    --> ID {} responded! ERROR", id);
                            defmt::info!("    --> ID {} responded! ERROR: {}", id, e);
                            break 'retry;
                        }
                    }
                }
            }
        }
    }

    // log::info!("Finished. Halting.");
    // defmt::info!("Finished. Halting.");
}
