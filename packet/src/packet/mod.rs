pub mod recv;
pub mod send;

use {core::fmt, enum_repr::EnumRepr};

#[inline]
pub const fn new<Insn: crate::Instruction>(id: u8, instruction: Insn) -> send::WithCrc<Insn>
where
    [(); { core::mem::size_of::<Insn>() as u16 + 3 } as usize]:,
    [(); { Insn::BYTE } as usize]:,
{
    let without_crc = send::WithoutCrc::new(id, instruction);
    let crc = {
        let mut crc_state = const { send::WithoutCrc::<Insn>::crc_init() };
        let () = crc_state.recurse_over_bytes({
            let ptr = {
                let init_ptr = &without_crc as *const _ as *const u8;
                unsafe { init_ptr.byte_offset(4) }
            };
            unsafe {
                core::slice::from_raw_parts(
                    ptr,
                    const { core::mem::size_of::<send::WithoutCrc<Insn>>() - 4 },
                )
            }
        });
        crc_state.collapse().to_le_bytes()
    };
    send::WithCrc { without_crc, crc }
}

#[derive(defmt::Format)]
#[EnumRepr(type = "u8")]
pub enum Instruction {
    Ping = 0x01,
    Read = 0x02,
    Write = 0x03,
    RegWrite = 0x04,
    Action = 0x05,
    FactoryReset = 0x06,
    Reboot = 0x08,
    Clear = 0x10,
    ControlTableBackup = 0x20,
    Status = 0x55,
    SyncRead = 0x82,
    SyncWrite = 0x83,
    FastSyncRead = 0x8A,
    BulkRead = 0x92,
    BulkWrite = 0x93,
    FastBulkRead = 0x9A,
}

impl fmt::Display for Instruction {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Ping => write!(f, "Ping"),
            Self::Read => write!(f, "Read"),
            Self::Write => write!(f, "Write"),
            Self::RegWrite => write!(f, "Register-Write"),
            Self::Action => write!(f, "Action"),
            Self::FactoryReset => write!(f, "Factory Reset"),
            Self::Reboot => write!(f, "Reboot"),
            Self::Clear => write!(f, "Clear"),
            Self::ControlTableBackup => write!(f, "Control Table Backup"),
            Self::Status => write!(f, "Status"),
            Self::SyncRead => write!(f, "Synchronized Read"),
            Self::SyncWrite => write!(f, "Synchronized Write"),
            Self::FastSyncRead => write!(f, "Fast Synchronized Read"),
            Self::BulkRead => write!(f, "Bulk Read"),
            Self::BulkWrite => write!(f, "Bulk Write"),
            Self::FastBulkRead => write!(f, "Fast Bulk Read"),
        }
    }
}

#[derive(defmt::Format)]
#[EnumRepr(type = "u16")]
pub enum ControlTableAddress {
    ModelNumber = 0,
    ModelInfo = 2,
    FirmwareVersion = 6,
    Id = 7,
    BaudRate = 8,
    ReturnDelayTime = 9,
    DriveMode = 10,
    OperatingMode = 11,
    ShadowId = 12,
    ProtocolType = 13,
    HomingOffset = 20,
    MovingThreshold = 24,
    TemperatureLimit = 31,
    MaxVoltageLimit = 34,
    PwmLimit = 36,
    CurrentLimit = 38,
    VelocityLimit = 44,
    MaxPositionLimit = 48,
    MinPositionLimit = 52,
    ExternalPortMode1 = 56,
    ExternalPortMode2 = 57,
    ExternalPortMode3 = 58,
    StartupConfig = 60,
    Shutdown = 63,
    TorqueEnable = 64,
    Led = 65,
    StatusReturnLevel = 68,
    RegisteredInstruction = 69,
    HardwareErrorStatus = 70,
    VelocityIGain = 76,
    VelocityPGain = 78,
    PositionDGain = 80,
    PositionIGain = 82,
    PositionPGain = 84,
    Feedforward2ndGain = 88,
    Feedforward1stGain = 90,
    BusWatchdog = 98,
    GoalPwm = 100,
    GoalCurrent = 102,
    GoalVelocity = 104,
    ProfileAcceleration = 108,
    ProfileVelocity = 112,
    GoalPosition = 116,
    RealtimeTick = 120,
    Moving = 122,
    MovingStatus = 123,
    PresentPwm = 124,
    PresentCurrent = 126,
    PresentVelocity = 128,
    PresentPosition = 132,
    VelocityTrajectory = 136,
    PositionTrajectory = 140,
    PresentInputVoltage = 144,
    PresentTemperature = 146,
    BackupReady = 147,
    ExternalPortData1 = 152,
    ExternalPortData2 = 154,
    ExternalPortData3 = 156,
    // then a bunch of indirect addresses
    Unrecognized = u16::MAX,
}

impl fmt::Display for ControlTableAddress {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::ModelNumber => write!(f, "Model Number"),
            Self::ModelInfo => write!(f, "Model Info"),
            Self::FirmwareVersion => write!(f, "Firmware Version"),
            Self::Id => write!(f, "ID"),
            Self::BaudRate => write!(f, "Baud Rate"),
            Self::ReturnDelayTime => write!(f, "Return Delay Time"),
            Self::DriveMode => write!(f, "Drive Mode"),
            Self::OperatingMode => write!(f, "Operating Mode"),
            Self::ShadowId => write!(f, "Shadow ID"),
            Self::ProtocolType => write!(f, "Protocol Type"),
            Self::HomingOffset => write!(f, "Homing Offset"),
            Self::MovingThreshold => write!(f, "Moving Threshold"),
            Self::TemperatureLimit => write!(f, "Temperature Limit"),
            Self::MaxVoltageLimit => write!(f, "Max. Voltage Limit"),
            Self::PwmLimit => write!(f, "PWM Limit"),
            Self::CurrentLimit => write!(f, "Current Limit"),
            Self::VelocityLimit => write!(f, "Velocity Limit"),
            Self::MaxPositionLimit => write!(f, "Max. Position Limit"),
            Self::MinPositionLimit => write!(f, "Min. Position Limit"),
            Self::ExternalPortMode1 => write!(f, "External Port Mode #1"),
            Self::ExternalPortMode2 => write!(f, "External Port Mode #2"),
            Self::ExternalPortMode3 => write!(f, "External Port Mode #3"),
            Self::StartupConfig => write!(f, "Startup Configuration"),
            Self::Shutdown => write!(f, "Shutdown"),
            Self::TorqueEnable => write!(f, "Torque Enable"),
            Self::Led => write!(f, "LED"),
            Self::StatusReturnLevel => write!(f, "Status Return Level"),
            Self::RegisteredInstruction => write!(f, "Registered Instruction"),
            Self::HardwareErrorStatus => write!(f, "Hardware Error Status"),
            Self::VelocityIGain => write!(f, "Velocity I Gain"),
            Self::VelocityPGain => write!(f, "Velocity P Gain"),
            Self::PositionDGain => write!(f, "Position D Gain"),
            Self::PositionIGain => write!(f, "Position I Gain"),
            Self::PositionPGain => write!(f, "Position P Gain"),
            Self::Feedforward2ndGain => write!(f, "Feedforward2ndGain"),
            Self::Feedforward1stGain => write!(f, "Feedforward1stGain"),
            Self::BusWatchdog => write!(f, "Bus Watchdog"),
            Self::GoalPwm => write!(f, "Goal PWM"),
            Self::GoalCurrent => write!(f, "Goal Current"),
            Self::GoalVelocity => write!(f, "Goal Velocity"),
            Self::ProfileAcceleration => write!(f, "Profile Acceleration"),
            Self::ProfileVelocity => write!(f, "Profile Velocity"),
            Self::GoalPosition => write!(f, "Goal Position"),
            Self::RealtimeTick => write!(f, "Real-Time Tick"),
            Self::Moving => write!(f, "Moving"),
            Self::MovingStatus => write!(f, "Moving Status"),
            Self::PresentPwm => write!(f, "Present PWM"),
            Self::PresentCurrent => write!(f, "Present Current"),
            Self::PresentVelocity => write!(f, "Present Velocity"),
            Self::PresentPosition => write!(f, "Present Position"),
            Self::VelocityTrajectory => write!(f, "Velocity Trajectory"),
            Self::PositionTrajectory => write!(f, "Position Trajectory"),
            Self::PresentInputVoltage => write!(f, "Present Input Voltage"),
            Self::PresentTemperature => write!(f, "Present Temperature"),
            Self::BackupReady => write!(f, "Back-Up Ready"),
            Self::ExternalPortData1 => write!(f, "External Port Data #1"),
            Self::ExternalPortData2 => write!(f, "External Port Data #2"),
            Self::ExternalPortData3 => write!(f, "External Port Data #3"),
            Self::Unrecognized => write!(f, "[unrecognized control table address]"),
        }
    }
}
