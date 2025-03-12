use {
    crate::{bus::Bus, comm::Comm, mutex::Mutex},
    paste::paste,
};

macro_rules! instruction_method {
    ($id:ident) => {
        #[inline(always)]
        pub async fn $id(
            &self,
        ) -> Result<
            paste! { ::dxl_packet::recv::[< $id:camel >] },
            crate::Error<
                C,
                M,
                paste! { <::dxl_packet::send::[< $id:camel >] as ::dxl_packet::Instruction>::Recv },
            >,
        > {
            self.0
                .lock()
                .await
                .map_err(crate::Error::Mutex)?
                .$id::<ID>()
                .await
                .map_err(crate::Error::Bus)
        }
    };
}

macro_rules! control_table_methods {
    ($id:ident, $bits:expr) => {
        paste! {
            #[inline]
            pub async fn [< read_ $id:snake >](
                &self,
            ) -> Result<::dxl_packet::recv::Read<::dxl_packet::control_table::$id>, crate::Error<C, M, paste! { ::dxl_packet::recv::Read::<::dxl_packet::control_table::[< $id >]> }>> {
                self.0.lock().await.map_err(crate::Error::Mutex)?.[< read_ $id:snake >]::<ID>().await.map_err(crate::Error::Bus)
            }

            #[inline]
            pub async fn [< write_ $id:snake >](
                &self, value: [< u $bits >],
            ) -> Result<::dxl_packet::recv::Write, crate::Error<C, M, ::dxl_packet::recv::Write>> {
                self.0.lock().await.map_err(crate::Error::Mutex)?.[< write_ $id:snake >]::<ID>(value.to_le_bytes()).await.map_err(crate::Error::Bus)
            }

            #[inline]
            pub async fn [< reg_write_ $id:snake >](
                &self, value: [< u $bits >],
            ) -> Result<::dxl_packet::recv::RegWrite, crate::Error<C, M, ::dxl_packet::recv::RegWrite>> {
                self.0.lock().await.map_err(crate::Error::Mutex)?.[< reg_write_ $id:snake >]::<ID>(value.to_le_bytes()).await.map_err(crate::Error::Bus)
            }
        }
    };
}

pub enum InitError<C: Comm, M: Mutex> {
    TorqueOff {
        id: u8,
        error: crate::Error<C, M, ::dxl_packet::recv::Write>,
    },
    VelocityProfile {
        id: u8,
        error: crate::Error<C, M, ::dxl_packet::recv::Write>,
    },
    AccelerationProfile {
        id: u8,
        error: crate::Error<C, M, ::dxl_packet::recv::Write>,
    },
    TorqueOn {
        id: u8,
        error: crate::Error<C, M, ::dxl_packet::recv::Write>,
    },
    #[cfg(debug_assertions)]
    Id(crate::bus::IdError),
    #[cfg(debug_assertions)]
    Mutex(M::Error),
}

impl<C: Comm, M: Mutex> defmt::Format for InitError<C, M> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::TorqueOff { id, ref error } => defmt::write!(
                f,
                "Error disabling torque while initializing Dynamixel ID {}: {}",
                id,
                error
            ),
            Self::VelocityProfile { id, ref error } => defmt::write!(
                f,
                "Error configuring velocity profile while initializing Dynamixel ID {}: {}",
                id,
                error
            ),
            Self::AccelerationProfile { id, ref error } => defmt::write!(
                f,
                "Error configuring acceleration profile while initializing Dynamixel ID {}: {}",
                id,
                error
            ),
            Self::TorqueOn { id, ref error } => defmt::write!(
                f,
                "Error enabling torque while initializing Dynamixel ID {}: {}",
                id,
                error
            ),
            #[cfg(debug_assertions)]
            Self::Id(ref e) => defmt::Format::format(e, f),
            #[cfg(debug_assertions)]
            Self::Mutex(ref e) => defmt::Format::format(e, f),
        }
    }
}

/*
impl<C: Comm, M: Mutex> defmt::Format for InitError<C, M> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::TorqueOff { id, ref error } => write!(f, "Error disabling torque while setting up a Dynamixel ID {id}: {error}"),
            Self::VelocityProfile { id, ref error } => write!(f, "Error configuring a velocity profile for Dynamixel ID {id}: {error}"),
            Self::AccelerationProfile { id, ref error } => write!(f, "Error configuring an acceleration profile for Dynamixel ID {id}: {error}"),
            Self::TorqueOn { id, ref error } => write!(f, "Error enabling torque while setting up a Dynamixel ID {id}: {error}"),
            #[cfg(debug_assertions)]
            Self::Mutex(ref e) => write!(f, "Error waiting for permission to use the serial bus to check Dynamixel IDs already in use: {e}"),
            #[cfg(debug_assertions)]
            Self::Id(ref e) => defmt::Format::fmt(e, f),
        }
    }
}
*/

pub struct Actuator<'bus, const ID: u8, C: Comm, M: Mutex<Item = Bus<C>>>(&'bus M);

impl<'bus, const ID: u8, C: Comm, M: Mutex<Item = Bus<C>>> Actuator<'bus, ID, C, M> {
    #[inline(always)]
    pub async fn new(bus: &'bus M) -> Result<Self, InitError<C, M>> {
        let actuator = Self(bus);
        let () = actuator
            .write_torque_enable(0)
            .await
            .map_err(|error| InitError::TorqueOff { id: ID, error })?;
        {
            let mut max = u32::MAX;
            'max_velocity: loop {
                match actuator.write_profile_velocity(max).await {
                    Ok(()) => break 'max_velocity,
                    Err(crate::Error::Bus(crate::bus::Error::Parse(
                        ::dxl_packet::packet::recv::PersistentError::Software(
                            ::dxl_packet::packet::recv::SoftwareError::DataRangeError,
                        ),
                    ))) => {
                        defmt::debug!(
                            "Maximum velocity of `{}` is too much; cutting in half...",
                            max
                        );
                        max >>= 1
                    }
                    Err(error) => return Err(InitError::VelocityProfile { id: ID, error }),
                }
            }
        }
        let () = actuator
            .write_profile_acceleration(128)
            .await
            .map_err(|error| InitError::AccelerationProfile { id: ID, error })?;
        let () = actuator
            .write_torque_enable(1)
            .await
            .map_err(|error| InitError::TorqueOn { id: ID, error })?;

        #[cfg(debug_assertions)]
        let () = bus
            .lock()
            .await
            .map_err(InitError::Mutex)?
            .check_duplicate_id(ID)
            .map_err(InitError::Id)?;

        Ok(actuator)
    }

    instruction_method!(ping);
    instruction_method!(action);
    instruction_method!(factory_reset);
    instruction_method!(reboot);

    control_table_methods!(ModelNumber, 16);
    control_table_methods!(ModelInformation, 32);
    control_table_methods!(FirmwareVersion, 8);
    control_table_methods!(Id, 8);
    control_table_methods!(BaudRate, 8);
    control_table_methods!(ReturnDelayTime, 8);
    control_table_methods!(DriveMode, 8);
    control_table_methods!(OperatingMode, 8);
    control_table_methods!(SecondaryId, 8);
    control_table_methods!(ProtocolType, 8);
    control_table_methods!(HomingOffset, 32);
    control_table_methods!(MovingThreshold, 32);
    control_table_methods!(TemperatureLimit, 8);
    control_table_methods!(MaxVoltageLimit, 16);
    control_table_methods!(MinVoltageLimit, 16);
    control_table_methods!(PwmLimit, 16);
    control_table_methods!(CurrentLimit, 16);
    control_table_methods!(VelocityLimit, 32);
    control_table_methods!(MaxPositionLimit, 32);
    control_table_methods!(MinPositionLimit, 32);
    control_table_methods!(StartupConfiguration, 8);
    control_table_methods!(PwmSlope, 8);
    control_table_methods!(Shutdown, 8);
    control_table_methods!(TorqueEnable, 8);
    control_table_methods!(Led, 8);
    control_table_methods!(StatusReturnLevel, 8);
    control_table_methods!(RegisteredInstruction, 8);
    control_table_methods!(HardwareErrorStatus, 8);
    control_table_methods!(VelocityIGain, 16);
    control_table_methods!(VelocityPGain, 16);
    control_table_methods!(PositionDGain, 16);
    control_table_methods!(PositionIGain, 16);
    control_table_methods!(PositionPGain, 16);
    control_table_methods!(Feedforward2ndGain, 16);
    control_table_methods!(Feedforward1stGain, 16);
    control_table_methods!(BusWatchdog, 8);
    control_table_methods!(GoalPwm, 16);
    control_table_methods!(GoalCurrent, 16);
    control_table_methods!(GoalVelocity, 32);
    control_table_methods!(ProfileAcceleration, 32);
    control_table_methods!(ProfileVelocity, 32);
    control_table_methods!(GoalPosition, 32);
    control_table_methods!(RealtimeTick, 16);
    control_table_methods!(Moving, 8);
    control_table_methods!(MovingStatus, 8);
    control_table_methods!(PresentPwm, 16);
    control_table_methods!(PresentCurrent, 16);
    control_table_methods!(PresentVelocity, 32);
    control_table_methods!(PresentPosition, 32);
    control_table_methods!(VelocityTrajectory, 32);
    control_table_methods!(PositionTrajectory, 32);
    control_table_methods!(PresentInputVoltage, 16);
    control_table_methods!(PresentTemperature, 8);
    control_table_methods!(BackupReady, 8);
}
