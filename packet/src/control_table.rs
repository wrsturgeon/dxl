pub trait Item {
    const ADDRESS: u8;
    const BYTES: u16;
    const DESCRIPTION: &str;
}

pub struct ModelNumber;
impl Item for ModelNumber {
    const ADDRESS: u8 = 0;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Model Number";
}

pub struct ModelInformation;
impl Item for ModelInformation {
    const ADDRESS: u8 = 2;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Model Information";
}

pub struct FirmwareVersion;
impl Item for FirmwareVersion {
    const ADDRESS: u8 = 6;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Firmware Version";
}

pub struct Id;
impl Item for Id {
    const ADDRESS: u8 = 7;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "ID";
}

pub struct BaudRate;
impl Item for BaudRate {
    const ADDRESS: u8 = 8;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Baud Rate";
}

pub struct ReturnDelayTime;
impl Item for ReturnDelayTime {
    const ADDRESS: u8 = 9;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Return Delay Time";
}

pub struct DriveMode;
impl Item for DriveMode {
    const ADDRESS: u8 = 10;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Drive Mode";
}

pub struct OperatingMode;
impl Item for OperatingMode {
    const ADDRESS: u8 = 11;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Operating Mode";
}

pub struct SecondaryId;
impl Item for SecondaryId {
    const ADDRESS: u8 = 12;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Secondary ID";
}

pub struct ProtocolType;
impl Item for ProtocolType {
    const ADDRESS: u8 = 13;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Protocol Type";
}

pub struct HomingOffset;
impl Item for HomingOffset {
    const ADDRESS: u8 = 20;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Homing Offset";
}

pub struct MovingThreshold;
impl Item for MovingThreshold {
    const ADDRESS: u8 = 24;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Moving Threshold";
}

pub struct TemperatureLimit;
impl Item for TemperatureLimit {
    const ADDRESS: u8 = 31;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Temperature Limit";
}

pub struct MaxVoltageLimit;
impl Item for MaxVoltageLimit {
    const ADDRESS: u8 = 32;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Max Voltage Limit";
}

pub struct MinVoltageLimit;
impl Item for MinVoltageLimit {
    const ADDRESS: u8 = 34;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Min Voltage Limit";
}

pub struct PwmLimit;
impl Item for PwmLimit {
    const ADDRESS: u8 = 36;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "PWM Limit";
}

pub struct CurrentLimit;
impl Item for CurrentLimit {
    const ADDRESS: u8 = 38;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Current Limit";
}

pub struct VelocityLimit;
impl Item for VelocityLimit {
    const ADDRESS: u8 = 44;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Velocity Limit";
}

pub struct MaxPositionLimit;
impl Item for MaxPositionLimit {
    const ADDRESS: u8 = 48;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Max Position Limit";
}

pub struct MinPositionLimit;
impl Item for MinPositionLimit {
    const ADDRESS: u8 = 52;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Min Position Limit";
}

pub struct StartupConfiguration;
impl Item for StartupConfiguration {
    const ADDRESS: u8 = 60;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Startup Configuration";
}

pub struct PwmSlope;
impl Item for PwmSlope {
    const ADDRESS: u8 = 62;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "PWM Slope";
}

pub struct Shutdown;
impl Item for Shutdown {
    const ADDRESS: u8 = 63;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Shutdown";
}

pub struct TorqueEnable;
impl Item for TorqueEnable {
    const ADDRESS: u8 = 64;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Torque Enable";
}

pub struct Led;
impl Item for Led {
    const ADDRESS: u8 = 65;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "LED";
}

pub struct StatusReturnLevel;
impl Item for StatusReturnLevel {
    const ADDRESS: u8 = 68;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Status Return Level";
}

pub struct RegisteredInstruction;
impl Item for RegisteredInstruction {
    const ADDRESS: u8 = 69;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Registered Instruction";
}

pub struct HardwareErrorStatus;
impl Item for HardwareErrorStatus {
    const ADDRESS: u8 = 70;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Hardware Error Status";
}

pub struct VelocityIGain;
impl Item for VelocityIGain {
    const ADDRESS: u8 = 76;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Velocity I Gain";
}

pub struct VelocityPGain;
impl Item for VelocityPGain {
    const ADDRESS: u8 = 78;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Velocity P Gain";
}

pub struct PositionDGain;
impl Item for PositionDGain {
    const ADDRESS: u8 = 80;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Position D Gain";
}

pub struct PositionIGain;
impl Item for PositionIGain {
    const ADDRESS: u8 = 82;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Position I Gain";
}

pub struct PositionPGain;
impl Item for PositionPGain {
    const ADDRESS: u8 = 84;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Position P Gain";
}

pub struct Feedforward2ndGain;
impl Item for Feedforward2ndGain {
    const ADDRESS: u8 = 88;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Feedforward Second Gain";
}

pub struct Feedforward1stGain;
impl Item for Feedforward1stGain {
    const ADDRESS: u8 = 90;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Feedforward First Gain";
}

pub struct BusWatchdog;
impl Item for BusWatchdog {
    const ADDRESS: u8 = 98;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Bus Watchdog";
}

pub struct GoalPwm;
impl Item for GoalPwm {
    const ADDRESS: u8 = 100;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Goal PWM";
}

pub struct GoalCurrent;
impl Item for GoalCurrent {
    const ADDRESS: u8 = 102;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Goal Current";
}

pub struct GoalVelocity;
impl Item for GoalVelocity {
    const ADDRESS: u8 = 104;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Goal Velocity";
}

pub struct ProfileAcceleration;
impl Item for ProfileAcceleration {
    const ADDRESS: u8 = 108;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Profile Acceleration";
}

pub struct ProfileVelocity;
impl Item for ProfileVelocity {
    const ADDRESS: u8 = 112;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Profile Velocity";
}

pub struct GoalPosition;
impl Item for GoalPosition {
    const ADDRESS: u8 = 116;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Goal Position";
}

pub struct RealtimeTick;
impl Item for RealtimeTick {
    const ADDRESS: u8 = 120;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Real-Time Tick";
}

pub struct Moving;
impl Item for Moving {
    const ADDRESS: u8 = 122;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Moving";
}

pub struct MovingStatus;
impl Item for MovingStatus {
    const ADDRESS: u8 = 123;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Moving Status";
}

pub struct PresentPwm;
impl Item for PresentPwm {
    const ADDRESS: u8 = 124;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Present PWM";
}

pub struct PresentCurrent;
impl Item for PresentCurrent {
    const ADDRESS: u8 = 126;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Present Current";
}

pub struct PresentVelocity;
impl Item for PresentVelocity {
    const ADDRESS: u8 = 128;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Present Velocity";
}

pub struct PresentPosition;
impl Item for PresentPosition {
    const ADDRESS: u8 = 132;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Present Position";
}

pub struct VelocityTrajectory;
impl Item for VelocityTrajectory {
    const ADDRESS: u8 = 136;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Velocity Trajectory";
}

pub struct PositionTrajectory;
impl Item for PositionTrajectory {
    const ADDRESS: u8 = 140;
    const BYTES: u16 = 4;
    const DESCRIPTION: &str = "Position Trajectory";
}

pub struct PresentInputVoltage;
impl Item for PresentInputVoltage {
    const ADDRESS: u8 = 144;
    const BYTES: u16 = 2;
    const DESCRIPTION: &str = "Present Input Voltage";
}

pub struct PresentTemperature;
impl Item for PresentTemperature {
    const ADDRESS: u8 = 146;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Present Temperature";
}

pub struct BackupReady;
impl Item for BackupReady {
    const ADDRESS: u8 = 147;
    const BYTES: u16 = 1;
    const DESCRIPTION: &str = "Backup Ready";
}

#[repr(u8)]
pub enum Baud {
    Baud9600 = 0,
    Baud57600 = 1,
    Baud115200 = 2,
    Baud1000000 = 3,
    Baud2000000 = 4,
    Baud3000000 = 5,
    Baud4000000 = 6,
}
