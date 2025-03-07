use {
    crate::comm::Comm,
    dxl_packet::{
        control_table,
        instruction::{self, Instruction},
        packet,
        parse::Parse,
    },
    paste::paste,
};

pub struct Bus<C: Comm>(C);

macro_rules! instruction_method {
    ($id:ident) => {
        #[inline]
        pub async fn $id<const ID: u8>(
            &mut self,
        ) -> Result<
            <paste! { instruction::recv::[< $id:camel >] } as Parse<u8>>::Output,
            crate::Error<C>,
        > {
            self.comm::<paste! { instruction::[< $id:camel >] }, ID>({
                let payload = <paste! { instruction::[< $id:camel >] } as Instruction>::Send::new();
                log::debug!("Sending {payload:?} to DXL {ID}");
                payload
            })
            .await
        }
    };
}

macro_rules! control_table_methods {
    ($id:ident) => { paste! {
        #[inline]
        pub async fn [< read_ $id:snake >]<const ID: u8>(
            &mut self,
        ) -> Result<<instruction::recv::Read<control_table::$id> as Parse<u8>>::Output, crate::Error<C>> {
            self.comm::<instruction::Read<control_table::$id>, ID>({
                let payload = <instruction::Read<control_table::$id> as Instruction>::Send::new();
                log::debug!("Reading {payload:?} from DXL {ID}");
                payload
            })
            .await
        }

        #[inline]
        pub async fn [< write_ $id:snake >]<const ID: u8>(
            &mut self,
            bytes: [u8; <control_table::$id as control_table::Item>::BYTES as usize]
        ) -> Result<<instruction::recv::Write<control_table::$id> as Parse<u8>>::Output, crate::Error<C>> {
            self.comm::<instruction::Write<control_table::$id>, ID>({
                let payload = <instruction::Write<control_table::$id> as Instruction>::Send::new(bytes);
                log::debug!("Writing {payload:?} to DXL {ID}");
                payload
            })
            .await
        }

        #[inline]
        pub async fn [< reg_write_ $id:snake >]<const ID: u8>(
            &mut self,
            bytes: [u8; <control_table::$id as control_table::Item>::BYTES as usize]
        ) -> Result<<instruction::recv::RegWrite<control_table::$id> as Parse<u8>>::Output, crate::Error<C>> {
            self.comm::<instruction::RegWrite<control_table::$id>, ID>({
                let payload = <instruction::RegWrite<control_table::$id> as Instruction>::Send::new(bytes);
                log::debug!("Register-writing {payload:?} to DXL {ID}");
                payload
            })
            .await
        }
    } };
}

impl<C: Comm> Bus<C> {
    #[inline]
    pub async fn comm<Insn: Instruction, const ID: u8>(
        &mut self,
        parameters: Insn::Send,
    ) -> Result<<Insn::Recv as Parse<u8>>::Output, crate::Error<C>>
    where
        [(); { Insn::BYTE } as usize]:,
        [(); { core::mem::size_of::<Insn::Send>() as u16 + 3 } as usize]:,
        [(); { core::mem::size_of::<Insn::Recv>() as u16 + 4 } as usize]:,
        [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) & 0xFF) as u8 } as usize]:,
        [(); { ((core::mem::size_of::<Insn::Recv>() as u16 + 4) >> 8) as u8 } as usize]:,
    {
        packet::parse::<Insn, ID>(
            &mut self
                .0
                .comm(packet::new::<Insn, ID>(parameters).as_buffer())
                .await
                .map_err(crate::Error::Send)?,
        )
        .await
        .map_err(crate::Error::Packet)
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
