#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]

use {
    core::num::NonZeroU32,
    defmt_rtt as _,
    dxl_driver::{bus::Bus, comm::Comm, mutex::Mutex as _},
    dxl_packet::recv::Read,
    dxl_rp::serial,
    embassy_executor::Spawner,
    embassy_rp::{
        bind_interrupts,
        peripherals::{UART1, USB},
        uart, usb,
    },
    embassy_time::{Duration, Instant, Timer},
    panic_probe as _,
    static_cell::StaticCell,
};

bind_interrupts!(struct Irqs {
    UART1_IRQ => uart::InterruptHandler<UART1>;
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
});

const BAUD_RATES: &[u32] = &[
    9_600, 57_600, 115_200, 1_000_000, 2_000_000, 3_000_000, 4_000_000, 4_500_000,
];

/*
const SAFE_PROFILE_VELOCITY: u32 = 16;
const SAFE_PROFILE_ACCELERATION: u32 = 1;
*/

const SAFE_CURRENT: i16 = 3;

const RESISTANCE_STARTUP: Duration = Duration::from_secs(1);
const RESISTANCE_TIMEOUT: Duration = Duration::from_millis(100);

const PROFILE_VELOCITY: u32 = 0xFFF;
const PROFILE_ACCELERATION: u32 = 4;

const LOG_WIGGLE_ROOM: u8 = 5;

const POSITION_TOLERANCE: i16 = 8;

#[inline]
async fn pos<C: Comm>(bus: &mut Bus<C>, id: u8) -> i16 {
    loop {
        match bus.read_present_position(id).await {
            Ok(Read { bytes }) => {
                let pos = i32::from_le_bytes(bytes);
                let pos = match i16::try_from(pos) {
                    Ok(ok) => ok,
                    Err(e) => {
                        defmt::error!("Invalid position returned from ID {}: {} ({})", id, pos, e,);
                        log::error!("Invalid position returned from ID {id}: {pos}",);
                        continue;
                    }
                };
                return pos;
            }
            Err(e) => {
                defmt::error!(
                    "ERROR: ID {} has been working but could not read its position: {}",
                    id,
                    e
                );
                log::error!("ERROR: ID {id} has been working but could not read its position",);
            }
        }
    }
}

#[inline]
async fn position_when_stopped<C: Comm>(bus: &mut Bus<C>, id: u8, increasing: bool) -> i16 {
    'goal_current: loop {
        match bus
            .write_goal_current(
                id,
                (if increasing {
                    SAFE_CURRENT
                } else {
                    -SAFE_CURRENT
                })
                .to_le_bytes(),
            )
            .await
        {
            Ok(()) => break 'goal_current,
            Err(e) => {
                defmt::error!(
                    "ERROR: ID {} has been working but could not write its position: {}",
                    id,
                    e
                );
                log::error!("ERROR: ID {id} has been working but could not write its position");
            }
        }
    }

    let () = Timer::after(RESISTANCE_STARTUP).await;

    let mut extreme = pos(bus, id).await;
    let mut last_extreme_time = Instant::now();

    loop {
        let p = pos(bus, id).await;

        defmt::info!("{}", p);
        log::info!("{p}");

        let new_extreme = if increasing { p > extreme } else { p < extreme };
        if new_extreme {
            last_extreme_time = Instant::now();
            extreme = p;
        } else {
            if last_extreme_time.elapsed() >= RESISTANCE_TIMEOUT {
                return p;
            }
        }

        if increasing {
            if p > const { 0xFFF - POSITION_TOLERANCE } {
                return 0xFFF;
            }
        } else {
            if p < POSITION_TOLERANCE {
                return 0;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    static BAUDS: StaticCell<[Option<NonZeroU32>; dxl_packet::N_IDS as usize]> = StaticCell::new();
    static STOPS: StaticCell<[(i16, i16); dxl_packet::N_IDS as usize]> = StaticCell::new();
    static DIRECTIONS: StaticCell<[bool; dxl_packet::N_IDS as usize]> = StaticCell::new();

    let p = embassy_rp::init(Default::default());

    {
        // USB background task:
        #[embassy_executor::task]
        pub async fn task(driver: usb::Driver<'static, USB>) {
            embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
        }
        match spawner.spawn(task(usb::Driver::new(p.USB, Irqs))) {
            Ok(()) => {
                let () = defmt::info!("Spawned USB task");
                let () = log::info!("Spawned USB task");
            }
            Err(e) => {
                let () = log::error!("Error spawning USB task: {e}");
                let () = defmt::panic!("Error spawning USB task: {}", e);
            }
        }
    }

    let () = Timer::after(Duration::from_secs(3)).await;

    let dxl_bus_mutex = dxl_rp::bus(
        const { BAUD_RATES[0] },
        p.PIN_7,
        p.UART1,
        p.PIN_8,
        p.PIN_9,
        Irqs,
        p.DMA_CH1,
        p.DMA_CH2,
    );
    let mut bus = match dxl_bus_mutex.lock().await {
        Ok(ok) => ok,
        Err(e) => {
            log::error!("Couldn't acquire the Dynamixel bus mutex lock");
            defmt::panic!("Couldn't acquire the Dynamixel bus mutex lock: {}", e);
        }
    };

    let baud_rates: &mut _ = BAUDS.init([None; dxl_packet::N_IDS as usize]);
    for &baud in BAUD_RATES {
        defmt::info!("");
        log::info!("");
        defmt::info!("Scanning at {} baud...", baud);
        log::info!("Scanning at {baud} baud...");

        let () = bus.set_baud(baud);

        for id in 0..dxl_packet::N_IDS {
            log::debug!("Pinging {}...", id);
            defmt::debug!("Pinging {}...", id);
            match bus.write_torque_enable(id, [0]).await {
                Ok(()) => {
                    defmt::info!("    --> ID {} responded!", id);
                    log::info!("    --> ID {id} responded!");
                    *unsafe { baud_rates.get_unchecked_mut(id as usize) } = NonZeroU32::new(baud);
                }
                Err(dxl_driver::bus::Error::Io(dxl_driver::IoError::Recv(
                    serial::RecvError::TimedOut(_),
                ))) => {
                    log::debug!("    --> timed out");
                    defmt::debug!("    --> timed out");
                }
                Err(e) => {
                    defmt::info!("    --> ID {} responded! ERROR: {}", id, e);
                    log::info!("    --> ID {id} responded! ERROR");
                }
            }
        }
    }

    defmt::info!("Checking stops for all Dynamixels that responded...");
    log::info!("Checking stops for all Dynamixels that responded...");
    let stops: &mut _ = STOPS.init([(i16::MAX, 0); dxl_packet::N_IDS as usize]);
    for id in 0..dxl_packet::N_IDS {
        let Some(baud) = (*unsafe { baud_rates.get_unchecked(id as usize) }) else {
            continue;
        };

        defmt::info!("");
        log::info!("");
        defmt::info!("Checking stops for ID {} ({} baud)...", id, baud);
        log::info!("Checking stops for ID {id} ({baud} baud)...");

        let () = bus.set_baud(baud.get());

        'reboot: loop {
            match bus.reboot(id).await {
                Ok(()) => loop {
                    match bus.write_torque_enable(id, [0]).await {
                        Ok(()) => break 'reboot,
                        Err(dxl_driver::bus::Error::Io(dxl_driver::IoError::Recv(
                            serial::RecvError::TimedOut(_),
                        ))) => continue,
                        Err(e) => {
                            defmt::error!(
                                "ERROR: ID {} has been working but could not disable torque: {}",
                                id,
                                e
                            );
                            log::error!(
                                "ERROR: ID {id} has been working but could not disable torque",
                            );
                        }
                    }
                },
                Err(e) => {
                    defmt::error!(
                        "ERROR: ID {} has been working but could not reboot: {}",
                        id,
                        e
                    );
                    log::error!("ERROR: ID {id} has been working but could not reboot");
                }
            }
        }

        'operating_mode: loop {
            // Use *current-control mode*, not position control:
            match bus.write_operating_mode(id, [0]).await {
                Ok(()) => break 'operating_mode,
                Err(e) => {
                    defmt::error!(
                        "ERROR: ID {} has been working but could not set its operating mode: {}",
                        id,
                        e
                    );
                    log::error!(
                        "ERROR: ID {id} has been working but could not set its operating mode",
                    );
                }
            }
        }

        /*
        'velocity: loop {
        match bus
            .write_profile_velocity(id, SAFE_PROFILE_VELOCITY.to_le_bytes())
            .await
        {
            Ok(()) => break 'velocity,
            Err(e) => {
                defmt::error!(
                    "ERROR: ID {} has been working but could not set its velocity profile: {}",
                    id,
                    e
                );
                log::error!(
                    "ERROR: ID {id} has been working but could not set its velocity profile",
                );
            }
        }
        }

        'acceleration: loop {
        match bus
            .write_profile_acceleration(id, SAFE_PROFILE_ACCELERATION.to_le_bytes())
            .await
        {
            Ok(()) => break 'acceleration,
            Err(e) => {
                defmt::error!(
                    "ERROR: ID {} has been working but could not set its acceleration profile: {}",
                    id,
                    e
                );
                log::error!(
                    "ERROR: ID {id} has been working but could not set its acceleration profile",
                );
            }
        }
        }
        */

        'torque: loop {
            match bus.write_torque_enable(id, [1]).await {
                Ok(()) => break 'torque,
                Err(e) => {
                    defmt::error!(
                        "ERROR: ID {} has been working but could not enable torque: {}",
                        id,
                        e
                    );
                    log::error!("ERROR: ID {id} has been working but could not enable torque");
                }
            }
        }

        let min = position_when_stopped(&mut bus, id, false).await;
        defmt::info!("ID {} stopped decreasing at {}", id, min);
        log::info!("ID {id} stopped decreasing at {min}");

        'torque: loop {
            match bus.write_torque_enable(id, [0]).await {
                Ok(()) => break 'torque,
                Err(e) => {
                    defmt::error!(
                        "ERROR: ID {} has been working but could not disable torque: {}",
                        id,
                        e
                    );
                    log::error!("ERROR: ID {id} has been working but could not disable torque");
                }
            }
        }
        let () = Timer::after(Duration::from_millis(500)).await;
        'torque: loop {
            match bus.write_torque_enable(id, [1]).await {
                Ok(()) => break 'torque,
                Err(e) => {
                    defmt::error!(
                        "ERROR: ID {} has been working but could not enable torque: {}",
                        id,
                        e
                    );
                    log::error!("ERROR: ID {id} has been working but could not enable torque");
                }
            }
        }

        let max = position_when_stopped(&mut bus, id, true).await;
        defmt::info!("ID {} stopped increasing at {}", id, max);
        log::info!("ID {id} stopped increasing at {max}");

        let stop: &mut _ = unsafe { stops.get_unchecked_mut(id as usize) };
        *stop = (
            min - (min >> LOG_WIGGLE_ROOM) + (max >> LOG_WIGGLE_ROOM),
            max - (max >> LOG_WIGGLE_ROOM) + (min >> LOG_WIGGLE_ROOM),
        );
        defmt::info!("ID {} stops set to {}", id, stop);
        log::info!("ID {id} stops set to {stop:?}");

        'torque: loop {
            match bus.write_torque_enable(id, [0]).await {
                Ok(()) => break 'torque,
                Err(e) => {
                    defmt::error!(
                        "ERROR: ID {} has been working but could not disable torque: {}",
                        id,
                        e
                    );
                    log::error!("ERROR: ID {id} has been working but could not disable torque");
                }
            }
        }
    }
    defmt::info!("");
    log::info!("");
    defmt::info!("Checked stops");
    log::info!("Checked stops");

    'ids: for id in 0..dxl_packet::N_IDS {
        let Some(baud) = (*unsafe { baud_rates.get_unchecked(id as usize) }) else {
            continue 'ids;
        };
        let () = bus.set_baud(baud.get());
        let (min, _) = *unsafe { stops.get_unchecked(id as usize) };

        // Back to position control mode:
        'operating_mode: loop {
            match bus.write_operating_mode(id, [3]).await {
                Ok(()) => break 'operating_mode,
                Err(e) => {
                    defmt::error!(
                        "ERROR: ID {} has been working but could not set its operating mode: {}",
                        id,
                        e
                    );
                    log::error!(
                        "ERROR: ID {id} has been working but could not set its operating mode",
                    );
                }
            }
        }

        'velocity: loop {
            match bus
                .write_profile_velocity(id, PROFILE_VELOCITY.to_le_bytes())
                .await
            {
                Ok(()) => break 'velocity,
                Err(e) => {
                    defmt::error!(
                        "Error writing velocity profile to {} for ID {}: {}",
                        PROFILE_VELOCITY,
                        id,
                        e
                    );
                    log::error!("Error writing velocity profile to {PROFILE_VELOCITY} for ID {id}");
                }
            }
        }
        'acceleration: loop {
            match bus
                .write_profile_acceleration(id, PROFILE_ACCELERATION.to_le_bytes())
                .await
            {
                Ok(()) => break 'acceleration,
                Err(e) => {
                    defmt::error!(
                        "Error writing acceleration profile to {} for ID {}: {}",
                        PROFILE_ACCELERATION,
                        id,
                        e
                    );
                    log::error!(
                        "Error writing acceleration profile to {PROFILE_ACCELERATION} for ID {id}"
                    );
                }
            }
        }
        'torque_on: loop {
            match bus.write_torque_enable(id, [1]).await {
                Ok(()) => break 'torque_on,
                Err(e) => {
                    defmt::error!("Error enabling torque for ID {}: {}", id, e);
                    log::error!("Error enabling torque for ID {id}");
                }
            }
        }
        'send: loop {
            match bus
                .write_goal_position(id, i32::from(min).to_le_bytes())
                .await
            {
                Ok(()) => break 'send,
                Err(e) => {
                    defmt::error!("Error sending ID {} to {}: {}", id, min, e);
                    log::error!("Error sending ID {id} to {min}");
                }
            }
        }
    }

    let directions: &mut _ = DIRECTIONS.init([false; dxl_packet::N_IDS as usize]);

    loop {
        'ids: for id in 0..dxl_packet::N_IDS {
            let Some(baud) = (*unsafe { baud_rates.get_unchecked(id as usize) }) else {
                continue 'ids;
            };
            let () = bus.set_baud(baud.get());
            let (min, max) = *unsafe { stops.get_unchecked(id as usize) };
            'pos: loop {
                match bus.read_present_position(id).await {
                    Ok(Read { bytes }) => {
                        let pos = i32::from_le_bytes(bytes);
                        let direction: &mut _ =
                            unsafe { directions.get_unchecked_mut(id as usize) };
                        let error = ((if *direction { max } else { min }) as i32 - pos).abs();
                        if error < i32::from(POSITION_TOLERANCE) {
                            *direction = !*direction;
                            let pos = if *direction { max } else { min };
                            'send: loop {
                                match bus
                                    .write_goal_position(id, i32::from(pos).to_le_bytes())
                                    .await
                                {
                                    Ok(()) => break 'send,
                                    Err(e) => {
                                        defmt::error!("Error sending ID {} to {}: {}", id, pos, e);
                                        log::error!("Error sending ID {id} to {pos}");
                                    }
                                }
                            }
                        }
                        break 'pos;
                    }
                    Err(e) => {
                        defmt::error!("Error reading ID {}'s position: {}", id, e);
                        log::error!("Error reading ID {id}'s position");
                    }
                }
            }
        }
    }
}
