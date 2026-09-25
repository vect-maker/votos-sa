use nle_cloud_sdk::models::{DeviceQueryParams, ProjectQueryParams};
use nle_cloud_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

type AnyResult<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Simplified in-memory snapshot of a device.
/// Only keeps the most critical data for frontend consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSnapshot {
    pub device_id: i32,
    pub name: String,
    pub tag: String,
    pub is_online: bool,
    pub sensors: HashMap<String, serde_json::Value>,
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
                is_online: false,
                sensors: HashMap::new(),
            }),
        }
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

        // 2. Query sensor and actuator values via project real-time data endpoint
        let realtime_items = client
            .get_project_sensors_realtime(self.project_id, Some(self.device_id), None)
            .await?;

        let mut write_guard = self.state.write().await;
        let mut changed = write_guard.is_online != is_online;
        write_guard.is_online = is_online;

        for item in realtime_items {
            if let Some(tag) = item.get("ApiTag").and_then(|v| v.as_str()) {
                if let Some(val) = item.get("Value") {
                    let previous = write_guard.sensors.insert(tag.to_string(), val.clone());
                    if previous.as_ref() != Some(val) {
                        changed = true;
                    }
                }
            }
        }

        Ok(changed)
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
}

impl ProjectManager {
    /// Discovers project by name on NLECloud once at startup, caching real IDs and tags,
    /// and instantiates DeviceNode instances for matched devices.
    pub async fn init_from_cloud(
        client: &NleCloudClient,
        project_name: &str,
        device_namespace: Option<&str>,
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
                Box::<dyn std::error::Error + Send + Sync>::from(format!(
                    "Project '{project_name}' not found on NLECloud. Please run `just provision` first."
                ))
            })?;

        let project_id = prj.project_id;
        let project_tag = prj.project_tag;
        tracing::info!(
            "Cached project in memory: '{}' (ID: {project_id}, Tag: {:?})",
            project_name,
            project_tag
        );

        // 2. Resolve expected device name and tag
        let (expected_dev_name, expected_dev_tag) = crate::project::resolve_device_name_and_tag(
            device_namespace,
            project_id,
            crate::constants::DEFAULT_DEVICE_BASE_NAME,
        );

        // 3. Query devices for this project and instantiate DeviceNode
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

            // Match device by expected name/tag or project ID
            if dev_name == expected_dev_name || dev_tag == expected_dev_tag || dev.project_id == Some(project_id) {
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
            let mut write_guard = dev.state.write().await;
            if let Some(&online) = status_map.get(&dev.device_id) {
                if write_guard.is_online != online {
                    write_guard.is_online = online;
                    any_changed = true;
                }
            }

            if let Some(telemetry) = device_telemetry.get(&dev.device_id) {
                for (tag, val) in telemetry {
                    let previous = write_guard.sensors.insert(tag.clone(), val.clone());
                    if previous.as_ref() != Some(val) {
                        any_changed = true;
                    }
                }
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
        .ok_or_else(|| {
            Box::<dyn std::error::Error + Send + Sync>::from(
                "No target device available to send command",
            )
        })?;

        // Check if device is offline before attempting command
        let is_online = dev.state.read().await.is_online;
        if !is_online {
            return Err(Box::<dyn std::error::Error + Send + Sync>::from(format!(
                "Device '{}' (ID: {}) is currently offline. Power on the device to control actuators.",
                dev.name, dev.device_id
            )));
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

        // Update local state immediately
        {
            let mut write_guard = dev.state.write().await;
            write_guard
                .sensors
                .insert(api_tag.to_string(), send_val.clone());
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
