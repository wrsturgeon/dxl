#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    defmt_rtt as _,
    dxl_driver::comm::Comm as _,
    dxl_packet::stream::Stream as _,
    dxl_rp::{Comm, serial::RecvError},
    embassy_executor::Spawner,
    embassy_futures::select::{Either, select},
    embassy_rp::{
        bind_interrupts,
        flash::{self, Flash},
        gpio,
        peripherals::{TRNG, UART1, USB},
        trng::{self, Trng},
        uart, usb,
    },
    embassy_time::{Duration, Instant, TimeoutError, Timer, with_timeout},
    embassy_usb::{
        self, Builder, UsbDevice,
        class::cdc_acm::{self, CdcAcmClass},
    },
    panic_probe as _,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
    TRNG_IRQ => trng::InterruptHandler<TRNG>;
    UART1_IRQ => uart::InterruptHandler<UART1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const N_RECORDING_SLOTS: usize = 1;
const RECORDING_SLOT_SIZE_BYTES: usize = 1_usize << (18 + 3);
const RECORDING_BUFFER_SIZE_BYTES: usize = N_RECORDING_SLOTS * RECORDING_SLOT_SIZE_BYTES;
const FLASH_SIZE_MIB: usize = 4096;
const FLASH_SIZE_BYTES: usize = FLASH_SIZE_MIB * 1024;
const FLASH_START: usize = 0x10_00_00_00;

const BAUD: u32 = 115_200;

const DEBOUNCING_TIME: Duration = Duration::from_millis(5);

#[used]
#[unsafe(link_section = ".recording_buffer")]
static RECORDING_BUFFER: [u8; RECORDING_BUFFER_SIZE_BYTES] = [0xFF; RECORDING_BUFFER_SIZE_BYTES];

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    static CDC_ACM_CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CDC_ACM_BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CDC_ACM_CONTROL_BUF: StaticCell<[u8; 256]> = StaticCell::new();
    static CDC_ACM_STATE: StaticCell<cdc_acm::State> = StaticCell::new();

    let p = embassy_rp::init(Default::default());

    let mut playback_pin = gpio::Input::new(p.PIN_18, gpio::Pull::Up);
    let () = playback_pin.set_schmitt(true);

    let mut record_switch = gpio::Input::new(p.PIN_19, gpio::Pull::Up);
    let () = record_switch.set_schmitt(true);

    let recording_slots: &[[u8; RECORDING_SLOT_SIZE_BYTES]; N_RECORDING_SLOTS] =
        unsafe { &*(&RECORDING_BUFFER as *const [u8; RECORDING_BUFFER_SIZE_BYTES]).cast() };
    let recording_slot_offsets = recording_slots.each_ref().map(
        |slot: &[u8; RECORDING_SLOT_SIZE_BYTES]| match u32::try_from(
            slot.as_ptr() as usize - FLASH_START,
        ) {
            Ok(ok) => ok,
            Err(e) => defmt::panic!("Couldn't fit a flash offset into a `u32`: {}", e),
        },
    );
    let mut n_slots_initialized: u8 = 0;
    let mut selected_recording_slot: u8 = 0;

    let mut flash = Flash::<_, _, FLASH_SIZE_BYTES>::new(p.FLASH, p.DMA_CH0);
    for (i, &offset) in recording_slot_offsets.iter().enumerate() {
        if slot_is_initialized(&mut flash, offset).await {
            defmt::info!("Recording slot #{} has a recording stored!", i);
            n_slots_initialized += 1;
        } else {
            defmt::info!("Recording slot #{} is not in a known state; erasing...", i);
            match flash.blocking_erase(
                offset,
                offset + defmt::unwrap!(u32::try_from(RECORDING_SLOT_SIZE_BYTES)),
            ) {
                Ok(()) => defmt::info!("    done"),
                Err(e) => defmt::error!("    ERROR: {}", e),
            }
        }
    }

    let mut dxl_comm = Comm::new(
        BAUD, p.PIN_7, p.UART1, p.PIN_8, p.PIN_9, Irqs, p.DMA_CH1, p.DMA_CH2,
    );

    let mut rng = Trng::new(p.TRNG, Irqs, trng::Config::default());

    // Create the USB driver from the HAL.
    let cdc_acm_driver = usb::Driver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let mut usb_config = embassy_usb::Config::new(0xC0DE, 0xCAFE);
    usb_config.manufacturer = Some("Spectral Motion (Will Sturgeon, 2025)");
    usb_config.product = Some("Picomixel U2D2 Emulator");
    usb_config.serial_number = Some("12345678");
    usb_config.max_power = 100;
    usb_config.max_packet_size_0 = 64;

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let cdc_acm_config_descriptor = CDC_ACM_CONFIG_DESCRIPTOR.init([0; 256]);
    let cdc_acm_bos_descriptor = CDC_ACM_BOS_DESCRIPTOR.init([0; 256]);
    let cdc_acm_control_buf = CDC_ACM_CONTROL_BUF.init([0; 256]);

    let cdc_acm_state = CDC_ACM_STATE.init(cdc_acm::State::new());

    let mut cdc_acm_builder = Builder::new(
        cdc_acm_driver,
        usb_config,
        cdc_acm_config_descriptor,
        cdc_acm_bos_descriptor,
        &mut [], // no msos descriptors
        cdc_acm_control_buf,
    );

    // Create classes on the builder.
    let cdc_acm_class = CdcAcmClass::new(&mut cdc_acm_builder, cdc_acm_state, 64);

    // Build the builder.
    let cdc_acm_runner = cdc_acm_builder.build();

    {
        // USB background task:
        #[embassy_executor::task]
        pub async fn task(mut runner: UsbDevice<'static, usb::Driver<'static, USB>>) -> ! {
            runner.run().await
        }
        let () = match spawner.spawn(task(cdc_acm_runner)) {
            Ok(()) => defmt::debug!("Spawned USB task"),
            Err(e) => defmt::panic!("Error spawning USB task: {}", e),
        };
    }

    let (mut usb_tx, mut usb_rx) = cdc_acm_class.split();

    let mut record_switch_ever_used = false;
    loop {
        let button_pressed = select(
            wait_for_low_debounced(&mut record_switch),
            wait_for_low_debounced(&mut playback_pin),
        );
        match button_pressed.await {
            Either::First(()) => {
                let () = trigger_recording(
                    &mut selected_recording_slot,
                    &mut n_slots_initialized,
                    &mut record_switch,
                    &mut usb_rx,
                    &mut usb_tx,
                    &mut dxl_comm,
                    &mut flash,
                    &recording_slot_offsets,
                )
                .await;
                record_switch_ever_used = true;
            }
            Either::Second(()) => {
                let () = trigger_playback(
                    &mut selected_recording_slot,
                    n_slots_initialized,
                    record_switch_ever_used,
                    &mut dxl_comm,
                    &mut flash,
                    &recording_slot_offsets,
                    &mut rng,
                )
                .await;
            }
        }
    }
}

#[inline]
async fn wait_for_low_debounced(input: &mut gpio::Input<'_>) {
    let () = input.wait_for_falling_edge().await;
    loop {
        let () = input.wait_for_low().await;
        let trigger_unless_canceled = with_timeout(DEBOUNCING_TIME, input.wait_for_high()).await;
        match trigger_unless_canceled {
            Ok(()) => { /* Canceled by a rising edge! */ }
            Err(TimeoutError) => return,
        }
    }
}

#[inline]
async fn wait_for_high_debounced(input: &mut gpio::Input<'_>) {
    let () = input.wait_for_rising_edge().await;
    loop {
        let () = input.wait_for_high().await;
        let trigger_unless_canceled = with_timeout(DEBOUNCING_TIME, input.wait_for_low()).await;
        match trigger_unless_canceled {
            Ok(()) => { /* Canceled by a falling edge! */ }
            Err(TimeoutError) => return,
        }
    }
}

#[inline]
async fn slot_is_initialized<T: flash::Instance, const FLASH_SIZE: usize>(
    flash: &mut Flash<'_, T, flash::Async, FLASH_SIZE>,
    offset: u32,
) -> bool {
    let mut buffer = [0xFF; 8];
    match flash.read(offset, &mut buffer).await {
        Ok(()) => {}
        Err(e) => {
            defmt::panic!("Error reading from a recording slot in flash: {}", e)
        }
    }
    // (buffer[0] == 0x00) && (buffer[4..] == [0x42, 0x42, 0x42, 0x42])
    buffer[0] == 0x00
}

#[inline]
async fn offset_of_nth_initialized_slot<T: flash::Instance, const FLASH_SIZE: usize>(
    flash: &mut Flash<'_, T, flash::Async, FLASH_SIZE>,
    recording_slot_offsets: &[u32; N_RECORDING_SLOTS],
    n: u8,
) -> Option<(u8, u32)> {
    let mut slots_seen = 0;
    for (i, &offset) in recording_slot_offsets.iter().enumerate() {
        if slot_is_initialized(flash, offset).await {
            if slots_seen == n {
                return Some((defmt::unwrap!(u8::try_from(i)), offset));
            }
            slots_seen += 1;
        }
    }
    None
}

#[inline]
async fn offset_of_first_empty_slot<T: flash::Instance, const FLASH_SIZE: usize>(
    flash: &mut Flash<'_, T, flash::Async, FLASH_SIZE>,
    recording_slot_offsets: &[u32; N_RECORDING_SLOTS],
) -> Option<(u8, u32)> {
    for (i, &offset) in recording_slot_offsets.iter().enumerate() {
        if !slot_is_initialized(flash, offset).await {
            return Some((defmt::unwrap!(u8::try_from(i)), offset));
        }
    }
    None
}

// #[derive(defmt::Format)]
// enum ParseDxlHeader {
//     Init,
//     FirstFf,
//     SecondFf,
//     Fd,
//     Countdown(u8),
// }

#[inline]
async fn trigger_recording<
    'usb,
    UsbDriver: embassy_usb_driver::Driver<'usb>,
    HardwareUart: uart::Instance,
    FlashInstance: flash::Instance,
    const FLASH_SIZE: usize,
>(
    selected_recording_slot: &mut u8,
    n_slots_initialized: &mut u8,
    stop_if_high: &mut gpio::Input<'_>,
    usb_rx: &mut cdc_acm::Receiver<'usb, UsbDriver>,
    usb_tx: &mut cdc_acm::Sender<'usb, UsbDriver>,
    dxl_comm: &mut Comm<'_, '_, HardwareUart>,
    flash: &mut Flash<'_, FlashInstance, flash::Async, FLASH_SIZE>,
    recording_slot_offsets: &[u32; N_RECORDING_SLOTS],
) {
    // If there's an empty recording slot, record into that;
    // if not, stay where we are, unless we switch back without input, in which case advance one.
    let maybe_theres_an_empty_slot =
        offset_of_first_empty_slot(flash, &recording_slot_offsets).await;
    let offset = match maybe_theres_an_empty_slot {
        Some((slot_index, offset)) => {
            *selected_recording_slot = slot_index;
            defmt::info!(
                "Slot #{} was empty; recording into it.",
                *selected_recording_slot
            );
            offset
        }
        None => {
            defmt::warn!(
                "No empty recording slots! OVERWRITING slot #{}.",
                *selected_recording_slot
            );
            recording_slot_offsets[usize::from(*selected_recording_slot)]
        }
    };
    let mut end = offset;

    let mut got_anything_over_usb = false;

    let do_until_switched_off = async {
        defmt::info!("Waiting for a USB connection...");
        let () = usb_rx.wait_connection().await;
        defmt::info!("    USB connected!");

        let mut first_packet_instant = None;
        let mut rx_buffer = [0; 64];
        let mut tx_buffer = [0xFF; 4 /* header bytes */ + 255 /* maximum packet length */];
        'packets: loop {
            let n = match usb_rx.read_packet(&mut rx_buffer).await {
                Ok(ok) => ok,
                Err(e) => {
                    defmt::error!("Error receiving via USB: {}", e);
                    break 'packets;
                }
            };
            let packet_from_usb = &rx_buffer[..n];
            defmt::debug!("Received `{:X}` via USB", packet_from_usb);
            if n <= 0 {
                break 'packets;
            }

            let timestamp: [u8; 3] = {
                let ms = first_packet_instant
                    .get_or_insert_with(Instant::now)
                    .elapsed()
                    .as_millis();
                if ms > 0xFF_FF_FF {
                    defmt::error!(
                        "Recording is too long (in total duration, not packets). Stopping."
                    );
                    break 'packets;
                }
                let [b1, b2, b3, ..] = ms.to_le_bytes();
                [b1, b2, b3]
            };

            let mut stream = match dxl_comm.comm(packet_from_usb).await {
                Ok(ok) => {
                    defmt::debug!("Sent `{:X}` via UART", packet_from_usb);
                    ok
                }
                Err(e) => {
                    defmt::error!("Error sending via UART: {}", e);
                    continue 'packets;
                }
            };

            // let mut parse_state = ParseDxlHeader::Init;
            'response: loop {
                let byte: u8 = match stream.next().await {
                    Ok(ok) => {
                        defmt::debug!("Received `{:X}` via UART", ok);
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
                    Ok(()) => defmt::debug!("Wrote `{:X}` via USB", packet),
                    Err(e) => defmt::error!("Error sending via USB: {}", e),
                }
                // match parse_state {
                //     ParseDxlHeader::Init => {
                //         if byte == 0xFF {
                //             parse_state = ParseDxlHeader::FirstFf
                //         }
                //     }
                //     ParseDxlHeader::FirstFf => {
                //         parse_state = if byte == 0xFF {
                //             ParseDxlHeader::SecondFf
                //         } else {
                //             ParseDxlHeader::Init
                //         }
                //     }
                //     ParseDxlHeader::SecondFf => {
                //         parse_state = if byte == 0xFD {
                //             ParseDxlHeader::Fd
                //         } else {
                //             ParseDxlHeader::Init
                //         }
                //     }
                //     ParseDxlHeader::Fd => {
                //         parse_state = if byte == 0x00 {
                //             ParseDxlHeader::Countdown(4)
                //         } else {
                //             ParseDxlHeader::Init
                //         }
                //     }
                //     ParseDxlHeader::Countdown(i) => {
                //         parse_state = if let Some(i) = i.checked_sub(1) {
                //             ParseDxlHeader::Countdown(i)
                //         } else {
                //             defmt::error!("Dynamixel packet error: {:X}", byte);
                //             ParseDxlHeader::Init
                //         }
                //     }
                // }
            }

            let data_size = {
                let big_size = packet_from_usb.len();
                let Ok(small_size) = u8::try_from(big_size) else {
                    defmt::error!(
                        "Packet too big ({} bytes). Skipping that packet, but continuing the recording.",
                        big_size
                    );
                    continue 'packets;
                };
                small_size
            };
            let store_size = 4 /* header bytes */ + u32::from(data_size);

            let start = end;
            let aligned_size = if (store_size & 7) != 0 {
                (store_size & !7) + 8
            } else {
                store_size
            };
            if end + aligned_size + 4
                > offset + defmt::unwrap!(u32::try_from(RECORDING_SLOT_SIZE_BYTES))
            {
                defmt::error!("Ran out of space in this recording spot. Stopping.");
                break 'packets;
            }
            end += aligned_size;

            tx_buffer[..3].copy_from_slice(&timestamp);
            tx_buffer[3] = data_size;
            tx_buffer[4..defmt::unwrap!(usize::try_from(store_size))]
                .copy_from_slice(packet_from_usb);
            let valid_buffer = &tx_buffer[..defmt::unwrap!(usize::try_from(store_size))];
            match flash.blocking_write(start, valid_buffer) {
                Ok(()) => defmt::debug!("Wrote {:X} to flash", valid_buffer),
                Err(e) => defmt::error!("Couldn't write {:X} to flash: {}", valid_buffer, e),
            }

            if !got_anything_over_usb
            /* yet, meaning this is the first */
            {
                defmt::info!("USB input receieved! Recording...");
                got_anything_over_usb = true;
                *n_slots_initialized += 1;
            }
        }
    };

    match select(do_until_switched_off, wait_for_high_debounced(stop_if_high)).await {
        Either::First(()) => defmt::info!("Recording finished."),
        Either::Second(()) => defmt::info!("Recording interrupted by switching off."),
    }
    if got_anything_over_usb {
        let valid_buffer = &[0, 0, 0, 0];
        match flash.blocking_write(end, valid_buffer) {
            Ok(()) => defmt::debug!("Wrote {:X} to flash", valid_buffer),
            Err(e) => defmt::error!("Couldn't write {:X} to flash: {}", valid_buffer, e),
        }
    } else {
        // Increment the slot index,
        // so we can flip the switch a few times to cycle through them:
        *selected_recording_slot += 1;
        if *selected_recording_slot >= defmt::unwrap!(u8::try_from(N_RECORDING_SLOTS)) {
            *selected_recording_slot = 0;
            defmt::debug!("Wrapping around to slot {}...", *selected_recording_slot);
        }
        let offset = recording_slot_offsets[usize::from(*selected_recording_slot)];
        defmt::info!(
            "Selected recording slot #{}, which {}.",
            *selected_recording_slot,
            if slot_is_initialized(flash, offset).await {
                "already has a saved perfomance"
            } else {
                "is empty"
            }
        );
    }
}

#[inline]
async fn trigger_playback<
    HardwareUart: uart::Instance,
    FlashInstance: flash::Instance,
    const FLASH_SIZE: usize,
    TrngInstance: trng::Instance,
>(
    selected_recording_slot: &mut u8,
    n_slots_initialized: u8,
    record_switch_ever_used: bool,
    dxl_comm: &mut Comm<'_, '_, HardwareUart>,
    flash: &mut Flash<'_, FlashInstance, flash::Async, FLASH_SIZE>,
    recording_slot_offsets: &[u32; N_RECORDING_SLOTS],
    rng: &mut Trng<'_, TrngInstance>,
) {
    if n_slots_initialized == 0 {
        defmt::warn!("No slots initialized; skipping playback.");
        return;
    }

    let mut offset = recording_slot_offsets[usize::from(*selected_recording_slot)];
    if !record_switch_ever_used {
        'select_another_slot: loop {
            let index_among_initialized_only = rng.blocking_next_u32() as u8 % n_slots_initialized;
            let opt = offset_of_nth_initialized_slot(
                flash,
                recording_slot_offsets,
                index_among_initialized_only,
            )
            .await;
            let (slot_index, slot_offset) = defmt::unwrap!(opt);
            *selected_recording_slot = slot_index;
            offset = slot_offset;
            if slot_is_initialized(flash, offset).await {
                break 'select_another_slot;
            }
        }
    } else if !slot_is_initialized(flash, offset).await {
        defmt::error!(
            "This slot (#{}) is empty, so it can't be played back; skipping.",
            *selected_recording_slot
        );
        return;
    }

    defmt::info!(
        "Playing the recording found in slot #{}...",
        *selected_recording_slot
    );

    let mut buffer = [0xFF; 4 /* header bytes */ + 255 /* maximum packet length */];
    let start_instant = Instant::now();
    'packets: loop {
        let start = offset;
        'flash_read: loop {
            match flash.read(start, &mut buffer[..8]).await {
                Ok(()) => break 'flash_read,
                Err(e) => {
                    defmt::error!("Error reading from a recording slot in flash: {}", e)
                }
            }
        }

        let [b1, b2, b3, b4, ..] = buffer;
        let header = u32::from_le_bytes([b1, b2, b3, b4]);
        let data_size = defmt::unwrap!(u8::try_from(header >> 24));
        if data_size == 0 {
            defmt::info!("Reached the end of this recording.");
            return;
        }
        let aligned_size = {
            let size = 4 + u32::from(data_size);
            if (size & 7) == 0 {
                size
            } else {
                (size & !7) + 8
            }
        };
        offset += aligned_size;

        'flash_read: loop {
            match flash
                .read(
                    start + 8,
                    &mut buffer[8..defmt::unwrap!(usize::try_from(aligned_size))],
                )
                .await
            {
                Ok(()) => break 'flash_read,
                Err(e) => {
                    defmt::error!("Error reading from a recording slot in flash: {}", e)
                }
            }
        }

        let packet_to_dxl = &buffer[4..(4 + usize::from(data_size))];

        let ms = header & 0xFF_FF_FF;
        let () = Timer::at(start_instant + Duration::from_millis(u64::from(ms))).await;

        let mut stream = match dxl_comm.comm(packet_to_dxl).await {
            Ok(ok) => {
                defmt::debug!("Sent `{:X}` via UART", packet_to_dxl);
                ok
            }
            Err(e) => {
                defmt::error!("Error sending via UART: {}", e);
                continue 'packets;
            }
        };

        'response: loop {
            let byte: u8 = match stream.next().await {
                Ok(ok) => {
                    defmt::debug!("Received `{:X}` via UART", ok);
                    ok
                }
                Err(RecvError::TimedOut(_)) => break 'response,
                Err(e) => {
                    defmt::error!("Error receiving via UART: {}", e);
                    continue 'response;
                }
            };
        }
    }
}
