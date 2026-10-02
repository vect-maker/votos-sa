use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "backend", version, about = "Backend CLI application")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the Axum HTTP & SSE (Server-Sent Events) server
    #[command(alias = "start", alias = "server")]
    Serve(ServeArgs),

    /// Project management commands
    #[command(subcommand)]
    Project(ProjectCommand),

    /// Provision an NLECloud project by name (idempotent)
    #[command(alias = "provision-project")]
    Provision(ProjectArgs),

    /// Delete an NLECloud project by name
    #[command(alias = "delete-project", alias = "destroy")]
    Delete(ProjectArgs),

    /// Run an interactive virtual device simulator over TCP
    #[command(alias = "sim", alias = "dummy")]
    Simulate(SimulateArgs),
}

#[derive(Subcommand, Debug, Clone)]
pub enum ProjectCommand {
    /// Provision an NLECloud project by name (idempotent)
    Provision(ProjectArgs),

    /// Delete an NLECloud project by name
    #[command(alias = "destroy")]
    Delete(ProjectArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ServeArgs {
    /// Host address to bind the server to
    #[arg(short = 'H', long, env = "HOST", default_value = "0.0.0.0")]
    pub host: String,

    /// Port to listen on
    #[arg(short, long, env = "PORT", default_value_t = 3000)]
    pub port: u16,

    /// Polling interval in milliseconds for NLECloud device state
    #[arg(long, env = "POLL_INTERVAL_MS", default_value_t = 2000)]
    pub poll_interval_ms: u64,

    /// Grace period in milliseconds for actuator commands to prevent polling from reverting state
    #[arg(long, env = "COMMAND_SETTLE_TIMEOUT_MS", default_value_t = 7000)]
    pub command_settle_timeout_ms: u64,

    #[command(flatten)]
    pub project: ProjectArgs,
}

#[derive(Args, Debug, Clone)]
pub struct ProjectArgs {
    /// Name of the project (can also be set via PROJECT_NAME or NLE_PROJECT_NAME env var)
    #[arg(
        short,
        long,
        env = "PROJECT_NAME",
        default_value = crate::constants::DEFAULT_PROJECT_NAME
    )]
    pub name: String,

    /// Path to declarative JSON file defining devices and peripherals (defaults to seed/devices.json)
    #[arg(
        short = 'f',
        long,
        env = "DEVICES_FILE",
        default_value = "seed/devices.json"
    )]
    pub devices_file: std::path::PathBuf,

    /// Optional namespace prefix for device names and tags (defaults to p<project_id>)
    #[arg(long, env = "DEVICE_NAMESPACE")]
    pub device_namespace: Option<String>,

    /// Industry category ID (default: 2 = Smart Home)
    #[arg(long, default_value_t = 2)]
    pub industry: u8,

    /// Network kind ID (default: 1 = WiFi)
    #[arg(long, default_value_t = 1)]
    pub network_kind: u8,

    /// NLECloud API base URL
    #[arg(long, env = "NLE_BASE_URL")]
    pub base_url: Option<String>,

    /// NLECloud AccessToken (optional if credentials are provided)
    #[arg(long, env = "NLE_TOKEN")]
    pub token: Option<String>,

    /// NLECloud Account username (or NLE_ACCOUNT env var)
    #[arg(short = 'u', long, env = "NLE_ACCOUNT")]
    pub account: Option<String>,

    /// NLECloud Account password (or NLE_PASSWORD env var)
    #[arg(short = 'P', long, env = "NLE_PASSWORD")]
    pub password: Option<String>,
}

impl ProjectArgs {
    /// Resolves the project name, honoring NLE_PROJECT_NAME if PROJECT_NAME was not overridden.
    pub fn resolved_name(&self) -> String {
        if self.name == crate::constants::DEFAULT_PROJECT_NAME {
            if let Ok(val) = std::env::var("NLE_PROJECT_NAME") {
                if !val.trim().is_empty() {
                    return val;
                }
            }
        }
        self.name.clone()
    }

    /// Resolves the devices JSON file path, checking both cwd and parent directory.
    pub fn resolve_devices_file(&self) -> std::path::PathBuf {
        if self.devices_file.exists() {
            return self.devices_file.clone();
        }
        let parent = std::path::Path::new("..").join(&self.devices_file);
        if parent.exists() {
            return parent;
        }
        self.devices_file.clone()
    }
}

#[derive(Args, Debug, Clone)]
pub struct SimulateArgs {
    /// NLECloud TCP gateway host
    #[arg(long, env = "NLE_GATEWAY_HOST", default_value = "newgateway.nlecloud.com")]
    pub gateway_host: String,

    /// NLECloud TCP gateway port
    #[arg(long, env = "NLE_GATEWAY_PORT", default_value_t = 8600)]
    pub gateway_port: u16,

    /// Interval in seconds between telemetry data uploads
    #[arg(long, env = "TELEMETRY_INTERVAL_SECS", default_value_t = 5)]
    pub telemetry_interval_secs: u64,

    /// Heartbeat interval in seconds
    #[arg(long, default_value_t = 30)]
    pub heartbeat_interval_secs: u64,

    /// Target device tag to simulate (if omitted, will query cloud and prompt user)
    #[arg(short = 'd', long, env = "NLE_DEVICE_TAG")]
    pub device_tag: Option<String>,

    /// Communication security key of the device (if omitted, fetched from cloud API)
    #[arg(short = 'k', long, env = "NLE_SECRET_KEY")]
    pub secret_key: Option<String>,

    #[command(flatten)]
    pub project: ProjectArgs,
}
