#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    core::mem::MaybeUninit,
    cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER},
    defmt_rtt as _,
    dxl_rp::{Actuator, Mutex},
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

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    UART0_IRQ => uart::InterruptHandler<UART0>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const DXL_ID: u8 = 1;
const BAUD: u32 = 1_000_000;

#[repr(C, packed)]
struct SmolBuffer {
    bytes: [u8; 255],
    size: u8,
}

impl SmolBuffer {
    #[inline]
    fn read(&self) -> &[u8] {
        &self.bytes[..self.size as usize]
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    static MOST_RECENT_PACKET: StaticCell<Mutex<SmolBuffer>> = StaticCell::new();

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

    let most_recent_packet = MOST_RECENT_PACKET.init(Mutex::new());
    {
        // UDP reception task:
        #[embassy_executor::task]
        async fn task(buffer: Mutex<SmolBuffer>) -> ! {
            asdf
        }
    }

    let dxl_bus = dxl_rp::bus(
        BAUD, p.PIN_13, p.UART0, p.PIN_16, p.PIN_17, Irqs, p.DMA_CH1, p.DMA_CH2,
    );
    let mut actuator = {
        let mut maybe_uninit = MaybeUninit::uninit();
        'actuator: loop {
            match Actuator::<DXL_ID, _>::init_at_position(&dxl_bus, "Test Dynamixel", 0.5, 0.001)
                .await
            {
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
            let () = Timer::after(Duration::from_secs(1)).await;
        }
        unsafe { maybe_uninit.assume_init() }
    };
    match actuator.write_profile_acceleration(32).await {
        Ok(()) => {}
        Err(e) => defmt::error!("{}", e),
    }

    let mut next = Instant::now();
    let mut state = true;
    loop {
        let () = match actuator.go_to(if state { 1. } else { 0. }).await {
            Ok(()) => {}
            Err(e) => defmt::error!("Error writing Dynamixel goal position: {}", e),
        };

        state = !state;
        next += Duration::from_millis(1500);
        let () = Timer::at(next).await;
    }
}
