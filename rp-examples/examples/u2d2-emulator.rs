#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    defmt_rtt as _,
    dxl_driver::comm::Comm as _,
    dxl_packet::stream::Stream as _,
    dxl_rp::{serial::RecvError, Comm}, // pull_high::PullHigh,
    embassy_executor::Spawner,
    embassy_futures::join::{join, join3},
    embassy_rp::{
        bind_interrupts, gpio,
        peripherals::{UART1, USB},
        uart,
        usb::{Driver, Instance, InterruptHandler},
    },
    embassy_sync::{blocking_mutex::raw::NoopRawMutex, pipe::Pipe},
    embassy_usb::{
        Builder, Config, UsbDevice,
        class::cdc_acm::{CdcAcmClass, Receiver, Sender, State},
        driver::EndpointError,
    },
    embedded_io_async::{Read, Write},
    panic_probe as _,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
    UART1_IRQ => uart::InterruptHandler<UART1>;
    USBCTRL_IRQ => InterruptHandler<USB>;
});

const BAUD: u32 = 115_200;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 256]> = StaticCell::new();
    static STATE: StaticCell<State> = StaticCell::new();

    let p = embassy_rp::init(Default::default());

    // Create the driver, from the HAL.
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
    let mut usb_runner = builder.build();

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

    /*
    // Pipe setup
    let mut usb_pipe: Pipe<NoopRawMutex, 20> = Pipe::new();
    let (mut usb_pipe_reader, mut usb_pipe_writer) = usb_pipe.split();

    let mut uart_pipe: Pipe<NoopRawMutex, 20> = Pipe::new();
    let (mut uart_pipe_reader, mut uart_pipe_writer) = uart_pipe.split();
    */

    let (mut usb_tx, mut usb_rx) = usb_cdc_acm_class.split();




    let mut buffer = [0; 64];
    'main_loop: loop {
        let n = match usb_rx.read_packet(&mut buffer).await {
            Ok(ok) => ok,
            Err(e) => {
                defmt::error!("Error receiving via USB: {}", e);
                continue 'main_loop
            }
        };
        let usb_in = &buffer[..n];
        defmt::debug!("Received `{:x}` via USB", usb_in);
        if n <= 0 {
            continue 'main_loop
        }

        let mut stream = match comm.comm(usb_in).await {
            Ok(ok) => {
                defmt::debug!("Sent `{:x}` via UART", usb_in);
                ok
            },
            Err(e) => {
                defmt::error!("Error sending via UART: {}", e);
                continue 'main_loop
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
                    continue 'response
                }
            };
            let packet = &[byte];
            match usb_tx.write_packet(packet).await {
                Ok(()) => 
                    defmt::debug!("Wrote `{:x}` via USB", packet),
                    Err(e)=>defmt::error!("Error sending via USB: {}", e),
            }
        }
    }

    /*
    // Read + write from USB
    let usb_future = async {
        loop {
            defmt::info!("Waiting for USB connection...");
            usb_rx.wait_connection().await;
            defmt::info!("Connected");
            let _ = join(
                usb_read(&mut usb_rx, &mut uart_pipe_writer),
                usb_write(&mut usb_tx, &mut usb_pipe_reader),
            )
            .await;
            defmt::info!("Disconnected");
        }
    };

    // Read + write from UART
    let uart_future = join(
        uart_read(&mut uart_rx, &mut usb_pipe_writer),
        uart_write(&mut tx_enable, &mut uart_tx, &mut uart_pipe_reader),
    );

    // Run everything concurrently.
    // If we had made everything `'static` above instead, we could do this using separate tasks instead.
    join3(usb_fut, usb_future, uart_future).await;
    */
}

/*

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => defmt::panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

/// Read from the USB and write it to the UART TX pipe
async fn usb_read<'d, T: Instance + 'd>(
    usb_rx: &mut Receiver<'d, Driver<'d, T>>,
    uart_pipe_writer: &mut embassy_sync::pipe::Writer<'_, NoopRawMutex, 20>,
) -> Result<(), Disconnected> {
    let mut buf = [0; 64];
    loop {
        let n = usb_rx.read_packet(&mut buf).await?;
        let data = &buf[..n];
        defmt::debug!("USB IN: {:x}", data);
        (*uart_pipe_writer).write(data).await;
    }
}

/// Read from the USB TX pipe and write it to the USB
async fn usb_write<'d, T: Instance + 'd>(
    usb_tx: &mut Sender<'d, Driver<'d, T>>,
    usb_pipe_reader: &mut embassy_sync::pipe::Reader<'_, NoopRawMutex, 20>,
) -> Result<(), Disconnected> {
    let mut buf = [0; 64];
    loop {
        let n = (*usb_pipe_reader).read(&mut buf).await;
        let data = &buf[..n];
        defmt::debug!("USB OUT: {:x}", data);
        usb_tx.write_packet(&data).await?;
    }
}

/// Read from the UART and write it to the USB TX pipe
async fn uart_read<PIO: pio::Instance, const SM: usize>(
    uart_rx: &mut PioUartRx<'_, PIO, SM>,
    usb_pipe_writer: &mut embassy_sync::pipe::Writer<'_, NoopRawMutex, 20>,
) -> ! {
    let mut buf = [0; 64];
    loop {
        let n = uart_rx.read(&mut buf).await.expect("UART read error");
        if n == 0 {
            continue;
        }
        let data = &buf[..n];
        defmt::debug!("UART IN: {:x}", buf);
        (*usb_pipe_writer).write(data).await;
    }
}

/// Read from the UART TX pipe and write it to the UART
async fn uart_write<'txen, PIO: pio::Instance, const SM: usize>(
    tx_enable: &mut gpio::Output<'txen>,
    uart_tx: &mut PioUartTx<'_, PIO, SM>,
    uart_pipe_reader: &mut embassy_sync::pipe::Reader<'_, NoopRawMutex, 20>,
) -> ! {
    let mut buf = [0; 64];
    loop {
        let n = (*uart_pipe_reader).read(&mut buf).await;
        let data = &buf[..n];
        defmt::debug!("UART OUT: {:x}", data);

        // Block incoming transmission ONLY WITHIN THIS SCOPE to allow outgoing transmission:
        let enable_tx = PullHigh::new(tx_enable);
        // Asynchronously ask hardware to transmit this buffer:
        let Ok(_usize) = uart_tx.write(&data).await;
        /*
        // Wait until it actually starts transmitting:
        while !self.uart.busy() {
            // let () = embassy_futures::yield_now().await;
        }
        */
        /*
        // Then wait until it finishes:
        while uart_tx.busy() {
            // let () = embassy_futures::yield_now().await;
        }
        */
        // Then lower the `tx_enable` pin by dropping `_enable_tx`:
        // NOTE: I'm pretty sure this could be implicit, but this couldn't hurt.
        drop(enable_tx);
    }
}

*/
