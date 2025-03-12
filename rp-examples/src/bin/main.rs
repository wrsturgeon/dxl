#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type, generic_const_exprs)]
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to construct Dynamixel packets on the stack"
)]

use {
    core::mem::MaybeUninit,
    cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER},
    defmt_rtt as _,
    embassy_executor::Spawner,
    embassy_rp::{
        bind_interrupts,
        gpio::{Level, Output},
        peripherals::{DMA_CH0, PIO0, UART0, USB},
        pio::{self, Pio},
        uart, usb,
    },
    embassy_time::{Duration, Instant, Timer},
    panic_probe as _,
    static_cell::StaticCell,
};

/*
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
*/

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    UART0_IRQ => uart::InterruptHandler<UART0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const DXL_ID: u8 = 1;
const BAUD: u32 = 57600;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    /*
    {
        // USB logging task:
        #[embassy_executor::task]
        pub async fn task(usb_driver: usb::Driver<'static, USB>) {
            embassy_usb_logger::run!(1024, log::LevelFilter::Debug, usb_driver);
        }
        let () = match spawner.spawn(task(usb::Driver::new(p.USB, Irqs))) {
            Ok(()) => {}
            Err(e) => log::error!("{e}"),
        };
    }
    */

    /*
    {
        #[embassy_executor::task]
        async fn task() {
            loop {
                let bs = ();
                let () = defmt::debug!("Stack: {:X}", &bs as *const _ as *const ());
                let () = yield_now().await;
            }
        }
        let () = match spawner.spawn(task()) {
            Ok(()) => {}
            Err(e) => defmt::error!("{}", e),
        };
    }
    */

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
            include_bytes!("../../cyw43-firmware/43439A0.bin"),
            include_bytes!("../../cyw43-firmware/43439A0_btfw.bin"),
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
            .init(include_bytes!("../../cyw43-firmware/43439A0_clm.bin"))
            .await;

        control
    };

    let dxl_bus = dxl_rp::bus(
        BAUD, p.PIN_13, p.UART0, p.PIN_16, p.PIN_17, Irqs, p.DMA_CH1, p.DMA_CH2,
    );
    let actuator = {
        let mut maybe_uninit = MaybeUninit::uninit();
        'actuator: loop {
            match dxl_bus.id::<DXL_ID>().await {
                Ok(ok) => {
                    maybe_uninit.write(ok);
                    break 'actuator;
                }
                Err(e) => defmt::warn!(
                    "Error initializing Dynamixel ID {}: {}; retrying...",
                    DXL_ID,
                    e
                ),
            }
            // let () = Timer::after(Duration::from_secs(1)).await;
        }
        unsafe { maybe_uninit.assume_init() }
    };

    let mut next = Instant::now();
    let mut state = true;
    loop {
        let () = control.gpio_set(0, state).await;
        defmt::info!("{}", state);
        defmt::info!("");

        let () = match actuator
            .write_goal_position(if state { 4095 } else { 0 })
            .await
        {
            Ok(()) => {}
            Err(e) => defmt::error!("Error writing Dynamixel goal position: {}", e),
        };

        state = !state;
        next += Duration::from_millis(1500);
        let () = Timer::at(next).await;
    }
}
