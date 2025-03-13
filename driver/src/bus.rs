use {crate::comm::Comm, paste::paste};

#[derive(defmt::Format)]
pub enum Error<C: Comm, Output> {
    Send(<C as Comm>::SendError),
    Recv(<C as Comm>::RecvError),
    Parse(::dxl_packet::packet::recv::PersistentError<Output>),
}

impl<C: Comm, X> Error<C, X> {
    #[inline]
    pub fn map<Y, F: FnOnce(X) -> Y>(self, f: F) -> Error<C, Y> {
        match self {
            Self::Send(e) => Error::Send(e),
            Self::Recv(e) => Error::Recv(e),
            Self::Parse(e) => Error::Parse(e.map(f)),
        }
    }
}

/*
impl<C: Comm, Output> defmt::Format for Error<C, Output> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Send(ref e) => write!(f, "Error sending serial communication: {e}"),
            Self::Recv(ref e) => write!(f, "Error receiving serial communication: {e}"),
            Self::Parse(ref e) => write!(f, "Error parsing received serial communication: {e}"),
        }
    }
}
*/

#[derive(defmt::Format)]
#[cfg(debug_assertions)]
pub enum IdError {
    InvalidId { id: u8 },
    AlreadyInUse { id: u8 },
}

/*
#[cfg(debug_assertions)]
impl defmt::Format for IdError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidId { id } => write!(f, "Invalid Dynamixel ID: {id}"),
            Self::AlreadyInUse { id } => write!(f, "Dynamixel ID already in use: {id}"),
        }
    }
}
*/

pub struct Bus<C: Comm> {
    comm: C,
    #[cfg(debug_assertions)]
    used_ids: [bool; 252],
}

macro_rules! instruction_method {
    ($id:ident) => {
        #[inline]
        pub async fn $id<const ID: u8>(
            &mut self,
        ) -> Result<
            paste! { ::dxl_packet::recv::[< $id:camel >] },
            Error<
                C,
                <paste! { ::dxl_packet::send::[< $id:camel >] } as ::dxl_packet::Instruction>::Recv,
            >,
        > {
            self.comm::<ID, paste! { ::dxl_packet::send:: [< $id:camel >] }>({
                let payload = paste! { ::dxl_packet::send:: [< $id:camel >] ::new() };
                // defmt::debug!("Sending {:X} to DXL {}", payload, ID);
                payload
            })
            .await
        }
    };
}

macro_rules! control_table_methods {
    ($id:ident) => {
        paste! {
            #[inline]
            pub async fn [< read_ $id:snake >]<const ID: u8>(
                &mut self,
            ) -> Result<::dxl_packet::recv::Read<::dxl_packet::control_table::$id>, Error<C, ::dxl_packet::recv::Read<::dxl_packet::control_table::$id>>> {
                self.comm::<ID, ::dxl_packet::send::Read<::dxl_packet::control_table::$id>>({
                    let payload = ::dxl_packet::send::Read::<::dxl_packet::control_table::$id>::new();
                    // defmt::debug!("Reading {:X} (\"{}\") from DXL {}", payload, <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::DESCRIPTION, ID);
                    payload
                })
                .await
            }

            #[inline]
            pub async fn [< write_ $id:snake >]<const ID: u8>(
                &mut self,
                bytes: [u8; <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::BYTES as usize]
            ) -> Result<::dxl_packet::recv::Write, Error<C, ::dxl_packet::recv::Write>> {
                self.comm::<ID, ::dxl_packet::send::Write<::dxl_packet::control_table::$id>>({
                    let payload = ::dxl_packet::send::Write::<::dxl_packet::control_table::$id>::new(bytes);
                    // defmt::debug!("Writing {:X} to DXL {}", payload, ID);
                    payload
                })
                .await
            }

            #[inline]
            pub async fn [< reg_write_ $id:snake >]<const ID: u8>(
                &mut self,
                bytes: [u8; <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::BYTES as usize]
            ) -> Result<::dxl_packet::recv::RegWrite, Error<C, ::dxl_packet::recv::RegWrite>> {
                self.comm::<ID, ::dxl_packet::send::RegWrite<::dxl_packet::control_table::$id>>({
                    let payload = ::dxl_packet::send::RegWrite::<::dxl_packet::control_table::$id>::new(bytes);
                    // defmt::debug!("Register-writing {:X} to DXL {}", payload, ID);
                    payload
                })
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
            used_ids: [false; 252],
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

    #[inline]
    pub async fn comm<const ID: u8, Insn: ::dxl_packet::Instruction>(
        &mut self,
        parameters: Insn,
    ) -> Result<Insn::Recv, Error<C, Insn::Recv>>
    where
        [(); { Insn::BYTE } as usize]:,
        [(); { core::mem::size_of::<Insn>() as u16 + 3 } as usize]:,
    {
        let mut stream = {
            let packet = ::dxl_packet::packet::new::<Insn, ID>(parameters);
            self.comm
                .comm(packet.as_buffer())
                .await
                .map_err(Error::Send)?
        };
        let mut state: ::dxl_packet::packet::recv::Persistent::<Insn, ID> = <::dxl_packet::packet::recv::Persistent::<Insn, ID> as ::dxl_packet::parse::State<_>>::INIT;
        loop {
            let byte: u8 = ::dxl_packet::stream::Stream::next(&mut stream)
                .await
                .map_err(Error::Recv)?;
            // defmt::debug!("Read `x{=u8:X}` over serial", byte);
            state = match ::dxl_packet::parse::State::push(state, byte).map_err(Error::Parse)? {
                ::dxl_packet::parse::Status::Complete(complete) => {
                    // defmt::debug!("Successfully decoded a packet: {:X}", complete);
                    return Ok(complete);
                }
                ::dxl_packet::parse::Status::Incomplete((updated, ())) => {
                    // defmt::debug!("Updating parser: {}", updated);
                    updated
                }
            };
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
