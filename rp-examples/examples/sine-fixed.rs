#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    defmt_rtt as _,
    dxl_driver::actuator,
    dxl_rp::Actuator,
    embassy_executor::Spawner,
    embassy_rp::{
        bind_interrupts,
        peripherals::{UART1, USB},
        uart, usb,
    },
    embassy_time::{Duration, Ticker, Timer},
    panic_probe as _,
};

bind_interrupts!(struct Irqs {
    UART1_IRQ => uart::InterruptHandler<UART1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const WAVE_PERIOD_MS: u64 = 5000;
const FRAME_PERIOD_MS: u64 = 20;

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
            Err(e) => {
                log::error!("Error spawning USB task");
                Timer::after(Duration::from_secs(1)).await;
                defmt::panic!("Error spawning USB task: {}", e);
            }
        };
    }

    let dxl_bus_mutex = dxl_rp::bus(
        115_200, p.PIN_7, p.UART1, p.PIN_8, p.PIN_9, Irqs, p.DMA_CH1, p.DMA_CH2,
    );

    let mut actuator = Actuator::persistent_init(
        &dxl_bus_mutex,
        actuator::Config {
            description: "Oscillating actuator",
            id: 1,
            position: actuator::Position::Specific {
                position: 0.5,
                tolerance: 0.001,
            },
        },
    )
    .await;
    log::info!("Scan succeeded");

    let mut counter: u64 = 0;
    let mut ticker = Ticker::every(Duration::from_millis(FRAME_PERIOD_MS));
    loop {
        let pos = 0.5_f32
            * (1_f32
                + libm::sinf(
                    const { 2_f32 * core::f32::consts::PI }
                        * ((counter as f32) / (WAVE_PERIOD_MS as f32)),
                ));
        log::info!("Sending to {}", pos);
        defmt::info!("Sending to {}", pos);
        match actuator.go_to(pos).await {
            Ok(()) => {}
            Err(e) => {
                log::error!("Error sending the actuator to {}", pos);
                defmt::error!("Error sending the actuator to {}: {}", pos, e);
            }
        }

        let () = ticker.next().await;
        counter += FRAME_PERIOD_MS;
        if counter >= WAVE_PERIOD_MS {
            counter -= WAVE_PERIOD_MS;
        }
    }
}
