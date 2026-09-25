//! System constants and strongly typed enums for NLECloud devices, sensor tags, and actuators.
//!
//! These enums map directly to NLECloud API magic numbers and provide type-safe conversions
//! for SDK consumers and future upstreaming into `nle-cloud-sdk`.

use serde::{Deserialize, Serialize};
use std::fmt;

// Default device base naming
pub const DEFAULT_DEVICE_BASE_NAME: &str = "dev1";
pub const DEFAULT_DEVICE_BASE_TAG: &str = "dev1";

// Sensor tags and configurations
pub const TAG_BRIGHTNESS: &str = "brightness";
pub const SENSOR_NAME_BRIGHTNESS: &str = "brightness";
pub const SENSOR_UNIT_FLUX: &str = "flux";
pub const SENSOR_TYPE_LDR: &str = "ldr";

// ============================================================================
// 1. Sensor Data Types
// ============================================================================

/// Sensor telemetry data types (`DataType`).
/// NLECloud values: 0: Integer, 1: Float, 2: Boolean, 3: String, 4: Enum, 5: Binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DataType {
    /// 32-bit Integer value
    Integer = 0,
    /// Single/Double precision floating point
    Float = 1,
    /// Boolean state (0 or 1)
    Boolean = 2,
    /// UTF-8 string value
    String = 3,
    /// Discrete enumerated text option
    Enum = 4,
    /// Base64 binary payload
    Binary = 5,
}

impl DataType {
    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

impl From<DataType> for u8 {
    fn from(dt: DataType) -> Self {
        dt as u8
    }
}

impl TryFrom<u8> for DataType {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Integer),
            1 => Ok(Self::Float),
            2 => Ok(Self::Boolean),
            3 => Ok(Self::String),
            4 => Ok(Self::Enum),
            5 => Ok(Self::Binary),
            other => Err(format!("Unknown DataType value: {other}")),
        }
    }
}

impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer => write!(f, "Integer"),
            Self::Float => write!(f, "Float"),
            Self::Boolean => write!(f, "Boolean"),
            Self::String => write!(f, "String"),
            Self::Enum => write!(f, "Enum"),
            Self::Binary => write!(f, "Binary"),
        }
    }
}

// ============================================================================
// 2. Sensor Transmission Types
// ============================================================================

/// Transmission behavior of a sensor/actuator (`TransType`).
/// NLECloud values: 0: Report only, 1: Report and control, 2: Alarm, 3: Fault.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TransType {
    /// Read-only telemetry reporting (Sensors)
    ReportOnly = 0,
    /// Bidirectional reporting and command control (Actuators)
    ReportAndControl = 1,
    /// Event-driven alarm signal
    Alarm = 2,
    /// Equipment fault reporting
    Fault = 3,
}

impl TransType {
    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

impl From<TransType> for u8 {
    fn from(tt: TransType) -> Self {
        tt as u8
    }
}

impl TryFrom<u8> for TransType {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::ReportOnly),
            1 => Ok(Self::ReportAndControl),
            2 => Ok(Self::Alarm),
            3 => Ok(Self::Fault),
            other => Err(format!("Unknown TransType value: {other}")),
        }
    }
}

impl fmt::Display for TransType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReportOnly => write!(f, "ReportOnly"),
            Self::ReportAndControl => write!(f, "ReportAndControl"),
            Self::Alarm => write!(f, "Alarm"),
            Self::Fault => write!(f, "Fault"),
        }
    }
}

// ============================================================================
// 3. Sensor Groups
// ============================================================================

/// Device sensor entity categorization (`Groups`).
/// NLECloud values: 1: Sensor, 2: Actuator, 3: Camera, 4: LED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SensorGroup {
    /// Sensor telemetry input
    Sensor = 1,
    /// Controllable actuator output
    Actuator = 2,
    /// IP Camera device
    Camera = 3,
    /// LED Matrix or Display
    Led = 4,
}

impl SensorGroup {
    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

impl From<SensorGroup> for u8 {
    fn from(g: SensorGroup) -> Self {
        g as u8
    }
}

impl TryFrom<u8> for SensorGroup {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::Sensor),
            2 => Ok(Self::Actuator),
            3 => Ok(Self::Camera),
            4 => Ok(Self::Led),
            other => Err(format!("Unknown SensorGroup value: {other}")),
        }
    }
}

impl fmt::Display for SensorGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sensor => write!(f, "Sensor"),
            Self::Actuator => write!(f, "Actuator"),
            Self::Camera => write!(f, "Camera"),
            Self::Led => write!(f, "Led"),
        }
    }
}

// ============================================================================
// 4. Sensor Protocols
// ============================================================================

/// Protocol used between device and sensor module (`Protocol`).
/// NLECloud values: 0: Unknown, 1: Modbus, 2: Zigbee, 3: TCP, 4: UDP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SensorProtocol {
    Unknown = 0,
    Modbus = 1,
    Zigbee = 2,
    Tcp = 3,
    Udp = 4,
}

impl SensorProtocol {
    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

impl From<SensorProtocol> for u8 {
    fn from(p: SensorProtocol) -> Self {
        p as u8
    }
}

impl TryFrom<u8> for SensorProtocol {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::Modbus),
            2 => Ok(Self::Zigbee),
            3 => Ok(Self::Tcp),
            4 => Ok(Self::Udp),
            other => Err(format!("Unknown SensorProtocol value: {other}")),
        }
    }
}

// ============================================================================
// 5. Actuator Operation Types
// ============================================================================

/// Operation style for actuators (`OperType`).
/// NLECloud values: 1: Switch (0/1), 2: Switch/Stop (0/1/2), 3: Button/Pulse, 4: Scale/Dimmer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ActuatorOperType {
    /// Binary switch (1: Open/On, 0: Close/Off)
    Switch = 1,
    /// Tri-state switch (1: Open, 0: Close, 2: Stop, e.g. motorized curtains)
    SwitchStop = 2,
    /// Momentary button or pulse action
    Button = 3,
    /// Continuous range / percentage / scale (e.g. dimmer, speed control)
    Scale = 4,
}

impl ActuatorOperType {
    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

impl From<ActuatorOperType> for u8 {
    fn from(ot: ActuatorOperType) -> Self {
        ot as u8
    }
}

impl TryFrom<u8> for ActuatorOperType {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::Switch),
            2 => Ok(Self::SwitchStop),
            3 => Ok(Self::Button),
            4 => Ok(Self::Scale),
            other => Err(format!("Unknown ActuatorOperType value: {other}")),
        }
    }
}

// ============================================================================
// 6. Device Communication Protocols
// ============================================================================

/// Device uplink communication protocol (`Protocol`).
/// NLECloud values: 1: TCP, 2: MQTT, 3: HTTP, 5: LWM2M, 6: CoAP, 7: TCP Transparent, 8: Modbus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DeviceProtocol {
    Tcp = 1,
    Mqtt = 2,
    Http = 3,
    Lwm2m = 5,
    Coap = 6,
    TcpTransparent = 7,
    Modbus = 8,
}

impl DeviceProtocol {
    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

impl From<DeviceProtocol> for u8 {
    fn from(p: DeviceProtocol) -> Self {
        p as u8
    }
}

impl TryFrom<u8> for DeviceProtocol {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::Tcp),
            2 => Ok(Self::Mqtt),
            3 => Ok(Self::Http),
            5 => Ok(Self::Lwm2m),
            6 => Ok(Self::Coap),
            7 => Ok(Self::TcpTransparent),
            8 => Ok(Self::Modbus),
            other => Err(format!("Unknown DeviceProtocol value: {other}")),
        }
    }
}

// ============================================================================
// 7. Project Industry Classifications
// ============================================================================

/// Project industry classification (`Industry`).
/// NLECloud values: 1: Security, 2: SmartHome, 3: Wearables, 4: Agriculture, 5: IndustrialIoT,
/// 6: NewEnergy, 7: OfficeBuilding, 8: SmartToys, 9: SmartCity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum IndustryKind {
    Security = 1,
    SmartHome = 2,
    Wearables = 3,
    Agriculture = 4,
    IndustrialIot = 5,
    NewEnergy = 6,
    OfficeBuilding = 7,
    SmartToys = 8,
    SmartCity = 9,
}

impl IndustryKind {
    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

impl From<IndustryKind> for u8 {
    fn from(ik: IndustryKind) -> Self {
        ik as u8
    }
}

impl TryFrom<u8> for IndustryKind {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::Security),
            2 => Ok(Self::SmartHome),
            3 => Ok(Self::Wearables),
            4 => Ok(Self::Agriculture),
            5 => Ok(Self::IndustrialIot),
            6 => Ok(Self::NewEnergy),
            7 => Ok(Self::OfficeBuilding),
            8 => Ok(Self::SmartToys),
            9 => Ok(Self::SmartCity),
            other => Err(format!("Unknown IndustryKind value: {other}")),
        }
    }
}

// ============================================================================
// 8. Project Network Connectivity Schemes
// ============================================================================

/// Project network connection method (`NetWorkKind`).
/// NLECloud values: 1: WiFi, 2: Ethernet, 3: Cellular, 4: Bluetooth, 5: NB-IoT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum NetworkKind {
    Wifi = 1,
    Ethernet = 2,
    Cellular = 3,
    Bluetooth = 4,
    NbIot = 5,
}

impl NetworkKind {
    pub const fn to_u8(self) -> u8 {
        self as u8
    }
}

impl From<NetworkKind> for u8 {
    fn from(nk: NetworkKind) -> Self {
        nk as u8
    }
}

impl TryFrom<u8> for NetworkKind {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::Wifi),
            2 => Ok(Self::Ethernet),
            3 => Ok(Self::Cellular),
            4 => Ok(Self::Bluetooth),
            5 => Ok(Self::NbIot),
            other => Err(format!("Unknown NetworkKind value: {other}")),
        }
    }
}

// ============================================================================
// 9. Historical Telemetry Query Methods
// ============================================================================

/// Query time-frame method for historical telemetry (`Method`).
/// NLECloud values: 1: Minutes, 2: Hours, 3: Days, 4: Weeks, 5: Months, 6: Date range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum TelemetryQueryMethod {
    PastMinutes = 1,
    PastHours = 2,
    PastDays = 3,
    PastWeeks = 4,
    PastMonths = 5,
    DateRange = 6,
}

impl TelemetryQueryMethod {
    pub const fn to_i32(self) -> i32 {
        self as i32
    }
}

impl From<TelemetryQueryMethod> for i32 {
    fn from(m: TelemetryQueryMethod) -> Self {
        m as i32
    }
}

impl TryFrom<i32> for TelemetryQueryMethod {
    type Error = String;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::PastMinutes),
            2 => Ok(Self::PastHours),
            3 => Ok(Self::PastDays),
            4 => Ok(Self::PastWeeks),
            5 => Ok(Self::PastMonths),
            6 => Ok(Self::DateRange),
            other => Err(format!("Unknown TelemetryQueryMethod value: {other}")),
        }
    }
}

// ============================================================================
// 10. Telemetry Aggregation Grouping Levels
// ============================================================================

/// Aggregation time-bucket level (`GroupBy`).
/// NLECloud values: 1: Minute, 2: Hour, 3: Day, 4: Month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum AggregationGroupBy {
    Minute = 1,
    Hour = 2,
    Day = 3,
    Month = 4,
}

impl AggregationGroupBy {
    pub const fn to_i32(self) -> i32 {
        self as i32
    }
}

impl From<AggregationGroupBy> for i32 {
    fn from(gb: AggregationGroupBy) -> Self {
        gb as i32
    }
}

impl TryFrom<i32> for AggregationGroupBy {
    type Error = String;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::Minute),
            2 => Ok(Self::Hour),
            3 => Ok(Self::Day),
            4 => Ok(Self::Month),
            other => Err(format!("Unknown AggregationGroupBy value: {other}")),
        }
    }
}

// ============================================================================
// 11. Automation Strategy Kinds & Status
// ============================================================================

/// Automation strategy action type (`Kind`).
/// NLECloud values: 1: Device Control, 2: Email Report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum StrategyKind {
    DeviceControl = 1,
    EmailNotification = 2,
}

impl From<StrategyKind> for u8 {
    fn from(sk: StrategyKind) -> Self {
        sk as u8
    }
}

/// Automation strategy enablement status (`Nullity`).
/// NLECloud values: 0: Enabled, 1: Disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum StrategyStatus {
    Enabled = 0,
    Disabled = 1,
}

impl From<StrategyStatus> for i32 {
    fn from(ss: StrategyStatus) -> Self {
        ss as i32
    }
}

// ============================================================================
// 12. Aggregation Functions & Sorting
// ============================================================================

/// Supported telemetry aggregation function (`Func`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AggregationFunction {
    Max,
    Min,
    Count,
}

impl AggregationFunction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Max => "MAX",
            Self::Min => "MIN",
            Self::Count => "COUNT",
        }
    }
}

impl fmt::Display for AggregationFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Telemetry query sort direction (`Sort`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl SortDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ascending => "ASC",
            Self::Descending => "DESC",
        }
    }
}

impl fmt::Display for SortDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enum_numeric_conversions() {
        assert_eq!(u8::from(DataType::Float), 1);
        assert_eq!(DataType::try_from(1).unwrap(), DataType::Float);

        assert_eq!(u8::from(TransType::ReportOnly), 0);
        assert_eq!(TransType::try_from(0).unwrap(), TransType::ReportOnly);

        assert_eq!(u8::from(SensorGroup::Actuator), 2);
        assert_eq!(SensorGroup::try_from(2).unwrap(), SensorGroup::Actuator);

        assert_eq!(u8::from(SensorProtocol::Modbus), 1);
        assert_eq!(SensorProtocol::try_from(1).unwrap(), SensorProtocol::Modbus);

        assert_eq!(u8::from(ActuatorOperType::Switch), 1);
        assert_eq!(ActuatorOperType::try_from(1).unwrap(), ActuatorOperType::Switch);

        assert_eq!(u8::from(DeviceProtocol::Tcp), 1);
        assert_eq!(DeviceProtocol::try_from(1).unwrap(), DeviceProtocol::Tcp);
        assert_eq!(u8::from(DeviceProtocol::Modbus), 8);

        assert_eq!(u8::from(IndustryKind::SmartHome), 2);
        assert_eq!(IndustryKind::try_from(2).unwrap(), IndustryKind::SmartHome);

        assert_eq!(u8::from(NetworkKind::Wifi), 1);
        assert_eq!(NetworkKind::try_from(1).unwrap(), NetworkKind::Wifi);

        assert_eq!(i32::from(TelemetryQueryMethod::DateRange), 6);
        assert_eq!(TelemetryQueryMethod::try_from(6).unwrap(), TelemetryQueryMethod::DateRange);

        assert_eq!(i32::from(AggregationGroupBy::Hour), 2);
        assert_eq!(AggregationGroupBy::try_from(2).unwrap(), AggregationGroupBy::Hour);
    }
}
