#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    defmt_rtt as _,
    dxl_driver::comm::Comm as _,
    dxl_packet::stream::Stream as _,
    dxl_rp::{Comm, serial::RecvError},
    embassy_executor::Spawner,
    embassy_rp::{
        bind_interrupts,
        peripherals::{UART1, USB},
        uart,
        usb::{Driver, InterruptHandler},
    },
    embassy_usb::{
        Builder, Config, UsbDevice,
        class::cdc_acm::{CdcAcmClass, State},
    },
    panic_probe as _,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
    UART1_IRQ => uart::InterruptHandler<UART1>;
    USBCTRL_IRQ => InterruptHandler<USB>;
});

const BAUD: u32 = 1_000_000; // 115_200;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 256]> = StaticCell::new();
    static STATE: StaticCell<State> = StaticCell::new();

    let p = embassy_rp::init(Default::default());

    // Create the USB driver from the HAL.
    let driver = Driver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Spectral Motion (Will Sturgeon, 2025)");
    config.product = Some("Picomixel U2D2 Emulator");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 256]);

    let state = STATE.init(State::new());

    let mut builder = Builder::new(
        driver,
        config,
        config_descriptor,
        bos_descriptor,
        &mut [], // no msos descriptors
        control_buf,
    );

    // Create classes on the builder.
    let usb_cdc_acm_class = CdcAcmClass::new(&mut builder, state, 64);

    // Build the builder.
    let usb_runner = builder.build();

    {
        // USB background task:
        #[embassy_executor::task]
        pub async fn task(
            mut runner: UsbDevice<'static, embassy_rp::usb::Driver<'static, USB>>,
        ) -> ! {
            runner.run().await
        }
        let () = match spawner.spawn(task(usb_runner)) {
            Ok(()) => defmt::info!("Spawned USB task"),
            Err(e) => defmt::panic!("Error spawning USB task: {}", e),
        };
    }

    let mut comm = Comm::new(
        BAUD, p.PIN_7, p.UART1, p.PIN_8, p.PIN_9, Irqs, p.DMA_CH1, p.DMA_CH2,
    );

    let (mut usb_tx, mut usb_rx) = usb_cdc_acm_class.split();

    let mut buffer = [0; 64];
    'main_loop: loop {
        let n = match usb_rx.read_packet(&mut buffer).await {
            Ok(ok) => ok,
            Err(e) => {
                defmt::error!("Error receiving via USB: {}", e);
                continue 'main_loop;
            }
        };
        let usb_in = &buffer[..n];
        defmt::debug!("Received `{:x}` via USB", usb_in);
        if n <= 0 {
            continue 'main_loop;
        }

        let mut stream = match comm.comm(usb_in).await {
            Ok(ok) => {
                defmt::debug!("Sent `{:x}` via UART", usb_in);
                ok
            }
            Err(e) => {
                defmt::error!("Error sending via UART: {}", e);
                continue 'main_loop;
            }
        };
        'response: loop {
            let byte: u8 = match stream.next().await {
                Ok(ok) => {
                    defmt::debug!("Received `{:x}` via UART", ok);
                    ok
                }
                Err(RecvError::TimedOut(_)) => break 'response,
                Err(e) => {
                    defmt::error!("Error receiving via UART: {}", e);
                    continue 'response;
                }
            };
            let packet = &[byte];
            match usb_tx.write_packet(packet).await {
                Ok(()) => defmt::debug!("Wrote `{:x}` via USB", packet),
                Err(e) => defmt::error!("Error sending via USB: {}", e),
            }
        }
    }
}
