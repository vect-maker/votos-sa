use anyhow::{bail, Context, Result};
use nle_cloud_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Declarative schema for a device in seed/devices.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSeed {
    pub name: String,
    pub tag: String,
    #[serde(default = "default_tcp_protocol")]
    pub protocol: String,
    #[serde(default)]
    pub sensors: Vec<SensorSeed>,
    #[serde(default)]
    pub actuators: Vec<ActuatorSeed>,
}

fn default_tcp_protocol() -> String {
    "TCP".to_string()
}

impl DeviceSeed {
    pub fn protocol_kind(&self) -> DeviceProtocol {
        match self.protocol.to_uppercase().as_str() {
            "MQTT" => DeviceProtocol::Mqtt,
            "HTTP" => DeviceProtocol::Http,
            "LWM2M" => DeviceProtocol::Lwm2m,
            "COAP" => DeviceProtocol::Coap,
            "MODBUS" => DeviceProtocol::Modbus,
            _ => DeviceProtocol::Tcp,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("Device name cannot be empty");
        }
        if self.tag.trim().is_empty() {
            bail!("Device tag cannot be empty");
        }
        if self.name.len() > 15 {
            bail!(
                "Device base name '{}' exceeds 15 characters (NLECloud API limit)",
                self.name
            );
        }
        if self.tag.len() > 30 {
            bail!(
                "Device base tag '{}' exceeds 30 characters (NLECloud API limit)",
                self.tag
            );
        }
        Ok(())
    }
}

/// Declarative schema for a sensor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorSeed {
    pub name: String,
    pub api_tag: String,
    #[serde(default = "default_float_type")]
    pub data_type: String,
    #[serde(default = "default_report_only")]
    pub trans_type: String,
    pub type_attrs: Option<String>,
    pub unit: Option<String>,
    pub precision: Option<u8>,
}

fn default_float_type() -> String {
    "Float".to_string()
}

fn default_report_only() -> String {
    "ReportOnly".to_string()
}

impl SensorSeed {
    pub fn data_type_kind(&self) -> DataType {
        match self.data_type.to_lowercase().as_str() {
            "bool" | "boolean" => DataType::Boolean,
            "int" | "integer" => DataType::Integer,
            "str" | "string" => DataType::String,
            "bin" | "binary" => DataType::Binary,
            _ => DataType::Float,
        }
    }

    pub fn trans_type_kind(&self) -> TransType {
        match self.trans_type.to_lowercase().as_str() {
            "reportandcontrol" | "control" | "rw" => TransType::ReportAndControl,
            _ => TransType::ReportOnly,
        }
    }
}

/// Declarative schema for an actuator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorSeed {
    pub name: String,
    pub api_tag: String,
    #[serde(default = "default_float_type")]
    pub data_type: String,
    #[serde(default = "default_report_control")]
    pub trans_type: String,
    #[serde(default = "default_switch_oper")]
    pub oper_type: String,
    pub serial_number: Option<i32>,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

fn default_report_control() -> String {
    "ReportAndControl".to_string()
}

fn default_switch_oper() -> String {
    "Switch".to_string()
}

impl ActuatorSeed {
    pub fn data_type_kind(&self) -> DataType {
        match self.data_type.to_lowercase().as_str() {
            "bool" | "boolean" => DataType::Boolean,
            "int" | "integer" => DataType::Integer,
            "str" | "string" => DataType::String,
            _ => DataType::Float,
        }
    }

    pub fn trans_type_kind(&self) -> TransType {
        TransType::ReportAndControl
    }

    pub fn oper_type_kind(&self) -> ActuatorOperType {
        match self.oper_type.to_lowercase().as_str() {
            "scale" => ActuatorOperType::Scale,
            "button" => ActuatorOperType::Button,
            "switchstop" | "switch_stop" => ActuatorOperType::SwitchStop,
            _ => ActuatorOperType::Switch,
        }
    }
}

/// Loads and validates device definitions from the target JSON file.
pub fn load_devices_from_file(path: &Path) -> Result<Vec<DeviceSeed>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read seed devices file at '{}'", path.display()))?;

    let devices: Vec<DeviceSeed> = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON seed from '{}'", path.display()))?;

    for dev in &devices {
        dev.validate()?;
    }

    Ok(devices)
}
