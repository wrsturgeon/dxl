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
    match bus.write_id::<17>([1]).await {
        Ok(()) => {}
        Err(e) => defmt::error!("{}", e),
    }

    defmt::info!("done; halting.");
}
