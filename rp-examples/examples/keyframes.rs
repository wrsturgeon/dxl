#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    cyw43_pio::{PioSpi, RM2_CLOCK_DIVIDER},
    defmt_rtt as _,
    embassy_executor::Spawner,
    embassy_net::udp::{self, UdpSocket},
    embassy_rp::{
        bind_interrupts,
        gpio::{Level, Output},
        peripherals::{DMA_CH0, PIO0, UART1, USB},
        pio::{self, Pio},
        uart, usb,
    },
    panic_probe as _,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    UART1_IRQ => uart::InterruptHandler<UART1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

// const BAUD: u32 = 1_000_000;

const CYW43_POWER_MANAGEMENT: cyw43::PowerManagementMode = cyw43::PowerManagementMode::None; // cyw43::PowerManagementMode::PowerSave;

const UDP_BUFFER_SIZE: usize = 256;
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
                    embassy_net::Ipv4Address::new(169, 254, 1, 1),
                    16,
                ),
                dns_servers: heapless::Vec::new(),
                gateway: None,
            }),
            NET_RESOURCES.init(embassy_net::StackResources::new()),
            rand_core::RngCore::next_u64(&mut embassy_rp::clocks::RoscRng),
        );

        {
            // Wireless networking task:
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

    match socket.bind(1234) {
        Ok(()) => {}
        Err(e) => defmt::panic!("Error binding UDP socket: {}", e),
    }

    /*
    let dxl_bus = dxl_rp::bus(
        BAUD, p.PIN_7, p.UART1, p.PIN_8, p.PIN_9, Irqs, p.DMA_CH1, p.DMA_CH2,
    );
    */

    let mut udp_buffer = [0; UDP_BUFFER_SIZE];

    'main_loop: loop {
        let (n_bytes, endpoint) = match socket.recv_from(&mut udp_buffer).await {
            Ok(ok) => ok,
            Err(e) => {
                defmt::error!("Error receiving a UDP packet: {}; discarding...", e);
                continue 'main_loop;
            }
        };
        defmt::debug!("UDP packet from {}", endpoint);

        let s = match core::str::from_utf8(&udp_buffer[..n_bytes]) {
            Err(_e) => {
                defmt::error!("Packet was not valid UTF-8: {:X}", udp_buffer[..n_bytes]);
                continue 'main_loop;
            }
            Ok(ok) => ok,
        };

        let mut bytes = s.bytes();
        match bytes.next() {
            None => {
                defmt::error!("Unexpected zero-size packet; discarding...");
                continue 'main_loop;
            }
            Some(b'/') => {}
            Some(other) => {
                defmt::error!("First character was {}, not '/'; discarding...", other);
                continue 'main_loop;
            }
        }
        let mut id: u8 = 0;
        'id: loop {
            match bytes.next() {
                None => {
                    defmt::error!("Unexpected end of packet while parsing ID; discarding...");
                    continue 'main_loop;
                }
                Some(b'/') => break 'id,
                Some(c @ b'0'..=b'9') => {
                    id = {
                        let Some(some) = id.checked_mul(10).and_then(|i| i.checked_add(c - b'0'))
                        else {
                            defmt::error!("ID too large; discarding...");
                            continue 'main_loop;
                        };
                        some
                    }
                }
                Some(other) => {
                    defmt::error!(
                        "Unexpected character ({}) while parsing ID; discarding...",
                        other
                    );
                    continue 'main_loop;
                }
            }
        }
        let mut pos: u16 = 0;
        'pos: loop {
            match bytes.next() {
                None | Some(0) => break 'pos,
                Some(c @ b'0'..=b'9') => {
                    pos = {
                        let Some(some) = pos
                            .checked_mul(10)
                            .and_then(|i| i.checked_add((c - b'0').into()))
                        else {
                            defmt::error!("Position too large; discarding...");
                            continue 'main_loop;
                        };
                        some
                    }
                }
                Some(other) => {
                    defmt::error!(
                        "Unexpected character ({}) while parsing position; discarding...",
                        other
                    );
                    continue 'main_loop;
                }
            }
        }

        /*
        let f_pos = pos as f32 / 65535_f32;
        let result = match id {
            1 => mouth_1.go_to(f_pos).await,
            2 => mouth_2.go_to(f_pos).await,
            3 => mouth_3.go_to(f_pos).await,
            _ => {
                defmt::error!("Invalid ID: {}", id);
                continue;
            }
        };
        match result {
            Ok(()) => {}
            Err(e) => defmt::error!(
                "Error sending a position to the actuator: {}; discarding...",
                e
            ),
        }
        */
    }
}
