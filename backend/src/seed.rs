use anyhow::{bail, Context, Result};
use nle_cloud_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Declarative specification of a reusable peripheral set (sensors and actuators).
/// In the peripheral spec JSON, the key in the top-level dictionary is the unique set name.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct PeripheralSetSeed {
    #[serde(default)]
    pub sensors: Vec<SensorSeed>,
    #[serde(default)]
    pub actuators: Vec<ActuatorSeed>,
}

/// Raw schema as deserialized from seed/devices.json.
/// A device can reference a reusable peripheral set by name, and/or define inline sensors/actuators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDeviceSeed {
    pub name: String,
    pub tag: String,
    #[serde(default = "default_tcp_protocol")]
    pub protocol: String,
    /// Unique name of the peripheral set defined in the peripherals spec JSON
    #[serde(
        default,
        alias = "peripheral_set_name",
        alias = "peripherals",
        alias = "peripheral_spec"
    )]
    pub peripheral_set: Option<String>,
    #[serde(default)]
    pub sensors: Vec<SensorSeed>,
    #[serde(default)]
    pub actuators: Vec<ActuatorSeed>,
}

/// Fully resolved declarative device definition with all peripherals populated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceSeed {
    pub name: String,
    pub tag: String,
    #[serde(default = "default_tcp_protocol")]
    pub protocol: String,
    #[serde(default)]
    pub peripheral_set: Option<String>,
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Loads the peripheral specifications dictionary from a JSON file.
/// The dictionary key is the unique peripheral set name.
pub fn load_peripherals_from_file(path: &Path) -> Result<HashMap<String, PeripheralSetSeed>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read seed peripherals file at '{}'", path.display()))?;

    let map: HashMap<String, PeripheralSetSeed> = serde_json::from_str(&content).with_context(
        || format!("Failed to parse JSON peripherals specification from '{}'", path.display()),
    )?;

    Ok(map)
}

/// Loads and validates device definitions from devices JSON and peripheral specs JSON.
/// Each device can reference a peripheral set by name to automatically inherit its sensors and actuators.
pub fn load_devices_from_files(
    devices_path: &Path,
    peripherals_path: Option<&Path>,
) -> Result<Vec<DeviceSeed>> {
    let devices_content = fs::read_to_string(devices_path)
        .with_context(|| format!("Failed to read seed devices file at '{}'", devices_path.display()))?;

    let raw_devices: Vec<RawDeviceSeed> = serde_json::from_str(&devices_content)
        .with_context(|| format!("Failed to parse JSON seed devices from '{}'", devices_path.display()))?;

    // Load peripheral specs if a path was given or if peripherals.json exists alongside devices.json
    let peripherals_map = match peripherals_path {
        Some(p) if p.exists() => load_peripherals_from_file(p)?,
        _ => {
            let default_peer = devices_path.parent().map(|d| d.join("peripherals.json"));
            if let Some(ref peer) = default_peer {
                if peer.exists() {
                    load_peripherals_from_file(peer)?
                } else {
                    HashMap::new()
                }
            } else {
                HashMap::new()
            }
        }
    };

    let mut devices = Vec::with_capacity(raw_devices.len());

    for raw in raw_devices {
        let mut sensors = Vec::new();
        let mut actuators = Vec::new();

        if let Some(ref set_name) = raw.peripheral_set {
            if let Some(spec) = peripherals_map.get(set_name) {
                sensors.extend(spec.sensors.clone());
                actuators.extend(spec.actuators.clone());
            } else {
                let mut available_keys: Vec<&String> = peripherals_map.keys().collect();
                available_keys.sort();
                bail!(
                    "Device '{}' references unknown peripheral_set '{}'. Available sets in peripherals JSON: {:?}",
                    raw.name,
                    set_name,
                    available_keys
                );
            }
        }

        // Allow inline overrides or extra sensors/actuators if specified directly on the device
        sensors.extend(raw.sensors);
        actuators.extend(raw.actuators);

        let dev = DeviceSeed {
            name: raw.name,
            tag: raw.tag,
            protocol: raw.protocol,
            peripheral_set: raw.peripheral_set,
            sensors,
            actuators,
        };

        dev.validate()?;
        devices.push(dev);
    }

    Ok(devices)
}

/// Loads and validates device definitions from the target JSON file,
/// automatically resolving any peer `peripherals.json` in the same directory.
pub fn load_devices_from_file(path: &Path) -> Result<Vec<DeviceSeed>> {
    load_devices_from_files(path, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(1);

    fn create_test_dir(name: &str) -> std::path::PathBuf {
        let count = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("votos_seed_{name}_{}_{count}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("Failed to create test temp dir");
        path
    }

    #[test]
    fn test_load_devices_with_peripheral_sets() -> Result<()> {
        let temp_dir = create_test_dir("periph_sets");
        let periph_path = temp_dir.join("peripherals.json");
        let devices_path = temp_dir.join("devices.json");

        let periph_json = r#"{
            "station_kit": {
                "sensors": [
                    { "name": "brightness", "api_tag": "brightness", "data_type": "Float" }
                ],
                "actuators": [
                    { "name": "lamp", "api_tag": "lamp", "oper_type": "Switch" },
                    { "name": "servo_home", "api_tag": "servo_home", "oper_type": "Button" }
                ]
            }
        }"#;

        let devices_json = r#"[
            {
                "name": "dev1",
                "tag": "dev1",
                "protocol": "TCP",
                "peripheral_set": "station_kit"
            },
            {
                "name": "dev2",
                "tag": "dev2",
                "peripheral_set": "station_kit"
            }
        ]"#;

        let mut f1 = fs::File::create(&periph_path)?;
        f1.write_all(periph_json.as_bytes())?;

        let mut f2 = fs::File::create(&devices_path)?;
        f2.write_all(devices_json.as_bytes())?;

        let devices = load_devices_from_files(&devices_path, Some(&periph_path))?;
        assert_eq!(devices.len(), 2);

        for dev in &devices {
            assert_eq!(dev.sensors.len(), 1);
            assert_eq!(dev.sensors[0].api_tag, "brightness");
            assert_eq!(dev.actuators.len(), 2);
            assert_eq!(dev.actuators[0].api_tag, "lamp");
            assert_eq!(dev.actuators[1].api_tag, "servo_home");
            assert_eq!(dev.actuators[1].oper_type_kind(), ActuatorOperType::Button);
        }

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }

    #[test]
    fn test_unknown_peripheral_set_errors() -> Result<()> {
        let temp_dir = create_test_dir("unknown_periph");
        let periph_path = temp_dir.join("peripherals.json");
        let devices_path = temp_dir.join("devices.json");

        fs::write(&periph_path, "{}")?;
        fs::write(
            &devices_path,
            r#"[{"name": "d1", "tag": "d1", "peripheral_set": "nonexistent"}]"#,
        )?;

        let result = load_devices_from_files(&devices_path, Some(&periph_path));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("unknown peripheral_set 'nonexistent'"));

        let _ = fs::remove_dir_all(&temp_dir);
        Ok(())
    }

    #[test]
    fn test_load_actual_seed_files() -> Result<()> {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let devices_path = repo_root.join("seed/devices.json");
        let periph_path = repo_root.join("seed/peripherals.json");

        if devices_path.exists() && periph_path.exists() {
            let devices = load_devices_from_files(&devices_path, Some(&periph_path))?;
            assert_eq!(devices.len(), 2);
            for d in &devices {
                assert_eq!(d.sensors.len(), 1);
                assert_eq!(d.actuators.len(), 7);
                assert!(d.actuators.iter().any(|a| a.api_tag == "servo_home"));
                assert!(d.actuators.iter().any(|a| a.api_tag == "calibrate_light"));
            }
        }
        Ok(())
    }
}
