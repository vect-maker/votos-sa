//! Project constants for NLECloud devices, sensor tags, and actuators.
//!
//! Strongly-typed enums (DataType, TransType, DeviceProtocol, IndustryKind, NetworkKind, etc.)
//! are now provided directly by `nle_cloud_sdk` and re-exported here for convenience.

pub use nle_cloud_sdk::prelude::*;

// Default device base naming
pub const DEFAULT_DEVICE_BASE_NAME: &str = "dev1";
pub const DEFAULT_DEVICE_BASE_TAG: &str = "dev1";

// Sensor tags and configurations
pub const TAG_BRIGHTNESS: &str = "brightness";
pub const SENSOR_NAME_BRIGHTNESS: &str = "brightness";
pub const SENSOR_UNIT_FLUX: &str = "flux";
pub const SENSOR_TYPE_LDR: &str = "ldr";

// Actuator tags and configurations for servos
pub const TAG_SERVO_X: &str = "servo_x";
pub const ACTUATOR_NAME_SERVO_X: &str = "servo_x";

pub const TAG_SERVO_Y: &str = "servo_y";
pub const ACTUATOR_NAME_SERVO_Y: &str = "servo_y";

pub const SERVO_MAX_ANGLE: u16 = 180;
pub const SERVO_UNIT_DEGREE: &str = "deg";

// Boolean switch actuators
pub const TAG_LAMP: &str = "lamp";
pub const ACTUATOR_NAME_LAMP: &str = "lamp";

pub const TAG_FAN: &str = "fan";
pub const ACTUATOR_NAME_FAN: &str = "fan";

pub const TAG_LOCK: &str = "lock";
pub const ACTUATOR_NAME_LOCK: &str = "lock";
