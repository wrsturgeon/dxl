#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    defmt_rtt as _,
    dxl_driver::mutex::Mutex as _,
    dxl_packet::recv,
    dxl_rp::serial,
    embassy_executor::Spawner,
    embassy_rp::{bind_interrupts, peripherals::UART0, uart},
    panic_probe as _,
};

bind_interrupts!(struct Irqs {
    UART0_IRQ => uart::InterruptHandler<UART0>;
});

const BAUD_RATES: &[u32] = &[
    9_600, 57_600, 1_000_000, 2_000_000, 3_000_000, 4_000_000, 4_500_000,
];

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    if BAUD_RATES.len() > 0 {
        let dxl_bus = dxl_rp::bus(
            BAUD_RATES[0],
            p.PIN_13,
            p.UART0,
            p.PIN_16,
            p.PIN_17,
            Irqs,
            p.DMA_CH1,
            p.DMA_CH2,
        );

        'baud: for &baud in BAUD_RATES {
            defmt::info!("");
            defmt::info!("{} baud:", baud);

            match dxl_bus.lock().await {
                Err(e) => {
                    defmt::error!("Couldn't set baud to {}: {}", baud, e);
                    continue 'baud;
                }
                Ok(mut bus) => {
                    let () = bus.set_baud(baud);
                }
            }

            for id in dxl_packet::MIN_ID..=dxl_packet::MAX_ID {
                defmt::debug!("Pinging {}...", id);
                'retry: loop {
                    if let Ok(mut bus) = dxl_bus.lock().await {
                        match bus.ping(id).await {
                            Ok(recv::Ping {
                                model_number,
                                firmware_version,
                            }) => {
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
                                defmt::debug!("    --> timed out");
                                break 'retry;
                            }
                            Err(e) => {
                                defmt::info!("    --> ID {} responded! ERROR: {}", id, e,);
                                break 'retry;
                            }
                        }
                    }
                }
            }
        }
    }

    defmt::info!("Finished. Halting.");
}
