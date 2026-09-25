//! System constants and strongly typed enums for NLECloud devices, sensor tags, and actuators.

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

/// Sensor telemetry data type in NLECloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DataType {
    Integer = 0,
    Float = 1,
    Boolean = 2,
    String = 3,
    Enum = 4,
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
            other => Err(format!("Unknown DataType: {other}")),
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

/// Transmission type for sensors and actuators in NLECloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TransType {
    ReportOnly = 0,
    ReportAndControl = 1,
    Alarm = 2,
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
            other => Err(format!("Unknown TransType: {other}")),
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

/// Device communication protocol in NLECloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DeviceProtocol {
    Tcp = 1,
    Mqtt = 2,
    Http = 3,
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
            other => Err(format!("Unknown DeviceProtocol: {other}")),
        }
    }
}

/// Actuator operation type in NLECloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ActuatorOperType {
    Switch = 1,
    SwitchStop = 2,
    Button = 3,
    Scale = 4,
}

impl From<ActuatorOperType> for u8 {
    fn from(ot: ActuatorOperType) -> Self {
        ot as u8
    }
}

/// Project Industry category in NLECloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum IndustryKind {
    Security = 1,
    SmartHome = 2,
    Wearable = 3,
    Agriculture = 4,
    IndustrialIot = 5,
}

impl From<IndustryKind> for u8 {
    fn from(ik: IndustryKind) -> Self {
        ik as u8
    }
}

/// Project Network connectivity type in NLECloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum NetworkKind {
    Wifi = 1,
    Ethernet = 2,
    Cellular = 3,
    Bluetooth = 4,
    NbIot = 5,
}

impl From<NetworkKind> for u8 {
    fn from(nk: NetworkKind) -> Self {
        nk as u8
    }
}
