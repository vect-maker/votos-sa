use crate::cli::SimulateArgs;
use crate::constants::*;
use crate::project::get_client;
use anyhow::{bail, Context, Result};
use crossterm::{
    cursor::{Hide, Show},
    event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use nle_cloud_sdk::models::{DeviceBaseInfoDto, DeviceQueryParams, ProjectQueryParams};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, Gauge, GraphType, Paragraph},
    Frame, Terminal,
};
use serde_json::json;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};

/// RAII Guard ensuring terminal raw mode and alternate screen are always restored.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, Show);
    }
}

/// Holds the in-memory state of the simulated smart home device.
#[derive(Debug, Clone)]
pub struct SimulatedDeviceState {
    pub device_tag: String,
    pub device_id: Option<i32>,
    pub device_name: Option<String>,
    pub brightness: f32,
    pub brightness_history: Vec<f64>,
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
            brightness_history: vec![135.5; 40],
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

    pub fn record_brightness(&mut self, val: f32) {
        self.brightness = val;
        self.brightness_history.push(val as f64);
        if self.brightness_history.len() > 60 {
            self.brightness_history.remove(0);
        }
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

/// Renders the complete Ratatui TUI dashboard.
fn render_ui(f: &mut Frame, state: &SimulatedDeviceState, args: &SimulateArgs) {
    let size = f.area();

    // Root layout: Header (3), Main Body (Min 14), Footer (3)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(14),
            Constraint::Length(3),
        ])
        .split(size);

    // --- 1. HEADER BAR ---
    let status_span = if state.is_online {
        Span::styled("● ONLINE", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        Span::styled("○ OFFLINE", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
    };

    let dev_name = state.device_name.as_deref().unwrap_or("N/A");
    let dev_id_str = state
        .device_id
        .map(|id| format!(" (ID: {id})"))
        .unwrap_or_default();

    let header_line = Line::from(vec![
        Span::styled(" Device: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{}{dev_id_str} [{dev_name}]", state.device_tag),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::raw("   │   "),
        Span::styled("Status: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        status_span,
        Span::raw("   │   "),
        Span::styled("Gateway: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{}:{}", args.gateway_host, args.gateway_port), Style::default().fg(Color::DarkGray)),
        Span::raw("   │   "),
        Span::styled("Traffic: ", Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
        Span::styled(format!("↑ {}  ↓ {}", state.packets_sent, state.packets_received), Style::default().fg(Color::Yellow)),
    ]);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .title(" NLECloud IoT Device Simulator ")
        .title_alignment(Alignment::Center)
        .border_style(Style::default().fg(Color::Cyan));

    f.render_widget(Paragraph::new(vec![header_line]).block(header_block), chunks[0]);

    // --- 2. MAIN BODY (Left: Peripherals 45%, Right: Tracing Logs 55%) ---
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[1]);

    // Split Left Column into Actuators (Length 11) and Sensor Chart (Min 6)
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(11), Constraint::Min(6)])
        .split(body_chunks[0]);

    // Split Actuators block into Buttons (5), Servo X (3), Servo Y (3)
    let actuator_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(left_chunks[0]);

    // 2.1 Buttons (Lamp, Fan, Lock)
    let lamp_badge = if state.lamp {
        Span::styled(" [● ON ] ", Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [○ OFF] ", Style::default().bg(Color::DarkGray).fg(Color::White))
    };

    let fan_badge = if state.fan {
        Span::styled(" [● ON ] ", Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [○ OFF] ", Style::default().bg(Color::DarkGray).fg(Color::White))
    };

    let lock_badge = if state.lock {
        Span::styled(" [🔒 LOCKED] ", Style::default().bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [🔓 UNLOCKED] ", Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD))
    };

    let buttons_text = vec![
        Line::from(vec![
            Span::styled(" Lamp: ", Style::default().add_modifier(Modifier::BOLD)),
            lamp_badge,
            Span::styled("   (press 'l' to toggle)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" Fan:  ", Style::default().add_modifier(Modifier::BOLD)),
            fan_badge,
            Span::styled("   (press 'f' to toggle)", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(" Lock: ", Style::default().add_modifier(Modifier::BOLD)),
            lock_badge,
            Span::styled("   (press 'k' to toggle)", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let buttons_block = Block::default()
        .borders(Borders::ALL)
        .title(" Actuators: Switches ")
        .border_style(Style::default().fg(Color::Blue));
    f.render_widget(Paragraph::new(buttons_text).block(buttons_block), actuator_rows[0]);

    // 2.2 Servo X Gauge (0..180 deg)
    let x_pct = ((state.servo_x as f32 / 180.0) * 100.0).clamp(0.0, 100.0) as u16;
    let servo_x_block = Block::default()
        .borders(Borders::ALL)
        .title(" Servo X (Horizontal 0-180°) [← / →] ")
        .border_style(Style::default().fg(Color::Rgb(56, 189, 248)));
    let servo_x_gauge = Gauge::default()
        .block(servo_x_block)
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(14, 165, 233))
                .bg(Color::Rgb(15, 23, 42))
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().fg(Color::Rgb(71, 85, 105)).bg(Color::Rgb(15, 23, 42)))
        .percent(x_pct)
        .label(format!("{}° / 180°  ({}%)", state.servo_x, x_pct));
    f.render_widget(servo_x_gauge, actuator_rows[1]);

    // 2.3 Servo Y Gauge (0..180 deg)
    let y_pct = ((state.servo_y as f32 / 180.0) * 100.0).clamp(0.0, 100.0) as u16;
    let servo_y_block = Block::default()
        .borders(Borders::ALL)
        .title(" Servo Y (Vertical 0-180°) [↓ / ↑] ")
        .border_style(Style::default().fg(Color::Rgb(192, 132, 252)));
    let servo_y_gauge = Gauge::default()
        .block(servo_y_block)
        .gauge_style(
            Style::default()
                .fg(Color::Rgb(168, 85, 247))
                .bg(Color::Rgb(15, 23, 42))
                .add_modifier(Modifier::BOLD),
        )
        .style(Style::default().fg(Color::Rgb(71, 85, 105)).bg(Color::Rgb(15, 23, 42)))
        .percent(y_pct)
        .label(format!("{}° / 180°  ({}%)", state.servo_y, y_pct));
    f.render_widget(servo_y_gauge, actuator_rows[2]);

    // 2.4 Sensor: Brightness Real-Time Line Graph
    let brightness_data: Vec<(f64, f64)> = state
        .brightness_history
        .iter()
        .enumerate()
        .map(|(i, &val)| (i as f64, val))
        .collect();

    let (min_val, max_val) = state.brightness_history.iter().fold(
        (state.brightness as f64, state.brightness as f64),
        |(min, max), &v| (min.min(v), max.max(v)),
    );

    let y_min = (min_val - 15.0).max(0.0).floor();
    let y_max = (max_val + 15.0).ceil();
    let y_mid = ((y_min + y_max) / 2.0).round();

    let num_points = state.brightness_history.len() as f64;
    let x_max = if num_points > 1.0 { num_points - 1.0 } else { 1.0 };

    let dataset = Dataset::default()
        .name(format!("{:.2} flux", state.brightness))
        .marker(Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Rgb(250, 204, 21))) // Vibrant Golden Amber
        .data(&brightness_data);

    let x_axis = Axis::default()
        .bounds([0.0, x_max])
        .style(Style::default().fg(Color::DarkGray))
        .labels(["-60s", "-30s", "Now"]);

    let y_axis = Axis::default()
        .bounds([y_min, y_max])
        .style(Style::default().fg(Color::DarkGray))
        .labels([
            format!("{y_min:.0}"),
            format!("{y_mid:.0}"),
            format!("{y_max:.0}"),
        ]);

    let chart_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Sensor: Brightness Line Graph ({:.2} flux) ", state.brightness))
        .border_style(Style::default().fg(Color::Rgb(250, 204, 21)));

    let chart = Chart::new(vec![dataset])
        .block(chart_block)
        .x_axis(x_axis)
        .y_axis(y_axis);
    f.render_widget(chart, left_chunks[1]);

    // 2.5 Right Column: Live Tracing Event Logs
    let logger_widget = TuiLoggerWidget::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Live Event & Tracing Logs ")
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .output_separator(' ')
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
        .output_target(false)
        .output_file(false)
        .output_line(false)
        .style_error(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_info(Style::default().fg(Color::Green))
        .style_debug(Style::default().fg(Color::DarkGray));
    f.render_widget(logger_widget, body_chunks[1]);

    // --- 3. FOOTER SHORTCUTS BAR ---
    let help_line = Line::from(vec![
        Span::styled(" [q] ", Style::default().fg(Color::Black).bg(Color::White).add_modifier(Modifier::BOLD)),
        Span::raw(" Quit   "),
        Span::styled(" [l] ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" Lamp   "),
        Span::styled(" [f] ", Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" Fan   "),
        Span::styled(" [k] ", Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(" Lock   "),
        Span::styled(" [←/→] ", Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Servo X   "),
        Span::styled(" [↓/↑] ", Style::default().fg(Color::Black).bg(Color::LightCyan).add_modifier(Modifier::BOLD)),
        Span::raw(" Servo Y   "),
        Span::styled(" [t] ", Style::default().fg(Color::Black).bg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::raw(" Telemetry Tick "),
    ]);

    let footer_p = Paragraph::new(vec![help_line])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Interactive Controls ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(footer_p, chunks[2]);
}

/// Runs the simulated TCP device lifecycle inside Ratatui and Crossterm.
pub async fn run(args: SimulateArgs) -> Result<()> {
    let (tag, key, dev_id, dev_name) = resolve_target_device(&args).await?;

    let mut state = SimulatedDeviceState::new(tag.clone(), dev_id, dev_name);
    let gateway_addr = format!("{}:{}", args.gateway_host, args.gateway_port);

    // Setup terminal and RAII guard
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, Hide)?;
    let _guard = TerminalGuard;

    // Set panic hook to restore terminal cleanly on unexpected panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, Show);
        original_hook(panic_info);
    }));

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    tracing::info!(
        device = %tag,
        id = ?dev_id,
        gateway = %gateway_addr,
        "Initiating TCP connection to NLECloud gateway..."
    );
    terminal.draw(|f| render_ui(f, &state, &args))?;

    let mut socket = TcpStream::connect(&gateway_addr)
        .await
        .with_context(|| format!("Failed to connect to NLECloud TCP gateway at {gateway_addr}"))?;

    tracing::info!(gateway = %gateway_addr, "TCP connection established.");
    terminal.draw(|f| render_ui(f, &state, &args))?;

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
    tracing::info!(device = %tag, "Sent gateway handshake request (t: 1)");
    terminal.draw(|f| render_ui(f, &state, &args))?;

    // Wait for handshake response (t: 2)
    let mut initial_buf = [0u8; 1024];
    let n = socket.read(&mut initial_buf).await?;
    if n == 0 {
        bail!("Gateway closed connection immediately after handshake request");
    }
    state.packets_received += 1;

    let initial_resp_str = String::from_utf8_lossy(&initial_buf[..n]);
    let handshake_resp: serde_json::Value = serde_json::from_str(initial_resp_str.trim())
        .with_context(|| format!("Failed to parse handshake response: '{initial_resp_str}'"))?;

    let status = handshake_resp.get("status").and_then(|s| s.as_i64()).unwrap_or(-1);
    if status != 0 {
        bail!("NLECloud gateway rejected connection (status: {status}). Verify device tag and secret key.");
    }

    state.is_online = true;
    state.last_event = "Handshake accepted. Device is ONLINE.".to_string();
    tracing::info!(device = %tag, status = status, "Device is ONLINE on NLECloud TCP gateway.");
    terminal.draw(|f| render_ui(f, &state, &args))?;

    // Initial telemetry sync
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
    telemetry_ticker.tick().await;

    let mut heartbeat_ticker = tokio::time::interval(Duration::from_secs(args.heartbeat_interval_secs));
    heartbeat_ticker.tick().await;

    let mut event_stream = EventStream::new();
    let mut read_buf = [0u8; 4096];
    let mut incoming_acc = String::new();
    let sim_start = Instant::now();

    loop {
        terminal.draw(|f| render_ui(f, &state, &args))?;

        tokio::select! {
            // Interactive Keyboard & Terminal Events
            Some(Ok(crossterm_event)) = event_stream.next() => {
                match crossterm_event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        // Quit
                        if key.code == KeyCode::Char('q')
                            || key.code == KeyCode::Esc
                            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
                        {
                            tracing::info!("User initiated shutdown via hotkey. Terminating simulator...");
                            break;
                        }

                        // Toggle Lamp (l)
                        if key.code == KeyCode::Char('l') {
                            state.lamp = !state.lamp;
                            let val = if state.lamp { 1 } else { 0 };
                            seq += 1;
                            let payload = json!({ "t": 3, "datatype": 1, "datas": { TAG_LAMP: val }, "msgid": seq });
                            let mut b = serde_json::to_vec(&payload)?;
                            b.push(b'\r');
                            socket.write_all(&b).await?;
                            state.packets_sent += 1;
                            tracing::info!(lamp = state.lamp, "Locally toggled lamp and synced to cloud (t: 3)");
                        }

                        // Toggle Fan (f)
                        if key.code == KeyCode::Char('f') {
                            state.fan = !state.fan;
                            let val = if state.fan { 1 } else { 0 };
                            seq += 1;
                            let payload = json!({ "t": 3, "datatype": 1, "datas": { TAG_FAN: val }, "msgid": seq });
                            let mut b = serde_json::to_vec(&payload)?;
                            b.push(b'\r');
                            socket.write_all(&b).await?;
                            state.packets_sent += 1;
                            tracing::info!(fan = state.fan, "Locally toggled fan and synced to cloud (t: 3)");
                        }

                        // Toggle Lock (k)
                        if key.code == KeyCode::Char('k') {
                            state.lock = !state.lock;
                            let val = if state.lock { 1 } else { 0 };
                            seq += 1;
                            let payload = json!({ "t": 3, "datatype": 1, "datas": { TAG_LOCK: val }, "msgid": seq });
                            let mut b = serde_json::to_vec(&payload)?;
                            b.push(b'\r');
                            socket.write_all(&b).await?;
                            state.packets_sent += 1;
                            tracing::info!(lock = state.lock, "Locally toggled lock and synced to cloud (t: 3)");
                        }

                        // Servo X Adjust (Left / Right)
                        if key.code == KeyCode::Left {
                            state.servo_x = state.servo_x.saturating_sub(5);
                            seq += 1;
                            let payload = json!({ "t": 3, "datatype": 1, "datas": { TAG_SERVO_X: state.servo_x }, "msgid": seq });
                            let mut b = serde_json::to_vec(&payload)?;
                            b.push(b'\r');
                            socket.write_all(&b).await?;
                            state.packets_sent += 1;
                            tracing::info!(servo_x = state.servo_x, "Locally adjusted Servo X (t: 3)");
                        }
                        if key.code == KeyCode::Right {
                            state.servo_x = (state.servo_x + 5).min(SERVO_MAX_ANGLE);
                            seq += 1;
                            let payload = json!({ "t": 3, "datatype": 1, "datas": { TAG_SERVO_X: state.servo_x }, "msgid": seq });
                            let mut b = serde_json::to_vec(&payload)?;
                            b.push(b'\r');
                            socket.write_all(&b).await?;
                            state.packets_sent += 1;
                            tracing::info!(servo_x = state.servo_x, "Locally adjusted Servo X (t: 3)");
                        }

                        // Servo Y Adjust (Down / Up)
                        if key.code == KeyCode::Down {
                            state.servo_y = state.servo_y.saturating_sub(5);
                            seq += 1;
                            let payload = json!({ "t": 3, "datatype": 1, "datas": { TAG_SERVO_Y: state.servo_y }, "msgid": seq });
                            let mut b = serde_json::to_vec(&payload)?;
                            b.push(b'\r');
                            socket.write_all(&b).await?;
                            state.packets_sent += 1;
                            tracing::info!(servo_y = state.servo_y, "Locally adjusted Servo Y (t: 3)");
                        }
                        if key.code == KeyCode::Up {
                            state.servo_y = (state.servo_y + 5).min(SERVO_MAX_ANGLE);
                            seq += 1;
                            let payload = json!({ "t": 3, "datatype": 1, "datas": { TAG_SERVO_Y: state.servo_y }, "msgid": seq });
                            let mut b = serde_json::to_vec(&payload)?;
                            b.push(b'\r');
                            socket.write_all(&b).await?;
                            state.packets_sent += 1;
                            tracing::info!(servo_y = state.servo_y, "Locally adjusted Servo Y (t: 3)");
                        }

                        // Force Telemetry Tick (t)
                        if key.code == KeyCode::Char('t') {
                            let elapsed = sim_start.elapsed().as_secs_f64();
                            let raw_val = 140.0 + 35.0 * (elapsed * 0.1).sin() + 10.0 * (elapsed * 0.28).cos();
                            state.record_brightness(((raw_val * 100.0).round() / 100.0) as f32);

                            seq += 1;
                            let payload = json!({
                                "t": 3,
                                "datatype": 1,
                                "datas": { TAG_BRIGHTNESS: state.brightness },
                                "msgid": seq
                            });
                            let mut b = serde_json::to_vec(&payload)?;
                            b.push(b'\r');
                            socket.write_all(&b).await?;
                            state.packets_sent += 1;
                            tracing::info!(brightness = state.brightness, msgid = seq, "Manual telemetry tick sent (t: 3)");
                        }
                    }
                    _ => {}
                }
            }

            // Periodic Telemetry upload
            _ = telemetry_ticker.tick() => {
                let elapsed = sim_start.elapsed().as_secs_f64();
                let raw_val = 140.0 + 35.0 * (elapsed * 0.1).sin() + 10.0 * (elapsed * 0.28).cos();
                state.record_brightness(((raw_val * 100.0).round() / 100.0) as f32);

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
                tracing::info!(brightness = state.brightness, msgid = seq, "Uploaded telemetry data (t: 3)");
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

                while let Some(pos) = incoming_acc.find(['\r', '\n']) {
                    let frame = incoming_acc[..pos].trim().to_string();
                    incoming_acc.drain(..=pos);

                    if frame.is_empty() {
                        continue;
                    }

                    if frame.starts_with('$') {
                        tracing::debug!(frame = %frame, "Gateway heartbeat response received");
                        continue;
                    }

                    match serde_json::from_str::<serde_json::Value>(&frame) {
                        Ok(val) => {
                            let msg_type = val.get("t").and_then(|t| t.as_i64()).unwrap_or(0);
                            match msg_type {
                                4 => {
                                    let ack_id = val.get("msgid").and_then(|m| m.as_i64()).unwrap_or(0);
                                    tracing::debug!(msgid = ack_id, "Cloud acknowledged telemetry packet (t: 4)");
                                }
                                5 => {
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

                                    match apitag.as_str() {
                                        TAG_LAMP => {
                                            if let Some(b) = parse_bool_value(&data) {
                                                state.lamp = b;
                                                reported_json_val = json!(if b { 1 } else { 0 });
                                            }
                                        }
                                        TAG_FAN => {
                                            if let Some(b) = parse_bool_value(&data) {
                                                state.fan = b;
                                                reported_json_val = json!(if b { 1 } else { 0 });
                                            }
                                        }
                                        TAG_LOCK => {
                                            if let Some(b) = parse_bool_value(&data) {
                                                state.lock = b;
                                                reported_json_val = json!(if b { 1 } else { 0 });
                                            }
                                        }
                                        TAG_SERVO_X => {
                                            if let Some(angle) = parse_u16_value(&data) {
                                                state.servo_x = angle;
                                                reported_json_val = json!(angle);
                                            }
                                        }
                                        TAG_SERVO_Y => {
                                            if let Some(angle) = parse_u16_value(&data) {
                                                state.servo_y = angle;
                                                reported_json_val = json!(angle);
                                            }
                                        }
                                        unknown => {
                                            tracing::warn!(tag = %unknown, "Received command for unknown actuator tag");
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
                tracing::info!("Received interrupt signal (Ctrl+C). Terminating simulator...");
                break;
            }
        }
    }

    Ok(())
}
