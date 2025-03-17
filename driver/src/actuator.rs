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
            #[cfg(debug_assertions)]
            {
                paste! { defmt::debug!("{} {}...", <::dxl_packet::send::[< $id:camel >] as ::dxl_packet::Instruction>::GERUND, self) };
            }
            self.bus
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
            ) -> Result<[< u $bits >], crate::Error<C, M, [< u $bits >]>> {
                #[cfg(debug_assertions)]
                {
                    defmt::debug!("Reading {}'s {}...", self, <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::DESCRIPTION);
                }
                let ::dxl_packet::recv::Read { bytes } = self.bus
                    .lock()
                    .await
                    .map_err(crate::Error::Mutex)?
                    .[< read_ $id:snake >]::<ID>()
                    .await
                    .map_err(|e| crate::Error::Bus(e.map(|::dxl_packet::recv::Read { bytes }| [< u $bits >]::from_le_bytes(bytes))))?;
                let uint = [< u $bits >]::from_le_bytes(bytes);
                #[cfg(debug_assertions)]
                {
                    defmt::debug!("    --> {}'s {} is {}", self, <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::DESCRIPTION, uint);
                }
                Ok(uint)
            }

            #[inline]
            pub async fn [< write_ $id:snake >](
                &self, value: [< u $bits >],
            ) -> Result<(), crate::Error<C, M, ()>> {
                #[cfg(debug_assertions)]
                {
                    defmt::debug!("Writing {}'s {} to {}...", self, <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::DESCRIPTION, value);
                }
                let () = self.bus.lock().await.map_err(crate::Error::Mutex)?.[< write_ $id:snake >]::<ID>(value.to_le_bytes()).await.map_err(crate::Error::Bus)?;
                #[cfg(debug_assertions)]
                {
                    defmt::debug!("    --> updated {}'s {} to {}", self, <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::DESCRIPTION, value);
                }
                Ok(())
            }

            #[inline]
            pub async fn [< reg_write_ $id:snake >](
                &self, value: [< u $bits >],
            ) -> Result<(), crate::Error<C, M, ()>> {
                #[cfg(debug_assertions)]
                {
                    defmt::debug!("Register-writing {}'s {} to {}...", self, <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::DESCRIPTION, value);
                }
                let () = self.bus.lock().await.map_err(crate::Error::Mutex)?.[< reg_write_ $id:snake >]::<ID>(value.to_le_bytes()).await.map_err(crate::Error::Bus)?;
                #[cfg(debug_assertions)]
                {
                    defmt::debug!("    --> registered an update of {}'s {} to {}", self, <::dxl_packet::control_table::$id as ::dxl_packet::control_table::Item>::DESCRIPTION, value);
                }
                Ok(())
            }
        }
    };
}

pub enum InitError<C: Comm, M: Mutex> {
    Write {
        id: u8,
        error: crate::Error<C, M, ::dxl_packet::recv::Write>,
    },
    FollowTo(FollowToError<C, M>),
    #[cfg(debug_assertions)]
    Id(crate::bus::IdError),
    #[cfg(debug_assertions)]
    Mutex(M::Error),
}

impl<C: Comm, M: Mutex> defmt::Format for InitError<C, M> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::Write { id, ref error } => defmt::write!(
                f,
                "Error setting parameters while initializing Dynamixel ID {}: {}",
                id,
                error
            ),
            Self::FollowTo(ref e) => defmt::Format::format(e, f),
            #[cfg(debug_assertions)]
            Self::Id(ref e) => defmt::Format::format(e, f),
            #[cfg(debug_assertions)]
            Self::Mutex(ref e) => defmt::Format::format(e, f),
        }
    }
}

pub enum RelativePositionError<C: Comm, M: Mutex> {
    LessThanZero {
        id: u8,
        position: f32,
    },
    GreaterThanOne {
        id: u8,
        position: f32,
    },
    Limits {
        id: u8,
        error: crate::Error<C, M, u32>,
    },
}

impl<C: Comm, M: Mutex> defmt::Format for RelativePositionError<C, M> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::LessThanZero { id, position } => defmt::write!(f, "Dynamixel ID {} received a position less than zero: {} (note that positions must be between 0 and 1, representing 0% and 100% of the range between their configured limits)", id, position),
            Self::GreaterThanOne { id, position } => defmt::write!(f, "Dynamixel ID {} received a position greater than one: {} (note that positions must be between 0 and 1, representing 0% and 100% of the range between their configured limits)", id, position),
            Self::Limits { id, ref error } => defmt::write!(
                f,
                "Error reading position limits for Dynamixel ID {}: {}",
                id,
                error
            ),
        }
    }
}

pub enum GoToError<C: Comm, M: Mutex> {
    RelativePosition(RelativePositionError<C, M>),
    Write {
        id: u8,
        error: crate::Error<C, M, ()>,
    },
}

impl<C: Comm, M: Mutex> defmt::Format for GoToError<C, M> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::RelativePosition(ref e) => defmt::Format::format(e, f),
            Self::Write { id, ref error } => {
                defmt::write!(f, "Error writing to Dynamixel ID {}: {}", id, error)
            }
        }
    }
}

pub enum FollowToError<C: Comm, M: Mutex> {
    RelativePosition(RelativePositionError<C, M>),
    Write {
        id: u8,
        error: crate::Error<C, M, ()>,
    },
    Position(PosError<C, M>),
}

impl<C: Comm, M: Mutex> defmt::Format for FollowToError<C, M> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::RelativePosition(ref e) => defmt::Format::format(e, f),
            Self::Write { id, ref error } => {
                defmt::write!(f, "Error writing to Dynamixel ID {}: {}", id, error)
            }
            Self::Position(ref e) => defmt::Format::format(e, f),
        }
    }
}

pub enum PosError<C: Comm, M: Mutex> {
    Read {
        id: u8,
        error: crate::Error<C, M, u32>,
    },
    RelativePosition(RelativePositionError<C, M>),
}

impl<C: Comm, M: Mutex> defmt::Format for PosError<C, M> {
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        match *self {
            Self::Read { id, ref error } => {
                defmt::write!(f, "Error reading from Dynamixel ID {}: {}", id, error)
            }
            Self::RelativePosition(ref e) => defmt::Format::format(e, f),
        }
    }
}

pub struct KnownLimits {
    min: f32,
    range: f32,
}

pub struct Actuator<'bus, const ID: u8, C: Comm, M: Mutex<Item = Bus<C>>> {
    bus: &'bus M,
    description: &'static str,
    limits: Option<KnownLimits>,
}

impl<'bus, const ID: u8, C: Comm, M: Mutex<Item = Bus<C>>> Actuator<'bus, ID, C, M> {
    #[inline(always)]
    pub async fn init_unconfigured(
        bus: &'bus M,
        description: &'static str,
    ) -> Result<Self, InitError<C, M>> {
        let actuator = Self {
            bus,
            description,
            limits: None,
        };

        #[cfg(debug_assertions)]
        let () = bus
            .lock()
            .await
            .map_err(InitError::Mutex)?
            .check_duplicate_id(ID)
            .map_err(InitError::Id)?;

        Ok(actuator)
    }

    #[inline(always)]
    async fn init_with_max_velocity(
        bus: &'bus M,
        description: &'static str,
    ) -> Result<Self, InitError<C, M>> {
        let actuator = Self::init_unconfigured(bus, description).await?;
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
                Err(error) => return Err(InitError::Write { id: ID, error }),
            }
        }
        Ok(actuator)
    }

    #[inline(always)]
    async fn init_with_profile(
        bus: &'bus M,
        description: &'static str,
    ) -> Result<Self, InitError<C, M>> {
        let actuator = Self::init_with_max_velocity(bus, description).await?;
        actuator
            .reset_acceleration_profile()
            .await
            .map_err(|error| InitError::Write { id: ID, error })?;
        Ok(actuator)
    }

    #[inline(always)]
    pub async fn init_in_place(
        bus: &'bus M,
        description: &'static str,
    ) -> Result<Self, InitError<C, M>> {
        let actuator = Self::init_with_profile(bus, description).await?;
        let () = actuator
            .torque_on()
            .await
            .map_err(|error| InitError::Write { id: ID, error })?;
        Ok(actuator)
    }

    #[inline]
    pub async fn init_at_position(
        bus: &'bus M,
        description: &'static str,
        position: f32,
        tolerance: f32,
    ) -> Result<Self, InitError<C, M>> {
        let mut actuator = Self::init_with_max_velocity(bus, description).await?;
        let () = actuator
            .write_profile_acceleration(1)
            .await
            .map_err(|error| InitError::Write { id: ID, error })?;
        defmt::info!("Slowly moving {} to position {}...", actuator, position);
        let () = actuator
            .torque_on()
            .await
            .map_err(|error| InitError::Write { id: ID, error })?;
        actuator
            .follow_to(position, tolerance)
            .await
            .map_err(InitError::FollowTo)?;
        actuator
            .reset_acceleration_profile()
            .await
            .map_err(|error| InitError::Write { id: ID, error })?;
        Ok(actuator)
    }

    #[inline(always)]
    pub async fn reset_acceleration_profile(&self) -> Result<(), crate::Error<C, M, ()>> {
        self.write_profile_acceleration(
            // Snappy enough without seeming digital:
            128,
        )
        .await
    }

    #[inline(always)]
    pub async fn torque_off(&self) -> Result<(), crate::Error<C, M, ()>> {
        self.write_torque_enable(0).await
    }

    #[inline(always)]
    pub async fn torque_on(&self) -> Result<(), crate::Error<C, M, ()>> {
        self.write_torque_enable(1).await
    }

    #[inline]
    pub async fn limits(&mut self) -> Result<&KnownLimits, crate::Error<C, M, u32>> {
        // If not already cached, calculate and cache:
        Ok(match self.limits {
            Some(ref known) => known,
            None => self.limits.insert({
                let max: u32 = self.read_max_position_limit().await?;
                let min: u32 = self.read_min_position_limit().await?;
                KnownLimits {
                    min: min as f32,
                    range: (max - min) as f32,
                }
            }),
        })
    }

    #[inline]
    async fn make_position_absolute(
        &mut self,
        relative: f32,
    ) -> Result<u32, RelativePositionError<C, M>> {
        if relative < 0. {
            return Err(RelativePositionError::LessThanZero {
                id: ID,
                position: relative,
            });
        }
        if relative > 1. {
            return Err(RelativePositionError::GreaterThanOne {
                id: ID,
                position: relative,
            });
        }
        let KnownLimits { min, range } = self
            .limits()
            .await
            .map_err(|error| RelativePositionError::Limits { id: ID, error })?;
        let absolute_position = min + (range * relative);
        Ok(absolute_position as u32)
    }

    #[inline]
    async fn make_position_relative(
        &mut self,
        absolute: u32,
    ) -> Result<f32, RelativePositionError<C, M>> {
        let KnownLimits { min, range } = self
            .limits()
            .await
            .map_err(|error| RelativePositionError::Limits { id: ID, error })?;
        Ok((absolute as f32 - min) / range)
    }

    #[inline]
    pub async fn go_to(&mut self, position: f32) -> Result<(), GoToError<C, M>> {
        let absolute_position = self
            .make_position_absolute(position)
            .await
            .map_err(GoToError::RelativePosition)?;
        self.write_goal_position(absolute_position)
            .await
            .map_err(|error| GoToError::Write { id: ID, error })
    }

    #[inline]
    pub async fn follow_to(
        &mut self,
        position: f32,
        tolerance: f32,
    ) -> Result<(), FollowToError<C, M>> {
        let absolute_position = self
            .make_position_absolute(position)
            .await
            .map_err(FollowToError::RelativePosition)?;
        let () = self
            .write_goal_position(absolute_position)
            .await
            .map_err(|error| FollowToError::Write { id: ID, error })?;
        loop {
            let actual_position = self.pos().await.map_err(FollowToError::Position)?;
            if (position - actual_position).abs() <= tolerance {
                defmt::debug!("Dynamixel #{} reached its goal position ({})", ID, position);
                return Ok(());
            }
        }
    }

    #[inline(always)]
    pub async fn pos(&mut self) -> Result<f32, PosError<C, M>> {
        let absolute = self
            .read_present_position()
            .await
            .map_err(|error| PosError::Read { id: ID, error })?;
        self.make_position_relative(absolute)
            .await
            .map_err(PosError::RelativePosition)
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

impl<'bus, const ID: u8, C: Comm, M: Mutex<Item = Bus<C>>> defmt::Format
    for Actuator<'bus, ID, C, M>
{
    #[inline]
    fn format(&self, f: defmt::Formatter) {
        defmt::write!(f, "Dynamixel ID {} (\"{}\")", ID, self.description)
    }
}
