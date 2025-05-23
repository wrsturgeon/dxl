use {
    crate::{comm::Comm, mutex::Mutex},
    ::dxl_packet::{New, packet::recv::PersistentConfig},
    paste::paste,
};

pub enum Error<C: Comm, Output> {
    Io(crate::IoError<C>),
    Packet(::dxl_packet::packet::recv::PersistentError<Output>),
}

impl<C: Comm, Output> defmt::Format for Error<C, Output> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::Io(ref e) => defmt::Format::format(e, f),
            Self::Packet(ref e) => defmt::write!(f, "Valid packet describing a real error: {}", e),
        }
    }
}

#[derive(defmt::Format)]
#[cfg(debug_assertions)]
pub enum IdError {
    InvalidId { id: u8 },
    AlreadyInUse { id: u8 },
}

pub struct Bus<C: Comm> {
    comm: C,
    #[cfg(debug_assertions)]
    used_ids: [bool; dxl_packet::N_IDS as usize],
}

macro_rules! instruction_method {
    ($id:ident) => {
        #[inline]
        pub async fn $id(
            &mut self,
            id: u8,
        ) -> Result<
            paste! { ::dxl_packet::recv::[< $id:camel >] },
            Error<
                C,
                <paste! { ::dxl_packet::send::[< $id:camel >] } as ::dxl_packet::Instruction>::Recv,
            >,
        > {
            self.comm::<paste! { ::dxl_packet::send:: [< $id:camel >] }>(
                id,
                paste! { ::dxl_packet::send:: [< $id:camel >] ::new() },
            )
            .await
        }
    };
}

macro_rules! control_table_methods {
    ($id:ident) => {
        paste! {
            #[inline]
            pub async fn [< read_ $id:snake >](
                &mut self,
                id: u8,
            ) -> Result<::dxl_packet::recv::Read<{ <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::BYTES as usize }>, Error<C, ::dxl_packet::recv::Read<{ <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::BYTES as usize }>>> {
                self.comm::<::dxl_packet::send::Read<::dxl_packet::control_table::$id>>(
                    id,
                    ::dxl_packet::send::Read::<::dxl_packet::control_table::$id>::new()
                )
                .await
            }

            #[inline]
            pub async fn [< write_ $id:snake >](
                &mut self,
                id: u8,
                bytes: [u8; <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::BYTES as usize]
            ) -> Result<::dxl_packet::recv::Write, Error<C, ::dxl_packet::recv::Write>> {
                self.comm::<::dxl_packet::send::Write<::dxl_packet::control_table::$id>>(
                    id,
                    ::dxl_packet::send::Write::<::dxl_packet::control_table::$id>::new(bytes)
                )
                .await
            }

            #[inline]
            pub async fn [< reg_write_ $id:snake >](
                &mut self,
                id: u8,
                bytes: [u8; <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::BYTES as usize]
            ) -> Result<::dxl_packet::recv::RegWrite, Error<C, ::dxl_packet::recv::RegWrite>> {
                self.comm::<::dxl_packet::send::RegWrite<::dxl_packet::control_table::$id>>(
                    id,
                    ::dxl_packet::send::RegWrite::<::dxl_packet::control_table::$id>::new(bytes)
                )
                .await
            }
        }
    };
}

impl<C: Comm> Bus<C> {
    #[inline(always)]
    pub const fn new(comm: C) -> Self {
        Self {
            comm,
            #[cfg(debug_assertions)]
            used_ids: [false; dxl_packet::N_IDS as usize],
        }
    }

    #[inline]
    #[cfg(debug_assertions)]
    pub fn check_duplicate_id(&mut self, id: u8) -> Result<(), IdError> {
        let Some(state) = self.used_ids.get_mut(id as usize) else {
            return Err(IdError::InvalidId { id });
        };
        if *state {
            return Err(IdError::AlreadyInUse { id });
        }
        *state = true;
        Ok(())
    }

    #[inline(always)]
    pub fn set_baud(&mut self, baud: u32) {
        self.comm.set_baud(baud)
    }

    #[inline]
    pub async fn comm<Insn: ::dxl_packet::Instruction>(
        &mut self,
        id: u8,
        parameters: Insn,
    ) -> Result<Insn::Recv, Error<C, Insn::Recv>>
    where
        [(); { Insn::BYTE } as usize]:,
        [(); { core::mem::size_of::<Insn>() as u16 + 3 } as usize]:,
    {
        let mut stream = {
            let packet = ::dxl_packet::packet::new::<Insn>(id, parameters);
            defmt::debug!("Packet: {}", packet.as_buffer());
            self.comm
                .comm(packet.as_buffer())
                .await
                .map_err(crate::IoError::Send)
                .map_err(Error::Io)?
        };
        let mut state: ::dxl_packet::packet::recv::Persistent<Insn> =
            <::dxl_packet::packet::recv::Persistent<Insn> as New>::new(PersistentConfig {
                expected_id: id,
            });
        loop {
            let byte: u8 = ::dxl_packet::stream::Stream::next(&mut stream)
                .await
                .map_err(|e| Error::Io(crate::IoError::Recv(e)))?;
            state = match ::dxl_packet::parse::State::push(state, byte).map_err(Error::Packet)? {
                ::dxl_packet::parse::Status::Complete(complete) => return Ok(complete),
                ::dxl_packet::parse::Status::Incomplete((updated, ())) => updated,
            };
            let () = C::yield_to_other_tasks().await;
        }
    }

    instruction_method!(ping);
    instruction_method!(action);
    instruction_method!(factory_reset);
    instruction_method!(reboot);

    control_table_methods!(ModelNumber);
    control_table_methods!(ModelInformation);
    control_table_methods!(FirmwareVersion);
    control_table_methods!(Id);
    control_table_methods!(BaudRate);
    control_table_methods!(ReturnDelayTime);
    control_table_methods!(DriveMode);
    control_table_methods!(OperatingMode);
    control_table_methods!(SecondaryId);
    control_table_methods!(ProtocolType);
    control_table_methods!(HomingOffset);
    control_table_methods!(MovingThreshold);
    control_table_methods!(TemperatureLimit);
    control_table_methods!(MaxVoltageLimit);
    control_table_methods!(MinVoltageLimit);
    control_table_methods!(PwmLimit);
    control_table_methods!(CurrentLimit);
    control_table_methods!(VelocityLimit);
    control_table_methods!(MaxPositionLimit);
    control_table_methods!(MinPositionLimit);
    control_table_methods!(StartupConfiguration);
    control_table_methods!(PwmSlope);
    control_table_methods!(Shutdown);
    control_table_methods!(TorqueEnable);
    control_table_methods!(Led);
    control_table_methods!(StatusReturnLevel);
    control_table_methods!(RegisteredInstruction);
    control_table_methods!(HardwareErrorStatus);
    control_table_methods!(VelocityIGain);
    control_table_methods!(VelocityPGain);
    control_table_methods!(PositionDGain);
    control_table_methods!(PositionIGain);
    control_table_methods!(PositionPGain);
    control_table_methods!(Feedforward2ndGain);
    control_table_methods!(Feedforward1stGain);
    control_table_methods!(BusWatchdog);
    control_table_methods!(GoalPwm);
    control_table_methods!(GoalCurrent);
    control_table_methods!(GoalVelocity);
    control_table_methods!(ProfileAcceleration);
    control_table_methods!(ProfileVelocity);
    control_table_methods!(GoalPosition);
    control_table_methods!(RealtimeTick);
    control_table_methods!(Moving);
    control_table_methods!(MovingStatus);
    control_table_methods!(PresentPwm);
    control_table_methods!(PresentCurrent);
    control_table_methods!(PresentVelocity);
    control_table_methods!(PresentPosition);
    control_table_methods!(VelocityTrajectory);
    control_table_methods!(PositionTrajectory);
    control_table_methods!(PresentInputVoltage);
    control_table_methods!(PresentTemperature);
    control_table_methods!(BackupReady);
}

const SCAN_BAUD: &[u32] = &[
    9_600, 57_600, 115_200, 1_000_000, 2_000_000, 3_000_000, 4_000_000, 4_500_000,
];

pub struct Scan<'bus, C: Comm, M: Mutex<Item = Bus<C>>> {
    bus: &'bus M,
    baud_index: usize,
    id: u8,
}

pub struct ScanFound<'bus, C: Comm, M: Mutex<Item = Bus<C>>> {
    pub baud: u32,
    pub id: u8,
    pub model_number: u16,
    pub firmware_version: u8,
    pub continue_scanning: Scan<'bus, C, M>,
}

impl<'bus, C: Comm, M: Mutex<Item = Bus<C>>> Scan<'bus, C, M> {
    #[inline(always)]
    pub fn start(bus: &'bus M) -> Self {
        Self {
            bus,
            baud_index: usize::MAX,
            id: dxl_packet::MAX_ID,
        }
    }

    #[inline]
    pub async fn next(mut self) -> Option<ScanFound<'bus, C, M>> {
        loop {
            if self.id == dxl_packet::MAX_ID {
                self.baud_index = self.baud_index.wrapping_add(1);
                let Some(&baud) = SCAN_BAUD.get(self.baud_index) else {
                    return None;
                };
                defmt::info!("");
                defmt::info!("Scanning at {} baud:", baud);
                let mut bus = self.bus.lock_persistent().await;
                let () = bus.set_baud(baud);
                self.id = dxl_packet::MIN_ID;
            }

            'ignore_unrelated_errors: loop {
                let response = {
                    let mut bus = self.bus.lock_persistent().await;
                    bus.ping(self.id).await
                };
                match response {
                    Ok(dxl_packet::recv::Ping {
                        model_number,
                        firmware_version,
                    }) => {
                        return Some(ScanFound {
                            baud: SCAN_BAUD[self.baud_index],
                            id: self.id,
                            model_number,
                            firmware_version,
                            continue_scanning: self,
                        });
                    }
                    Err(Error::Io(crate::IoError::Recv(_))) => break 'ignore_unrelated_errors,
                    Err(e) => defmt::warn!(
                        "Unrelated error while scanning ID {} at {} baud: {}; trying again...",
                        self.id,
                        SCAN_BAUD[self.baud_index],
                        e,
                    ),
                }
            }
        }
    }

    #[inline]
    pub async fn next_looping(mut self) -> ScanFound<'bus, C, M> {
        let bus = self.bus;
        loop {
            match self.next().await {
                Some(found) => return found,
                None => self = Self::start(bus),
            }
        }
    }
}
