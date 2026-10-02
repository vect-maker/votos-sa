use crate::cli::SimulateArgs;
use crate::constants::*;
use crate::project::get_client;
use anyhow::{bail, Context, Result};
use nle_cloud_sdk::models::{DeviceBaseInfoDto, DeviceQueryParams, ProjectQueryParams};
use serde_json::json;
use std::io::Write;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Holds the in-memory state of the simulated smart home device.
#[derive(Debug, Clone)]
pub struct SimulatedDeviceState {
    pub device_tag: String,
    pub device_id: Option<i32>,
    pub device_name: Option<String>,
    pub brightness: f32,
    pub lamp: bool,
    pub fan: bool,
    pub lock: bool,
    pub servo_x: u16,
    pub servo_y: u16,
    pub is_online: bool,
    pub last_event: String,
    pub packets_sent: u64,
    pub packets_received: u64,
}

impl SimulatedDeviceState {
    pub fn new(tag: String, device_id: Option<i32>, name: Option<String>) -> Self {
        Self {
            device_tag: tag,
            device_id,
            device_name: name,
            brightness: 135.5,
            lamp: false,
            fan: false,
            lock: true,
            servo_x: 90,
            servo_y: 45,
            is_online: false,
            last_event: "Initializing simulator...".to_string(),
            packets_sent: 0,
            packets_received: 0,
        }
    }

    /// Renders a terminal status panel displaying all peripherals, online status, and traffic.
    pub fn render_tui(&self, host: &str, port: u16) {
        let cyan = "\x1b[36m";
        let bold_cyan = "\x1b[1;36m";
        let bold_green = "\x1b[1;32m";
        let bold_red = "\x1b[1;31m";
        let yellow = "\x1b[33m";
        let bold_yellow = "\x1b[1;33m";
        let dim = "\x1b[2m";
        let bold = "\x1b[1m";
        let reset = "\x1b[0m";

        let status_badge = if self.is_online {
            format!("{bold_green}ONLINE ●{reset}")
        } else {
            format!("{bold_red}OFFLINE ○{reset}")
        };

        let lamp_badge = if self.lamp {
            format!("{bold_green}[ ON  ]{reset}")
        } else {
            format!("{dim}[ OFF ]{reset}")
        };

        let fan_badge = if self.fan {
            format!("{bold_green}[ ON  ]{reset}")
        } else {
            format!("{dim}[ OFF ]{reset}")
        };

        let lock_badge = if self.lock {
            format!("{bold_yellow}[ LOCKED ]{reset}")
        } else {
            format!("{bold_green}[ UNLOCKED ]{reset}")
        };

        let dev_id_str = self
            .device_id
            .map(|id| format!(" (ID: {id})"))
            .unwrap_or_default();
        let name_str = self
            .device_name
            .as_deref()
            .map(|n| format!(" [{n}]"))
            .unwrap_or_default();
        let target_display = format!("{}{dev_id_str}{name_str}", self.device_tag);

        let gateway_display = format!("{host}:{port}");
        let traffic_display = format!("↑ {:<4} pkts   ↓ {:<4} pkts", self.packets_sent, self.packets_received);

        println!();
        println!("{bold_cyan}┌─────────────────────── NLECLOUD VIRTUAL DEVICE STATUS ───────────────────────┐{reset}");
        println!("│ {bold}Device:{reset}  {cyan}{:<30}{reset} Status:  {:<26}│", target_display, status_badge);
        println!("│ {bold}Gateway:{reset} {dim}{:<30}{reset} Traffic: {:<29}│", gateway_display, traffic_display);
        println!("{bold_cyan}├──────────────────────────────────────────────────────────────────────────────┤{reset}");
        println!("│ {bold_yellow}PERIPHERALS{reset}                                                                  │");
        println!("│   • brightness : {yellow}{:>6.2} flux{reset}                                                 │", self.brightness);
        println!("│   • lamp       : {:<21}                                       │", lamp_badge);
        println!("│   • fan        : {:<21}                                       │", fan_badge);
        println!("│   • lock       : {:<21}                                       │", lock_badge);
        println!("│   • servo_x    : {cyan}{:>3}°{reset}                                                          │", self.servo_x);
        println!("│   • servo_y    : {cyan}{:>3}°{reset}                                                          │", self.servo_y);
        println!("{bold_cyan}├──────────────────────────────────────────────────────────────────────────────┤{reset}");
        println!("│ {bold}Last Action:{reset} {:<64} │", self.last_event.chars().take(64).collect::<String>());
        println!("{bold_cyan}└──────────────────────────────────────────────────────────────────────────────┘{reset}");
        println!();
    }
}

/// Resolves target device credentials (Tag and SecurityKey).
/// If not supplied via CLI options, interactively prompts user from account devices.
pub async fn resolve_target_device(
    args: &SimulateArgs,
) -> Result<(String, String, Option<i32>, Option<String>)> {
    // 1. Both device tag and secret key provided explicitly
    if let (Some(tag), Some(key)) = (&args.device_tag, &args.secret_key) {
        if !tag.trim().is_empty() && !key.trim().is_empty() {
            return Ok((tag.trim().to_string(), key.trim().to_string(), None, None));
        }
    }

    // 2. Query NLECloud API for devices
    tracing::info!("Discovering devices in NLECloud account...");
    let client = get_client(&args.project).await?;

    let paged_projects = client
        .get_projects(&ProjectQueryParams::builder().page_size(100).build(), None)
        .await
        .context("Failed to query projects from NLECloud")?;

    let target_project_name = args.project.resolved_name();
    let matching_projects: Vec<_> = paged_projects
        .page_set
        .iter()
        .filter(|p| p.name.as_deref() == Some(&target_project_name))
        .cloned()
        .collect();

    let projects_to_search = if !matching_projects.is_empty() {
        tracing::info!("Found project matching '{target_project_name}'. Searching devices...");
        matching_projects
    } else {
        tracing::info!("Project '{target_project_name}' not found; querying all projects in account...");
        paged_projects.page_set
    };

    let mut candidate_devices: Vec<(DeviceBaseInfoDto, String)> = Vec::new();

    for proj in projects_to_search {
        let p_name = proj
            .name
            .clone()
            .unwrap_or_else(|| format!("Project {}", proj.project_id));
        let query = DeviceQueryParams::builder()
            .project_key_word(proj.project_id.to_string())
            .page_size(100)
            .build();

        if let Ok(paged_devs) = client.get_devices(&query, None).await {
            for dev in paged_devs.page_set {
                candidate_devices.push((dev, p_name.clone()));
            }
        }
    }

    if candidate_devices.is_empty() {
        bail!("No devices found in NLECloud account for project '{target_project_name}'. Please provision a device first (e.g., `just provision`).");
    }

    // If device_tag was specified, find matching device
    let selected_device = if let Some(tag) = &args.device_tag {
        candidate_devices
            .into_iter()
            .find(|(d, _)| d.tag.as_deref() == Some(tag))
            .ok_or_else(|| anyhow::anyhow!("Device with tag '{tag}' not found in NLECloud account"))?
    } else if candidate_devices.len() == 1 {
        // Auto-select single device
        let chosen = candidate_devices.remove(0);
        let tag = chosen.0.tag.as_deref().unwrap_or("unknown");
        tracing::info!(tag = %tag, id = chosen.0.device_id, "Single device found in account. Automatically selected.");
        chosen
    } else {
        // Interactive selection prompt
        println!("\nAvailable devices in NLECloud account:");
        for (idx, (dev, proj_name)) in candidate_devices.iter().enumerate() {
            let tag = dev.tag.as_deref().unwrap_or("unknown");
            let name = dev.name.as_deref().unwrap_or("unnamed");
            let status = if dev.is_online { "ONLINE" } else { "OFFLINE" };
            println!(
                "  [{}] {tag} (ID: {}, Name: '{name}', Project: '{proj_name}', Status: {status})",
                idx + 1,
                dev.device_id
            );
        }
        print!(
            "\nSelect device to simulate [1-{}] (default 1): ",
            candidate_devices.len()
        );
        let _ = std::io::stdout().flush();

        let mut line = String::new();
        let mut reader = BufReader::new(tokio::io::stdin());
        let _ = reader.read_line(&mut line).await;

        let selection: usize = line.trim().parse().unwrap_or(1);
        let chosen_idx = if selection >= 1 && selection <= candidate_devices.len() {
            selection - 1
        } else {
            0
        };

        candidate_devices.remove(chosen_idx)
    };

    let (base_dev, _) = selected_device;
    let tag = base_dev
        .tag
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Selected device has no tag"))?;

    // Determine security key: use arg override or fetch full device info
    let key = if let Some(k) = &args.secret_key {
        k.trim().to_string()
    } else if let Some(k) = base_dev.security_key {
        k
    } else {
        tracing::info!(device_id = base_dev.device_id, "Fetching device security key from API...");
        let full_info = client
            .get_device_info(base_dev.device_id, None)
            .await
            .context("Failed to get device info for security key")?;
        full_info
            .base
            .security_key
            .ok_or_else(|| anyhow::anyhow!("Device has no security key configured in NLECloud"))?
    };

    Ok((tag, key, Some(base_dev.device_id), base_dev.name))
}

fn parse_bool_value(v: &serde_json::Value) -> Option<bool> {
    if let Some(b) = v.as_bool() {
        return Some(b);
    }
    if let Some(n) = v.as_i64() {
        return Some(n != 0);
    }
    if let Some(n) = v.as_f64() {
        return Some(n != 0.0);
    }
    if let Some(s) = v.as_str() {
        return match s.to_lowercase().as_str() {
            "1" | "true" | "on" => Some(true),
            "0" | "false" | "off" => Some(false),
            _ => None,
        };
    }
    None
}

fn parse_u16_value(v: &serde_json::Value) -> Option<u16> {
    if let Some(n) = v.as_u64() {
        return Some((n as u16).min(SERVO_MAX_ANGLE));
    }
    if let Some(n) = v.as_i64() {
        return Some((n.clamp(0, SERVO_MAX_ANGLE as i64)) as u16);
    }
    if let Some(f) = v.as_f64() {
        return Some((f.clamp(0.0, SERVO_MAX_ANGLE as f64)) as u16);
    }
    if let Some(s) = v.as_str() {
        return s
            .trim()
            .parse::<f64>()
            .ok()
            .map(|f| (f.clamp(0.0, SERVO_MAX_ANGLE as f64)) as u16);
    }
    None
}

/// Runs the simulated TCP device lifecycle.
pub async fn run(args: SimulateArgs) -> Result<()> {
    let (tag, key, dev_id, dev_name) = resolve_target_device(&args).await?;

    let mut state = SimulatedDeviceState::new(tag.clone(), dev_id, dev_name);
    let gateway_addr = format!("{}:{}", args.gateway_host, args.gateway_port);

    tracing::info!(
        device = %tag,
        id = ?dev_id,
        gateway = %gateway_addr,
        "Initiating TCP connection to NLECloud gateway..."
    );

    let mut socket = TcpStream::connect(&gateway_addr)
        .await
        .with_context(|| format!("Failed to connect to NLECloud TCP gateway at {gateway_addr}"))?;

    tracing::info!(gateway = %gateway_addr, "TCP connection established.");

    // 1. Handshake request (t: 1)
    let handshake_req = json!({
        "t": 1,
        "device": tag,
        "key": key
    });
    let mut handshake_bytes = serde_json::to_vec(&handshake_req)?;
    handshake_bytes.push(b'\r');
    socket.write_all(&handshake_bytes).await?;
    state.packets_sent += 1;
    tracing::info!(device = %tag, "Sent gateway handshake (t: 1)");

    // Wait for handshake response (t: 2)
    let mut initial_buf = [0u8; 1024];
    let n = socket.read(&mut initial_buf).await?;
    if n == 0 {
        bail!("Gateway closed connection immediately after handshake request");
    }
    state.packets_received += 1;

    let initial_resp_str = String::from_utf8_lossy(&initial_buf[..n]);
    tracing::debug!(raw = %initial_resp_str, "Received handshake response");

    let handshake_resp: serde_json::Value = serde_json::from_str(initial_resp_str.trim())
        .with_context(|| format!("Failed to parse handshake response: '{initial_resp_str}'"))?;

    let status = handshake_resp.get("status").and_then(|s| s.as_i64()).unwrap_or(-1);
    if status != 0 {
        bail!("NLECloud gateway rejected connection (status: {status}). Verify device tag and secret key.");
    }

    state.is_online = true;
    state.last_event = "Handshake accepted. Device is ONLINE.".to_string();
    tracing::info!(device = %tag, status = status, "Device is ONLINE on NLECloud TCP gateway.");

    // Render initial TUI
    state.render_tui(&args.gateway_host, args.gateway_port);

    // Initial telemetry sync so dashboard immediately sees peripheral values
    let mut seq: u64 = 1;
    let initial_sync = json!({
        "t": 3,
        "datatype": 1,
        "datas": {
            TAG_BRIGHTNESS: state.brightness,
            TAG_LAMP: if state.lamp { 1 } else { 0 },
            TAG_FAN: if state.fan { 1 } else { 0 },
            TAG_LOCK: if state.lock { 1 } else { 0 },
            TAG_SERVO_X: state.servo_x,
            TAG_SERVO_Y: state.servo_y
        },
        "msgid": seq
    });
    let mut sync_bytes = serde_json::to_vec(&initial_sync)?;
    sync_bytes.push(b'\r');
    socket.write_all(&sync_bytes).await?;
    state.packets_sent += 1;
    tracing::info!(msgid = seq, "Published initial peripheral telemetry state (t: 3)");

    let mut telemetry_ticker = tokio::time::interval(Duration::from_secs(args.telemetry_interval_secs));
    // Consume immediate first tick
    telemetry_ticker.tick().await;

    let mut heartbeat_ticker = tokio::time::interval(Duration::from_secs(args.heartbeat_interval_secs));
    // Consume immediate first tick
    heartbeat_ticker.tick().await;

    let mut read_buf = [0u8; 4096];
    let mut incoming_acc = String::new();
    let sim_start = Instant::now();

    loop {
        tokio::select! {
            // Periodic Telemetry upload
            _ = telemetry_ticker.tick() => {
                let elapsed = sim_start.elapsed().as_secs_f64();
                // Realistic sinusoidal fluctuation around 140 flux
                let raw_val = 140.0 + 35.0 * (elapsed * 0.1).sin() + 10.0 * (elapsed * 0.28).cos();
                state.brightness = ((raw_val * 100.0).round() / 100.0) as f32;

                seq += 1;
                let payload = json!({
                    "t": 3,
                    "datatype": 1,
                    "datas": {
                        TAG_BRIGHTNESS: state.brightness
                    },
                    "msgid": seq
                });
                let mut bytes = serde_json::to_vec(&payload)?;
                bytes.push(b'\r');
                if let Err(e) = socket.write_all(&bytes).await {
                    tracing::error!(error = %e, "Failed to send telemetry packet");
                    break;
                }
                state.packets_sent += 1;
                state.last_event = format!("Uploaded telemetry: brightness = {:.2} flux", state.brightness);
                tracing::info!(brightness = state.brightness, msgid = seq, "Uploaded telemetry data (t: 3)");
                state.render_tui(&args.gateway_host, args.gateway_port);
            }

            // Periodic Heartbeat
            _ = heartbeat_ticker.tick() => {
                if let Err(e) = socket.write_all(b"$#AT#\r").await {
                    tracing::error!(error = %e, "Failed to send heartbeat");
                    break;
                }
                state.packets_sent += 1;
                tracing::info!("Sent heartbeat frame ($#AT#)");
            }

            // Incoming packets from Gateway (Commands, ACKs, Heartbeats)
            read_res = socket.read(&mut read_buf) => {
                let n = match read_res {
                    Ok(0) => {
                        tracing::warn!("Gateway closed TCP connection.");
                        state.is_online = false;
                        state.last_event = "Gateway closed connection".to_string();
                        state.render_tui(&args.gateway_host, args.gateway_port);
                        break;
                    }
                    Ok(n) => n,
                    Err(e) => {
                        tracing::error!(error = %e, "Socket read error");
                        break;
                    }
                };

                state.packets_received += 1;
                incoming_acc.push_str(&String::from_utf8_lossy(&read_buf[..n]));

                // Process complete framed segments
                while let Some(pos) = incoming_acc.find(['\r', '\n']) {
                    let frame = incoming_acc[..pos].trim().to_string();
                    incoming_acc.drain(..=pos);

                    if frame.is_empty() {
                        continue;
                    }

                    if frame == "$#AT#" {
                        tracing::debug!("Gateway echoed heartbeat frame ($#AT#)");
                        continue;
                    }

                    match serde_json::from_str::<serde_json::Value>(&frame) {
                        Ok(val) => {
                            let msg_type = val.get("t").and_then(|t| t.as_i64()).unwrap_or(0);
                            match msg_type {
                                4 => {
                                    // Telemetry ACK (t: 4)
                                    let ack_id = val.get("msgid").and_then(|m| m.as_i64()).unwrap_or(0);
                                    tracing::debug!(msgid = ack_id, "Cloud acknowledged telemetry packet (t: 4)");
                                }
                                5 => {
                                    // Inbound Control Command (t: 5)
                                    let cmdid = val.get("cmdid").cloned().unwrap_or_else(|| json!(0));
                                    let apitag = val.get("apitag").and_then(|a| a.as_str()).unwrap_or("").to_string();
                                    let data = val.get("data").cloned().unwrap_or_else(|| json!(null));

                                    tracing::info!(
                                        cmdid = %cmdid,
                                        tag = %apitag,
                                        data = %data,
                                        "Received command from NLECloud (t: 5)"
                                    );

                                    // 1. Reply immediately with command ACK (t: 6)
                                    let ack_payload = json!({
                                        "t": 6,
                                        "cmdid": cmdid,
                                        "status": 0,
                                        "data": 0
                                    });
                                    let mut ack_bytes = serde_json::to_vec(&ack_payload)?;
                                    ack_bytes.push(b'\r');
                                    socket.write_all(&ack_bytes).await?;
                                    state.packets_sent += 1;
                                    tracing::info!(cmdid = %cmdid, "Sent command ACK response (t: 6)");

                                    // 2. Update state and report updated value via t: 3
                                    let mut reported_json_val = json!(null);
                                    let mut action_desc = String::new();

                                    match apitag.as_str() {
                                        TAG_LAMP => {
                                            if let Some(b) = parse_bool_value(&data) {
                                                state.lamp = b;
                                                reported_json_val = json!(if b { 1 } else { 0 });
                                                action_desc = format!("CMD: lamp -> {}", if b { "ON" } else { "OFF" });
                                            }
                                        }
                                        TAG_FAN => {
                                            if let Some(b) = parse_bool_value(&data) {
                                                state.fan = b;
                                                reported_json_val = json!(if b { 1 } else { 0 });
                                                action_desc = format!("CMD: fan -> {}", if b { "ON" } else { "OFF" });
                                            }
                                        }
                                        TAG_LOCK => {
                                            if let Some(b) = parse_bool_value(&data) {
                                                state.lock = b;
                                                reported_json_val = json!(if b { 1 } else { 0 });
                                                action_desc = format!("CMD: lock -> {}", if b { "LOCKED" } else { "UNLOCKED" });
                                            }
                                        }
                                        TAG_SERVO_X => {
                                            if let Some(angle) = parse_u16_value(&data) {
                                                state.servo_x = angle;
                                                reported_json_val = json!(angle);
                                                action_desc = format!("CMD: servo_x -> {angle}°");
                                            }
                                        }
                                        TAG_SERVO_Y => {
                                            if let Some(angle) = parse_u16_value(&data) {
                                                state.servo_y = angle;
                                                reported_json_val = json!(angle);
                                                action_desc = format!("CMD: servo_y -> {angle}°");
                                            }
                                        }
                                        unknown => {
                                            tracing::warn!(tag = %unknown, "Received command for unknown actuator tag");
                                            action_desc = format!("CMD: unknown tag '{unknown}'");
                                        }
                                    }

                                    if !reported_json_val.is_null() {
                                        seq += 1;
                                        let report_payload = json!({
                                            "t": 3,
                                            "datatype": 1,
                                            "datas": {
                                                &apitag: reported_json_val
                                            },
                                            "msgid": seq
                                        });
                                        let mut rep_bytes = serde_json::to_vec(&report_payload)?;
                                        rep_bytes.push(b'\r');
                                        socket.write_all(&rep_bytes).await?;
                                        state.packets_sent += 1;
                                        tracing::info!(tag = %apitag, msgid = seq, "Synced updated actuator state to cloud (t: 3)");
                                    }

                                    state.last_event = format!("{action_desc} (ACK & State synced)");
                                    state.render_tui(&args.gateway_host, args.gateway_port);
                                }
                                other => {
                                    tracing::debug!(msg_type = other, "Received other message from gateway");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, frame = %frame, "Failed to parse incoming gateway frame as JSON");
                        }
                    }
                }
            }

            // Graceful shutdown on Ctrl+C
            _ = tokio::signal::ctrl_c() => {
                println!();
                tracing::info!("Received interrupt signal (Ctrl+C). Terminating simulator...");
                state.is_online = false;
                state.last_event = "Simulator terminated by user".to_string();
                state.render_tui(&args.gateway_host, args.gateway_port);
                break;
            }
        }
    }

    Ok(())
}
