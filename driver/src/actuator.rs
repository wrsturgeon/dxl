use {
    crate::{bus::Bus, comm::Comm, mutex::Mutex},
    dxl_packet::{control_table, instruction, parse::Parse},
    paste::paste,
};

pub struct Actuator<'bus, const ID: u8, C: Comm, M: Mutex<Item = Bus<C>>>(&'bus M);

macro_rules! instruction_method {
    ($id:ident) => {
        #[inline(always)]
        pub async fn $id(
            &mut self,
        ) -> Result<
            <paste! { instruction::recv::[< $id:camel >] } as Parse<u8>>::Output,
            crate::Error<C>,
        > {
            self.0.lock().await.$id::<ID>().await
        }
    };
}

macro_rules! control_table_methods {
    ($id:ident) => { paste! {
        #[inline]
        pub async fn [< read_ $id:snake >](
            &mut self,
        ) -> Result<<instruction::recv::Read<control_table::$id> as Parse<u8>>::Output, crate::Error<C>>
        {
            self.0.lock().await.[< read_ $id:snake >]::<ID>().await
        }

        #[inline]
        pub async fn [< write_ $id:snake >](
            &mut self,
            bytes: [u8; <control_table::$id as control_table::Item>::BYTES as usize]
        ) -> Result<<instruction::recv::Write<control_table::$id> as Parse<u8>>::Output, crate::Error<C>>
        {
            self.0.lock().await.[< write_ $id:snake >]::<ID>(bytes).await
        }

        #[inline]
        pub async fn [< reg_write_ $id:snake >](
            &mut self,
            bytes: [u8; <control_table::$id as control_table::Item>::BYTES as usize]
        ) -> Result<<instruction::recv::RegWrite<control_table::$id> as Parse<u8>>::Output, crate::Error<C>>
        {
            self.0.lock().await.[< reg_write_ $id:snake >]::<ID>(bytes).await
        }
    } };
}

impl<'bus, const ID: u8, C: Comm, M: Mutex<Item = Bus<C>>> Actuator<'bus, ID, C, M> {
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
