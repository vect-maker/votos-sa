use nle_cloud_sdk::models::{DeviceQueryParams, ProjectQueryParams};
use nle_cloud_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use anyhow::{anyhow, bail, Result as AnyResult};
use crate::constants::{TAG_SERVO_HOME, TAG_SERVO_X, TAG_SERVO_Y};

/// Simplified in-memory snapshot of a device.
/// Only keeps the most critical data for frontend consumption.
/// If the device is offline or data cannot be fetched, `sensors` is `None` (`null` in JSON).
/// The online status is cleanly inferred from `sensors != null`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    pub device_id: i32,
    pub name: String,
    pub tag: String,
    pub sensors: Option<HashMap<String, serde_json::Value>>,
}

impl DeviceSnapshot {
    /// Inferred online status based on whether sensors telemetry is available.
    pub fn is_online(&self) -> bool {
        self.sensors.is_some()
    }
}

/// Checks loose equivalence between two JSON values in an IoT actuator/sensor context
/// (e.g., bool `true` == number `1`, `"1"` == `1`, `90.0` == `90`).
pub fn values_are_equivalent(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    if a == b {
        return true;
    }
    match (a, b) {
        // Bool vs Number: true == 1, false == 0
        (serde_json::Value::Bool(b_val), serde_json::Value::Number(num))
        | (serde_json::Value::Number(num), serde_json::Value::Bool(b_val)) => {
            if let Some(i) = num.as_i64() {
                (*b_val && i != 0) || (!*b_val && i == 0)
            } else if let Some(f) = num.as_f64() {
                (*b_val && f.abs() > f64::EPSILON) || (!*b_val && f.abs() <= f64::EPSILON)
            } else {
                false
            }
        }
        // Number vs Number (floating point tolerance / integer representation)
        (serde_json::Value::Number(n1), serde_json::Value::Number(n2)) => {
            if let (Some(i1), Some(i2)) = (n1.as_i64(), n2.as_i64()) {
                i1 == i2
            } else if let (Some(f1), Some(f2)) = (n1.as_f64(), n2.as_f64()) {
                (f1 - f2).abs() < 1e-3
            } else {
                false
            }
        }
        // String vs Bool / Number
        (serde_json::Value::String(s), other) | (other, serde_json::Value::String(s)) => {
            let s_trimmed = s.trim().to_lowercase();
            match other {
                serde_json::Value::Bool(b_val) => {
                    if *b_val {
                        s_trimmed == "true" || s_trimmed == "1" || s_trimmed == "on"
                    } else {
                        s_trimmed == "false" || s_trimmed == "0" || s_trimmed == "off"
                    }
                }
                serde_json::Value::Number(num) => {
                    if let Ok(parsed_f) = s_trimmed.parse::<f64>() {
                        if let Some(num_f) = num.as_f64() {
                            (parsed_f - num_f).abs() < 1e-3
                        } else {
                            false
                        }
                    } else if s_trimmed == "true" || s_trimmed == "on" {
                        num.as_f64().map(|f| f.abs() > f64::EPSILON).unwrap_or(false)
                    } else if s_trimmed == "false" || s_trimmed == "off" {
                        num.as_f64().map(|f| f.abs() <= f64::EPSILON).unwrap_or(false)
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        _ => false,
    }
}

/// Represents an in-flight actuator command awaiting cloud/hardware state reconciliation.
#[derive(Debug, Clone)]
pub struct PendingCommand {
    pub value: serde_json::Value,
    pub sent_at: std::time::Instant,
    pub ttl: std::time::Duration,
}

/// Device abstraction that maintains cached identity and state,
/// capable of polling its own data from NLECloud.
#[derive(Debug)]
pub struct DeviceNode {
    pub device_id: i32,
    pub project_id: i32,
    pub name: String,
    pub tag: String,
    pub state: RwLock<DeviceSnapshot>,
    pub pending_commands: RwLock<HashMap<String, PendingCommand>>,
}

impl DeviceNode {
    pub fn new(device_id: i32, project_id: i32, name: String, tag: String) -> Self {
        Self {
            device_id,
            project_id,
            name: name.clone(),
            tag: tag.clone(),
            state: RwLock::new(DeviceSnapshot {
                device_id,
                name,
                tag,
                sensors: None,
            }),
            pending_commands: RwLock::new(HashMap::new()),
        }
    }

    /// Reconciles freshly polled cloud sensor data with active pending commands.
    /// If cloud telemetry matches a pending command's target value, the pending command is cleared.
    /// If cloud telemetry still has the old value, but the command is within its TTL grace period,
    /// the pending target value overrides the stale cloud value so the local state does not revert.
    /// If the TTL has expired without cloud confirmation, the pending command is discarded.
    pub async fn reconcile_sensors(
        &self,
        mut cloud_sensors: HashMap<String, serde_json::Value>,
    ) -> HashMap<String, serde_json::Value> {
        let mut pending_guard = self.pending_commands.write().await;
        pending_guard.retain(|tag, pending| {
            if let Some(cloud_val) = cloud_sensors.get(tag) {
                if values_are_equivalent(cloud_val, &pending.value) {
                    tracing::debug!(
                        device_id = self.device_id,
                        tag = %tag,
                        cloud_val = ?cloud_val,
                        "Hardware/cloud confirmed pending command"
                    );
                    return false; // Confirmed by cloud telemetry!
                }
            }

            if pending.sent_at.elapsed() < pending.ttl {
                // Cloud is still reporting stale pre-command state; preserve target value
                cloud_sensors.insert(tag.clone(), pending.value.clone());
                true // Retain pending command
            } else {
                tracing::warn!(
                    device_id = self.device_id,
                    tag = %tag,
                    "Pending command expired without cloud confirmation; accepting cloud state"
                );
                false // Evict expired command
            }
        });

        cloud_sensors
    }

    /// Polls latest telemetry and status for this specific device.
    /// Returns true if any value changed.
    pub async fn poll_data(&self, client: &NleCloudClient) -> Result<bool> {
        // 1. Query online status
        let dev_id_str = self.device_id.to_string();
        let is_online = match client.get_devices_status(&dev_id_str, None).await {
            Ok(status_list) => status_list.first().map(|s| s.is_online).unwrap_or(false),
            Err(e) => {
                tracing::warn!("Failed to fetch status for device {}: {e}", self.device_id);
                false
            }
        };

        // 2. If online, query sensor and actuator values. If offline or failed, sensors is None.
        let new_sensors = if is_online {
            match client
                .get_project_sensors_realtime(self.project_id, Some(self.device_id), None)
                .await
            {
                Ok(realtime_items) => {
                    let mut map = HashMap::new();
                    for item in realtime_items {
                        if let Some(tag) = item.get("ApiTag").and_then(|v| v.as_str()) {
                            if let Some(val) = item.get("Value") {
                                map.insert(tag.to_string(), val.clone());
                            }
                        }
                    }
                    Some(self.reconcile_sensors(map).await)
                }
                Err(e) => {
                    tracing::warn!("Could not fetch sensors for device {}: {e}", self.device_id);
                    None
                }
            }
        } else {
            self.pending_commands.write().await.clear();
            None
        };

        let mut write_guard = self.state.write().await;
        let changed = write_guard.sensors != new_sensors;
        write_guard.sensors = new_sensors;

        Ok(changed)
    }

    /// Checks whether this device is currently online.
    pub async fn is_online(&self) -> bool {
        self.state.read().await.is_online()
    }

    /// Returns a snapshot of the current device state.
    pub async fn snapshot(&self) -> DeviceSnapshot {
        self.state.read().await.clone()
    }
}

/// Simplified in-memory snapshot of the project and its devices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub project_id: i32,
    pub name: String,
    pub tag: Option<String>,
    pub devices: Vec<DeviceSnapshot>,
}

/// Project abstraction that discovers and caches project/device metadata once by name,
/// and coordinates subsequent polling directly by real IDs.
#[derive(Debug)]
pub struct ProjectManager {
    pub project_id: i32,
    pub name: String,
    pub tag: Option<String>,
    pub devices: Vec<Arc<DeviceNode>>,
    pub command_ttl: std::time::Duration,
}

impl ProjectManager {
    /// Discovers project by name on NLECloud once at startup, caching real IDs and tags,
    /// and instantiates DeviceNode instances for matched devices.
    pub async fn init_from_cloud(
        client: &NleCloudClient,
        project_name: &str,
        _device_namespace: Option<&str>,
        command_ttl: std::time::Duration,
    ) -> AnyResult<Self> {
        tracing::info!("Discovering project '{project_name}' on NLECloud...");

        // 1. Query project once by name to cache real project ID & tag
        let query = ProjectQueryParams::builder()
            .keyword(project_name)
            .page_size(100)
            .build();

        let prj_paged = client.get_projects(&query, None).await?;
        let prj = prj_paged
            .page_set
            .into_iter()
            .find(|p| p.name.as_deref() == Some(project_name))
            .ok_or_else(|| {
                anyhow!(
                    "Project '{project_name}' not found on NLECloud. Please run `just provision` first."
                )
            })?;

        let project_id = prj.project_id;
        let project_tag = prj.project_tag;
        tracing::info!(
            "Cached project in memory: '{}' (ID: {project_id}, Tag: {:?})",
            project_name,
            project_tag
        );

        // 2. Query devices for this project and instantiate DeviceNode
        tracing::info!("Fetching devices for project ID {project_id}...");
        let dev_query = DeviceQueryParams::builder()
            .project_key_word(project_id.to_string())
            .page_size(100)
            .build();

        let dev_paged = client.get_devices(&dev_query, None).await?;

        let mut devices = Vec::new();
        for dev in dev_paged.page_set {
            let dev_name = dev.name.as_deref().unwrap_or_default();
            let dev_tag = dev.tag.as_deref().unwrap_or_default();

            tracing::info!(
                "Instantiated device in memory: '{}' (ID: {}, Tag: '{}')",
                dev_name,
                dev.device_id,
                dev_tag
            );
            let node = Arc::new(DeviceNode::new(
                dev.device_id,
                project_id,
                dev_name.to_string(),
                dev_tag.to_string(),
            ));

            devices.push(node);
        }

        if devices.is_empty() {
            tracing::warn!(
                "No matching devices found in project {project_id}. Make sure `just provision` has run."
            );
        }

        let manager = Self {
            project_id,
            name: project_name.to_string(),
            tag: project_tag,
            devices,
            command_ttl,
        };

        // Populate initial telemetry and status for all devices
        let _ = manager.poll_all(client).await;

        Ok(manager)
    }

    /// Polls all devices in the project via efficient batch endpoints.
    /// Returns true if any state changed.
    pub async fn poll_all(&self, client: &NleCloudClient) -> bool {
        if self.devices.is_empty() {
            return false;
        }

        // 1. Batch query online statuses for all devices
        let dev_ids_str = self
            .devices
            .iter()
            .map(|d| d.device_id.to_string())
            .collect::<Vec<_>>()
            .join(",");

        let status_map: HashMap<i32, bool> = match client.get_devices_status(&dev_ids_str, None).await {
            Ok(list) => list.into_iter().map(|s| (s.device_id, s.is_online)).collect(),
            Err(e) => {
                tracing::warn!("Failed to query device statuses: {e}");
                HashMap::new()
            }
        };

        // 2. Query all real-time sensor/actuator values for this project in one call
        let realtime_items = match client
            .get_project_sensors_realtime(self.project_id, None, None)
            .await
        {
            Ok(items) => items,
            Err(e) => {
                tracing::warn!("Failed to query project sensors realtime data: {e}");
                Vec::new()
            }
        };

        // Group realtime items by device_id
        let mut device_telemetry: HashMap<i32, Vec<(String, serde_json::Value)>> = HashMap::new();
        for item in realtime_items {
            let dev_id = item.get("DeviceId").and_then(|v| v.as_i64()).map(|v| v as i32);
            let api_tag = item.get("ApiTag").and_then(|v| v.as_str());
            let val = item.get("Value");

            if let (Some(did), Some(tag), Some(v)) = (dev_id, api_tag, val) {
                device_telemetry
                    .entry(did)
                    .or_default()
                    .push((tag.to_string(), v.clone()));
            }
        }

        let mut any_changed = false;
        for dev in &self.devices {
            let online = status_map.get(&dev.device_id).copied().unwrap_or(false);

            let new_sensors = if online {
                let map = if let Some(telemetry) = device_telemetry.get(&dev.device_id) {
                    let mut m = HashMap::new();
                    for (tag, val) in telemetry {
                        m.insert(tag.clone(), val.clone());
                    }
                    m
                } else {
                    HashMap::new()
                };

                Some(dev.reconcile_sensors(map).await)
            } else {
                dev.pending_commands.write().await.clear();
                None
            };

            let mut write_guard = dev.state.write().await;
            if write_guard.sensors != new_sensors {
                write_guard.sensors = new_sensors;
                any_changed = true;
            }
        }

        any_changed
    }

    /// Sends a command to an actuator on a device and updates local state.
    pub async fn control_actuator(
        &self,
        client: &NleCloudClient,
        device_id: Option<i32>,
        api_tag: &str,
        value: &serde_json::Value,
    ) -> AnyResult<(i32, serde_json::Value)> {
        let dev = if let Some(id) = device_id {
            self.devices.iter().find(|d| d.device_id == id)
        } else {
            self.devices.first()
        }
        .ok_or_else(|| anyhow!("No target device available to send command"))?;

        // Check if device is offline before attempting command
        if !dev.is_online().await {
            bail!(
                "Device '{}' (ID: {}) is currently offline. Power on the device to control actuators.",
                dev.name, dev.device_id
            );
        }

        // Format switch booleans to 1 / 0 if appropriate for NLECloud actuators
        let send_val = match value {
            serde_json::Value::Bool(b) => {
                serde_json::Value::Number(if *b { 1.into() } else { 0.into() })
            }
            other => other.clone(),
        };

        client
            .send_cmd(dev.device_id, api_tag, &send_val, None)
            .await?;

        // 1. Record pending command with settling TTL to prevent polling from reverting state
        {
            let mut pending_guard = dev.pending_commands.write().await;
            pending_guard.insert(
                api_tag.to_string(),
                PendingCommand {
                    value: send_val.clone(),
                    sent_at: std::time::Instant::now(),
                    ttl: self.command_ttl,
                },
            );
            if api_tag == TAG_SERVO_HOME {
                pending_guard.insert(
                    TAG_SERVO_X.to_string(),
                    PendingCommand {
                        value: serde_json::json!(90),
                        sent_at: std::time::Instant::now(),
                        ttl: self.command_ttl,
                    },
                );
                pending_guard.insert(
                    TAG_SERVO_Y.to_string(),
                    PendingCommand {
                        value: serde_json::json!(90),
                        sent_at: std::time::Instant::now(),
                        ttl: self.command_ttl,
                    },
                );
            }
        }

        // 2. Update local state immediately so snapshot reflects it
        {
            let mut write_guard = dev.state.write().await;
            match write_guard.sensors {
                Some(ref mut map) => {
                    map.insert(api_tag.to_string(), send_val.clone());
                    if api_tag == TAG_SERVO_HOME {
                        map.insert(TAG_SERVO_X.to_string(), serde_json::json!(90));
                        map.insert(TAG_SERVO_Y.to_string(), serde_json::json!(90));
                    }
                }
                None => {
                    let mut map = HashMap::new();
                    map.insert(api_tag.to_string(), send_val.clone());
                    if api_tag == TAG_SERVO_HOME {
                        map.insert(TAG_SERVO_X.to_string(), serde_json::json!(90));
                        map.insert(TAG_SERVO_Y.to_string(), serde_json::json!(90));
                    }
                    write_guard.sensors = Some(map);
                }
            }
        }

        Ok((dev.device_id, send_val))
    }

    /// Returns a list of all device snapshots.
    pub async fn get_device_snapshots(&self) -> Vec<DeviceSnapshot> {
        let mut list = Vec::with_capacity(self.devices.len());
        for dev in &self.devices {
            list.push(dev.snapshot().await);
        }
        list
    }

    /// Returns a specific device snapshot by ID.
    pub async fn get_device_snapshot(&self, device_id: i32) -> Option<DeviceSnapshot> {
        for dev in &self.devices {
            if dev.device_id == device_id {
                return Some(dev.snapshot().await);
            }
        }
        None
    }

    /// Returns a full snapshot of the project and its devices for serialization.
    pub async fn snapshot(&self) -> ProjectSnapshot {
        let mut device_snapshots = Vec::new();
        for dev in &self.devices {
            device_snapshots.push(dev.snapshot().await);
        }

        ProjectSnapshot {
            project_id: self.project_id,
            name: self.name.clone(),
            tag: self.tag.clone(),
            devices: device_snapshots,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_values_are_equivalent() {
        assert!(values_are_equivalent(&serde_json::json!(1), &serde_json::json!(1)));
        assert!(values_are_equivalent(&serde_json::json!(1), &serde_json::json!(1.0)));
        assert!(values_are_equivalent(&serde_json::json!(1), &serde_json::json!(true)));
        assert!(values_are_equivalent(&serde_json::json!(0), &serde_json::json!(false)));
        assert!(values_are_equivalent(&serde_json::json!("1"), &serde_json::json!(1)));
        assert!(values_are_equivalent(&serde_json::json!("true"), &serde_json::json!(1)));
        assert!(values_are_equivalent(&serde_json::json!(90), &serde_json::json!(90.0)));
        assert!(!values_are_equivalent(&serde_json::json!(0), &serde_json::json!(1)));
        assert!(!values_are_equivalent(&serde_json::json!(90), &serde_json::json!(45)));
    }

    #[tokio::test]
    async fn test_pending_command_reconciliation() {
        let dev = DeviceNode::new(1, 10, "Dev".into(), "tag".into());

        // Register pending command for "lamp" -> 1
        {
            let mut pending = dev.pending_commands.write().await;
            pending.insert(
                "lamp".into(),
                PendingCommand {
                    value: serde_json::json!(1),
                    sent_at: std::time::Instant::now(),
                    ttl: std::time::Duration::from_secs(5),
                },
            );
        }

        // 1. Stale cloud poll returns lamp = 0
        let mut cloud_data = HashMap::new();
        cloud_data.insert("lamp".into(), serde_json::json!(0));
        cloud_data.insert("brightness".into(), serde_json::json!(250));

        let reconciled = dev.reconcile_sensors(cloud_data).await;
        // Should preserve lamp = 1 because within TTL
        assert_eq!(reconciled.get("lamp"), Some(&serde_json::json!(1)));
        assert_eq!(reconciled.get("brightness"), Some(&serde_json::json!(250)));
        // Pending command still active
        assert!(dev.pending_commands.read().await.contains_key("lamp"));

        // 2. Now cloud catches up and reports lamp = 1
        let mut confirmed_data = HashMap::new();
        confirmed_data.insert("lamp".into(), serde_json::json!(1));
        let reconciled2 = dev.reconcile_sensors(confirmed_data).await;
        assert_eq!(reconciled2.get("lamp"), Some(&serde_json::json!(1)));
        // Pending command was cleared because cloud confirmed
        assert!(!dev.pending_commands.read().await.contains_key("lamp"));
    }
}

