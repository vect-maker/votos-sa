//! Project constants for NLECloud devices, sensor tags, and actuators.
//!
//! Strongly-typed enums (DataType, TransType, DeviceProtocol, IndustryKind, NetworkKind, etc.)
//! are provided directly by `nle_cloud_sdk` and re-exported here for convenience.

pub use nle_cloud_sdk::prelude::*;

/// Default project name on NLECloud
pub const DEFAULT_PROJECT_NAME: &str = "smart-home-dock";

/// Peripheral API tags matching declarative definitions and TCP telemetry frames
pub const TAG_BRIGHTNESS: &str = "brightness";
pub const TAG_SERVO_X: &str = "servo_x";
pub const TAG_SERVO_Y: &str = "servo_y";
pub const TAG_LAMP: &str = "lamp";
pub const TAG_FAN: &str = "fan";
pub const TAG_LOCK: &str = "lock";

/// Maximum physical servo rotation limit in degrees
pub const SERVO_MAX_ANGLE: u16 = 180;
