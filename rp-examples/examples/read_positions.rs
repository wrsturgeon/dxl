#![no_std]
#![no_main]
/*
#![expect(
    incomplete_features,
    reason = "`generic_const_exprs` necessary to construct Dynamixel packets on the stack"
)]
#![feature(generic_const_exprs, impl_trait_in_assoc_type)]
*/
#![feature(impl_trait_in_assoc_type)]

use {
    cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER},
    defmt_rtt as _,
    dxl_driver::mutex::Mutex as _,
    dxl_packet::recv,
    dxl_rp::serial,
    embassy_executor::Spawner,
    embassy_futures::yield_now,
    embassy_net::{IpEndpoint, IpAddress, Ipv4Address, udp::{self, UdpSocket}},
    embassy_rp::{
        bind_interrupts,
        gpio::{Level, Output},
        peripherals::{DMA_CH0, PIO0, UART0},
        pio::{self, Pio},
        uart,
    },
    embassy_time::{Duration, Instant},
    panic_probe as _,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    UART0_IRQ => uart::InterruptHandler<UART0>;
});

const FRAME_DURATION: Duration = Duration::from_millis(20);

const UDP_ADDR: Ipv4Address = Ipv4Address::new(169, 254, 1, 1); // Ipv4Address::new(192, 168, 4, 1);
const UDP_DEST: Ipv4Address = Ipv4Address::new(169, 254, 197, 30); // Ipv4Address::new(192, 168, 4, 2);
const UDP_PORT: u16 = 5_000;

const BAUD: u32 = 4_000_000;

const CYW43_POWER_MANAGEMENT: cyw43::PowerManagementMode = cyw43::PowerManagementMode::None; // cyw43::PowerManagementMode::PowerSave;

// const UDP_BUFFER_SIZE: usize = 256;
const UDP_RX_BUFFER_SIZE: usize = 256;
const UDP_TX_BUFFER_SIZE: usize = 256;
const UDP_RX_META_SIZE: usize = 256;
const UDP_TX_META_SIZE: usize = 256;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    static NET_RESOURCES: StaticCell<embassy_net::StackResources<3>> = StaticCell::new();

    let p = embassy_rp::init(Default::default());

    let (net_device, mut control) = {
        // CYW43 wireless board
        static STATE: StaticCell<cyw43::State> = StaticCell::new();

        let (net_device, _bt_device, mut control, runner) = cyw43::new_with_bluetooth(
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
                Ok(()) => defmt::info!("Spawned CYW43 task"),
                Err(e) => defmt::panic!("Error spawning CYW43 task: {}", e),
            };
        }

        let () = control
            .init(include_bytes!("../cyw43-firmware/43439A0_clm.bin"))
            .await;
        let () = control.set_power_management(CYW43_POWER_MANAGEMENT).await;

        (net_device, control)
    };

    let net_stack = {
        let (stack, runner) = embassy_net::new(
            net_device,
            embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
                address: embassy_net::Ipv4Cidr::new(
                             UDP_ADDR,
                    16,
                ),
                dns_servers: heapless::Vec::new(),
                gateway: None,
            }),
            // embassy_net::Config::dhcpv4(Default::default()),
            NET_RESOURCES.init(embassy_net::StackResources::new()),
            rand_core::RngCore::next_u64(&mut embassy_rp::clocks::RoscRng),
        );

        {
            // Wireless task:
            #[embassy_executor::task]
            async fn task(
                mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>,
            ) -> ! {
                runner.run().await
            }
            let () = match spawner.spawn(task(runner)) {
                Ok(()) => defmt::info!("Spawned wireless networking task"),
                Err(e) => defmt::panic!("Error spawning wireless networking task: {}", e),
            };
        }

        stack
    };

    control.start_ap_wpa2("picomixel", "spectral", 5).await;

    defmt::info!("Waiting for DHCP...");
    'wait_for_dhcp: loop {
        if let Some(config) = net_stack.config_v4() {
            defmt::info!("IP address: {}", config.address.address());
            break 'wait_for_dhcp;
        }
        embassy_futures::yield_now().await;
    }

    let mut rx_buffer: [u8; UDP_RX_BUFFER_SIZE] = [0; UDP_RX_BUFFER_SIZE];
    let mut tx_buffer: [u8; UDP_TX_BUFFER_SIZE] = [0; UDP_TX_BUFFER_SIZE];
    let mut rx_meta: [udp::PacketMetadata; UDP_RX_META_SIZE] =
        [udp::PacketMetadata::EMPTY; UDP_RX_META_SIZE];
    let mut tx_meta: [udp::PacketMetadata; UDP_TX_META_SIZE] =
        [udp::PacketMetadata::EMPTY; UDP_TX_META_SIZE];
    let mut socket = UdpSocket::new(
        net_stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    match socket.bind(UDP_PORT) {
        Ok(()) => {}
        Err(e) => defmt::panic!("Error binding UDP socket: {}", e),
    }

    let dxl_bus_mutex = dxl_rp::bus(
        BAUD,
        p.PIN_13,
        p.UART0,
        p.PIN_16,
        p.PIN_17,
        Irqs,
        p.DMA_CH1,
        p.DMA_CH2,
    );
    let mut bus = match dxl_bus_mutex.lock().await {
        Ok(ok) => ok,
        Err(e) => {
            defmt::error!("Couldn't acquire the Dynamixel bus mutex lock: {}", e);
            loop {}
        }
    };

    let mut active = [false; dxl_packet::N_IDS as usize];
    let mut position = [32768_u16; dxl_packet::N_IDS as usize];
    // let mut udp_buffer = [0; UDP_BUFFER_SIZE];
    let mut osc_buffer: [u8; 10] = [
        b'/',
        b'2',
        b'5',
        b'2',
        b'/',
        b'6',
        b'5',
        b'5',
        b'3',
        b'5',
    ];

    let mut next_frame = Instant::now();

    loop {

        defmt::debug!("Main loop...");

    // Iterating through 253 booleans will be so fast that it makes no sense to
    // cache active IDs as a smaller datatype, since that would require
    // maintaining a set of booleans AND a dynamic list that continually match,
    // and IDs tend to be smaller rather than larger in the first place
    for (i, id) in (dxl_packet::MIN_ID..=dxl_packet::MAX_ID).enumerate() {

        // defmt::debug!("ID {}...", id);

        // Check if we've received a UDP ping, in which case we should
        // measure each active servo's position and return them all:
        if Instant::now() > next_frame {
            next_frame += FRAME_DURATION;

            // Update positions all at once, s.t. we're
            // measuring them as close together as possible:
            for (i, id) in (dxl_packet::MIN_ID..=dxl_packet::MAX_ID).enumerate() {
                if active[i] {
                    match bus.read_present_position(id).await {
                        Ok(dxl_packet::recv::Read { bytes }) => position[i] = u32::from_le_bytes(bytes) as _,
                        Err(e) => defmt::error!("{}", e),
                    };
                }
            }

            for (i, id) in (dxl_packet::MIN_ID..=dxl_packet::MAX_ID).enumerate() {
                if active[i] {
                    osc_buffer[1] = b'0' + (id / 100);
                    osc_buffer[2] = b'0' + ((id / 10) % 10);
                    osc_buffer[3] = b'0' + (id % 10);
                    let p = position[i];
                    osc_buffer[5] = b'0' + (p / 10000) as u8;
                    osc_buffer[6] = b'0' + ((p / 1000) % 10) as u8;
                    osc_buffer[7] = b'0' + ((p / 100) % 10) as u8;
                    osc_buffer[8] = b'0' + ((p / 10) % 10) as u8;
                    osc_buffer[9] = b'0' + (p % 10) as u8;
                    match socket.send_to(&osc_buffer, IpEndpoint::new(IpAddress::Ipv4(UDP_DEST), UDP_PORT)).await {
                        Ok(()) => {}
                        Err(e) => defmt::error!("{}", e),
                    }
                    let () = socket.flush().await;
                    // defmt::info!("{}", core::str::from_utf8(&osc_buffer[..]).unwrap_or("[UTF-8 error]"));
                }
            }
        }

        // If this ID has been unresponsive, give it another chance:
        if !active[i] {
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
                    active[i] = true;
                }
                Err(dxl_driver::bus::Error::Io(dxl_driver::IoError::Recv(
                    serial::RecvError::TimedOut(_),
                ))) => {
                }
                Err(e) => {
                    defmt::info!("    --> ID {} responded! ERROR: {}", id, e,);
                    active[i] = true;
                }
            }
        }

    let () = yield_now().await;
    }

    let () = yield_now().await;
    }
}
