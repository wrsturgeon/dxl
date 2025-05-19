#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER},
    defmt_rtt as _,
    dxl_driver::mutex::Mutex as _,
    embassy_executor::Spawner,
    embassy_net::{
        Ipv4Address,
        udp::{self, UdpSocket},
    },
    embassy_rp::{
        bind_interrupts,
        gpio::{Level, Output},
        peripherals::{DMA_CH0, PIO0, UART1},
        pio::{self, Pio},
        uart,
    },
    embassy_time::Timer,
    panic_probe as _,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    UART1_IRQ => uart::InterruptHandler<UART1>;
});

const UDP_DEST: Ipv4Address = Ipv4Address::new(169, 254, 197, 30);
const UDP_PORT: u16 = 5_000;

const BAUD: u32 = 57_600;

const CYW43_POWER_MANAGEMENT: cyw43::PowerManagementMode = cyw43::PowerManagementMode::None; // cyw43::PowerManagementMode::PowerSave;

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
                address: embassy_net::Ipv4Cidr::new(UDP_DEST, 16),
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

    let () = match control
        .join("picomixel", cyw43::JoinOptions::new(b"spectral"))
        .await
    {
        Ok(()) => {}
        Err(_) => defmt::panic!("Couldn't join WiFi network"),
    };

    // Wait for DHCP, not necessary when using static IP
    defmt::info!("waiting for DHCP...");
    while !net_stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    defmt::info!("DHCP is now up!");

    defmt::info!("waiting for link up...");
    while !net_stack.is_link_up() {
        Timer::after_millis(500).await;
    }
    defmt::info!("Link is up!");

    defmt::info!("waiting for stack to be up...");
    net_stack.wait_config_up().await;
    defmt::info!("Stack is up!");

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
        BAUD, p.PIN_7, p.UART1, p.PIN_8, p.PIN_9, Irqs, p.DMA_CH1, p.DMA_CH2,
    );
    let mut bus = match dxl_bus_mutex.lock().await {
        Ok(ok) => ok,
        Err(e) => defmt::panic!("Couldn't acquire the Dynamixel bus mutex lock: {}", e),
    };

    for id in [21, 22, 24] {
        defmt::info!(
            "{}",
            bus.write_profile_acceleration(id, 128_u32.to_le_bytes())
                .await
        );
        defmt::info!(
            "{}",
            bus.write_profile_velocity(id, 2048_u32.to_le_bytes()).await
        );
        defmt::info!("{}", bus.write_torque_enable(id, [1]).await);
    }

    let mut osc_buffer: [u8; 10] = [b'/', b'2', b'5', b'2', b'/', b'6', b'5', b'5', b'3', b'5'];

    'main_loop: loop {
        defmt::debug!("Main loop...");

        let (n_bytes, endpoint) = match socket.recv_from(&mut osc_buffer).await {
            Err(e) => {
                defmt::error!("{}", e);
                continue 'main_loop;
            }
            Ok(ok) => ok,
        };
        defmt::info!(
            "Received {} ({} bytes) from {}",
            core::str::from_utf8(&osc_buffer[..n_bytes]).ok(),
            n_bytes,
            endpoint
        );

        if n_bytes != 10 {
            continue 'main_loop;
        }

        let id: u8 = match &osc_buffer[..5] {
            b"/021/" => 21,
            b"/022/" => 22,
            // b"/023/" => 23,
            b"/024/" => 24,
            // b"/025/" => &mut positions.p25,
            // b"/026/" => &mut positions.p26,
            // b"/031/" => &mut positions.p31,
            // b"/032/" => &mut positions.p32,
            // b"/033/" => &mut positions.p33,
            // b"/034/" => &mut positions.p34,
            // b"/035/" => &mut positions.p35,
            // b"/036/" => &mut positions.p36,
            // b"/041/" => &mut positions.p41,
            // b"/042/" => &mut positions.p42,
            // b"/043/" => &mut positions.p43,
            // b"/044/" => &mut positions.p44,
            // b"/045/" => &mut positions.p45,
            // b"/046/" => &mut positions.p46,
            other => {
                defmt::warn!("unrecognized ID: {:?}", core::str::from_utf8(other).ok());
                continue 'main_loop;
            }
        };

        let mut position = {
            let mut i = (osc_buffer[5] - b'0') as u16;
            i = 10 * i + (osc_buffer[6] - b'0') as u16;
            i = 10 * i + (osc_buffer[7] - b'0') as u16;
            i = 10 * i + (osc_buffer[8] - b'0') as u16;
            i = 10 * i + (osc_buffer[9] - b'0') as u16;
            i
        };
        match id {
            22 => {
                position -= 1024;
                position = 4095 - position;
            }
            24 => position = 4095 - position,
            _ => {}
        }

        defmt::info!("Sending {} to {}...", id, position);

        let [lo, hi] = position.to_le_bytes();
        match bus.write_goal_position(id, [lo, hi, 0, 0]).await {
            Ok(()) => defmt::info!("    done"),
            Err(e) => defmt::error!("    FAILED: {}", e),
        };
    }
}
