#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    defmt_rtt as _,
    dxl_driver::comm::Comm as _,
    dxl_packet::packet::{self, recv},
    dxl_rp::Comm,
    embassy_executor::Spawner,
    embassy_rp::{
        bind_interrupts,
        peripherals::{UART1, USB},
        uart, usb,
    },
    embassy_time::Instant,
    panic_probe as _,
};

bind_interrupts!(struct Irqs {
    UART1_IRQ => uart::InterruptHandler<UART1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const BAUD: u32 = 1_000_000;

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
            Ok(()) => {
                log::info!("Spawned USB task");
                defmt::info!("Spawned USB task");
            }
            Err(e) => {
                log::error!("Error spawning USB task: {e}");
                defmt::panic!("Error spawning USB task: {}", e);
            }
        };
    }

    let mut dxl_comm = Comm::new(
        BAUD, p.PIN_7, p.UART1, p.PIN_8, p.PIN_9, Irqs, p.DMA_CH1, p.DMA_CH2,
    );

    // Format: `9999.999s`
    let mut timestamp_buffer = [b' ', b' ', b' ', b' ', b'.', b' ', b' ', b' ', b's'];
    let start = Instant::now();

    'parse_another_packet: loop {
        let mut stream = dxl_comm.listen();

        let timestamp = {
            let mut ms: u64 = start.elapsed().as_millis();
            for i in (5..8).rev() {
                timestamp_buffer[i] = b'0' + ((ms % 10) as u8);
                ms /= 10;
            }
            for i in (0..4).rev() {
                if ms == 0 {
                    break;
                }
                timestamp_buffer[i] = b'0' + ((ms % 10) as u8);
                ms /= 10;
            }
            str::from_utf8(&timestamp_buffer).unwrap_or("[internal error: invalid UTF-8]")
        };

        macro_rules! next_byte {
            () => {
                match ::dxl_packet::stream::Stream::next(&mut stream).await {
                    Ok(ok) => ok,
                    Err(e @ dxl_rp::serial::RecvError::TimedOut(_)) => {
                        log::debug!("{timestamp}: {e}");
                        defmt::debug!("{}: {}", timestamp, e);
                        continue 'parse_another_packet;
                    }
                    Err(e) => {
                        log::error!("{timestamp}: {e}");
                        defmt::error!("{}: {}", timestamp, e);
                        continue 'parse_another_packet;
                    }
                }
            };
        }

        let b = next_byte!();
        if b != 0xFF {
            log::debug!(
                "{timestamp}: Invalid first header byte: expected `0xFF` but received `x{b:X?}`"
            );
            defmt::debug!(
                "{}: Invalid first header byte: expected `0xFF` but received `x{:X}`",
                timestamp,
                b,
            );
            continue 'parse_another_packet;
        }

        let b = next_byte!();
        if b != 0xFF {
            log::debug!(
                "{timestamp}: Invalid second header byte: expected `0xFF` but received `x{b:X?}`"
            );
            defmt::debug!(
                "{}: Invalid second header byte: expected `0xFF` but received `x{:X}`",
                timestamp,
                b,
            );
            continue 'parse_another_packet;
        }

        let b = next_byte!();
        if b != 0xFD {
            log::debug!(
                "{timestamp}: Invalid third header byte: expected `0xFD` but received `x{b:X?}`"
            );
            defmt::debug!(
                "{}: Invalid third header byte: expected `0xFD` but received `x{:X}`",
                timestamp,
                b,
            );
            continue 'parse_another_packet;
        }

        let b = next_byte!();
        if b != 0x00 {
            log::debug!(
                "{timestamp}: Invalid reserved byte: expected `0x00` but received `x{b:X?}`"
            );
            defmt::debug!(
                "{}: Invalid reserved byte: expected `0x00` but received `x{:X}`",
                timestamp,
                b,
            );
            continue 'parse_another_packet;
        }

        let id = next_byte!();
        if (id < dxl_packet::MIN_ID) || (id > dxl_packet::MAX_ID) {
            log::error!(
                "{timestamp}: Invalid ID: minimum is {} and maximum is {} but found {id}",
                dxl_packet::MIN_ID,
                dxl_packet::MAX_ID,
            );
            defmt::error!(
                "{}: Invalid ID: minimum is {} and maximum is {} but found {}",
                timestamp,
                dxl_packet::MIN_ID,
                dxl_packet::MAX_ID,
                id,
            );
            continue 'parse_another_packet;
        }

        let length = {
            let length_lo = next_byte!();
            let length_hi = next_byte!();
            u16::from_le_bytes([length_lo, length_hi])
        };

        let instruction = {
            let insn_byte = next_byte!();
            let Some(ok) = dxl_packet::packet::Instruction::from_repr(insn_byte) else {
                log::debug!("{timestamp}: Unrecognized instruction: `x{insn_byte:X?}`");
                defmt::debug!(
                    "{}: Unrecognized instruction: `x{:X}`",
                    timestamp,
                    insn_byte
                );
                continue 'parse_another_packet;
            };
            ok
        };

        let direction = if matches!(instruction, dxl_packet::packet::Instruction::Status) {
            // DXL -> MCU (status/return packet)
            // Has an extra byte for error reporting.
            let b = next_byte!();
            match recv::SoftwareError::check(b) {
                Err(_) => {
                    log::debug!("{timestamp}: Unrecognized error code: `x{b:X?}`");
                    defmt::debug!("{}: Unrecognized error code: `x{:X}`", timestamp, b);
                    continue 'parse_another_packet;
                }
                Ok(None) => {
                    // No software errors: woohoo!
                    "<-"
                }
                Ok(Some(e)) => {
                    // Error successfully brought to our awareness!
                    log::error!("{timestamp}: <- ID {id:>3}: {e}");
                    defmt::error!("{}: <- ID {}: {}", timestamp, id, e);
                    continue 'parse_another_packet;
                }
            }
        } else {
            // MCU -> DXL (status/return packet)
            // Doesn't have an extra byte for error reporting.
            "->"
        };

        let suffix = match instruction {
            dxl_packet::packet::Instruction::Ping => {
                let model_number = {
                    let lsb = next_byte!();
                    let msb = next_byte!();
                    u16::from_le_bytes([lsb, msb])
                };
                let firmware_version = next_byte!();
                log::info!(
                    "{timestamp}: {direction} ID {id:>3}: Ping: Model #{model_number} running firmware version #{firmware_version}"
                );
                log::info!(
                    "{}: {} ID {}: Ping: Model #{} running firmware version #{}",
                    timestamp,
                    direction,
                    id,
                    model_number,
                    firmware_version
                );
            }
            dxl_packet::packet::Instruction::Read => match direction {
                "->" => {
                    let offset = {
                        let lsb = next_byte!();
                        let msb = next_byte!();
                        u16::from_le_bytes([lsb, msb])
                    };
                    let name = packet::ControlTableAddress::from_repr(offset)
                        .unwrap_or(packet::ControlTableAddress::Unrecognized);
                    let length = {
                        let lsb = next_byte!();
                        let msb = next_byte!();
                        u16::from_le_bytes([lsb, msb])
                    };
                    log::info!(
                        "{timestamp}: {direction} ID {id:>3}: Reading {name}: Requesting {length} bytes at offset {offset}/x{offset:X?}",
                    );
                    defmt::info!(
                        "{}: {} ID {}: Reading {}: Requesting {} bytes at offset {}/x{:X}",
                        timestamp,
                        direction,
                        id,
                        name,
                        length,
                        offset,
                        offset,
                    );
                }
                "<-" => {
                    log::info!("{timestamp}: {direction} ID {id:>3}: Read: Successful");
                    defmt::info!("{}: {} ID {}: Read: Successful", timestamp, direction, id);
                }
                _ => {
                    log::error!(
                        "{timestamp}: Internal error: Read: Unrecognized direction: \"{direction}\""
                    );
                    defmt::error!(
                        "{}: Internal error: Read: Unrecognized direction: \"{}\"",
                        timestamp,
                        direction
                    );
                    continue 'parse_another_packet;
                }
            },
            dxl_packet::packet::Instruction::Write => match direction {
                "->" => {
                    let offset = {
                        let lsb = next_byte!();
                        let msb = next_byte!();
                        u16::from_le_bytes([lsb, msb])
                    };
                    let name = packet::ControlTableAddress::from_repr(offset)
                        .unwrap_or(packet::ControlTableAddress::Unrecognized);
                    let length = {
                        let lsb = next_byte!();
                        let msb = next_byte!();
                        u16::from_le_bytes([lsb, msb])
                    };
                    log::info!(
                        "{timestamp}: {direction} ID {id:>3}: Writing {name}: Requesting {length} bytes at offset {offset}/x{offset:X?}",
                    );
                    defmt::info!(
                        "{}: {} ID {}: Writing {}: Requesting {} bytes at offset {}/x{:X}",
                        timestamp,
                        direction,
                        id,
                        name,
                        length,
                        offset,
                        offset,
                    );
                }
                "<-" => {
                    log::info!("{timestamp}: {direction} ID {id:>3}: Write: Successful");
                    defmt::info!("{}: {} ID {}: Write: Successful", timestamp, direction, id,);
                }
                _ => {
                    log::error!(
                        "{timestamp}: Internal error: Write: Unrecognized direction: \"{direction}\""
                    );
                    defmt::error!(
                        "{}: Internal error: Write: Unrecognized direction: \"{}\"",
                        timestamp,
                        direction
                    );
                    continue 'parse_another_packet;
                }
            },
            _ => {
                log::info!("{timestamp}: {direction} ID {id:>3}: {instruction}");
                defmt::info!("{}: {} ID {}: {}", timestamp, direction, id, instruction);
            }
        };
    }
}
