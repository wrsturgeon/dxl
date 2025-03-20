#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    core::mem::MaybeUninit,
    cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER},
    defmt_rtt as _,
    dxl_driver::mutex::Mutex as _,
    dxl_packet::control_table::Baud,
    dxl_rp::Actuator,
    embassy_executor::Spawner,
    embassy_rp::{
        bind_interrupts,
        gpio::{Level, Output},
        peripherals::{DMA_CH0, PIO0, UART0, USB},
        pio::{self, Pio},
        uart, usb,
    },
    embassy_time::{Duration, Timer},
    panic_probe as _,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    UART0_IRQ => uart::InterruptHandler<UART0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const DXL_ID: u8 = 1;
const CURRENT_BAUD: u32 = 1_000_000;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let mut control = {
        // CYW43 wireless board
        static STATE: StaticCell<cyw43::State> = StaticCell::new();

        let (_net_device, _bt_device, mut control, runner) = cyw43::new_with_bluetooth(
            STATE.init(cyw43::State::new()),
            Output::new(p.PIN_23, Level::Low),
            {
                let mut pio = Pio::new(p.PIO0, Irqs);
                PioSpi::new(
                    &mut pio.common,
                    pio.sm0,
                    RM2_CLOCK_DIVIDER,
                    pio.irq0,
                    Output::new(p.PIN_25, Level::High),
                    p.PIN_24,
                    p.PIN_29,
                    p.DMA_CH0,
                )
            },
            include_bytes!("../cyw43-firmware/43439A0.bin"),
            include_bytes!("../cyw43-firmware/43439A0_btfw.bin"),
        )
        .await;

        {
            // CYW43 WiFi/Bluetooth task (also controls onboard LED for some reason):
            #[embassy_executor::task]
            async fn task(
                runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
            ) -> ! {
                runner.run().await
            }
            let () = match spawner.spawn(task(runner)) {
                Ok(()) => {}
                Err(e) => defmt::error!("Error spawning CYW43 task: {}", e),
            };
        }

        let () = control
            .init(include_bytes!("../cyw43-firmware/43439A0_clm.bin"))
            .await;

        control
    };

    let dxl_bus = dxl_rp::bus(
        CURRENT_BAUD,
        p.PIN_13,
        p.UART0,
        p.PIN_16,
        p.PIN_17,
        Irqs,
        p.DMA_CH1,
        p.DMA_CH2,
    );

    let mut bus = match dxl_bus.lock().await {
        Ok(ok) => ok,
        Err(e) => {
            defmt::error!("{}", e);
            loop {}
        }
    };
    defmt::info!("  0: {}", bus.ping::<0>().await);
    defmt::info!("  1: {}", bus.ping::<1>().await);
    defmt::info!("  2: {}", bus.ping::<2>().await);
    defmt::info!("  3: {}", bus.ping::<3>().await);
    defmt::info!("  4: {}", bus.ping::<4>().await);
    defmt::info!("  5: {}", bus.ping::<5>().await);
    defmt::info!("  6: {}", bus.ping::<6>().await);
    defmt::info!("  7: {}", bus.ping::<7>().await);
    defmt::info!("  8: {}", bus.ping::<8>().await);
    defmt::info!("  9: {}", bus.ping::<9>().await);
    defmt::info!(" 10: {}", bus.ping::<10>().await);
    defmt::info!(" 11: {}", bus.ping::<11>().await);
    defmt::info!(" 12: {}", bus.ping::<12>().await);
    defmt::info!(" 13: {}", bus.ping::<13>().await);
    defmt::info!(" 14: {}", bus.ping::<14>().await);
    defmt::info!(" 15: {}", bus.ping::<15>().await);
    defmt::info!(" 16: {}", bus.ping::<16>().await);
    defmt::info!(" 17: {}", bus.ping::<17>().await);
    defmt::info!(" 18: {}", bus.ping::<18>().await);
    defmt::info!(" 19: {}", bus.ping::<19>().await);
    defmt::info!(" 20: {}", bus.ping::<20>().await);
    defmt::info!(" 21: {}", bus.ping::<21>().await);
    defmt::info!(" 22: {}", bus.ping::<22>().await);
    defmt::info!(" 23: {}", bus.ping::<23>().await);
    defmt::info!(" 24: {}", bus.ping::<24>().await);
    defmt::info!(" 25: {}", bus.ping::<25>().await);
    defmt::info!(" 26: {}", bus.ping::<26>().await);
    defmt::info!(" 27: {}", bus.ping::<27>().await);
    defmt::info!(" 28: {}", bus.ping::<28>().await);
    defmt::info!(" 29: {}", bus.ping::<29>().await);

    defmt::info!("done; halting.");
}
