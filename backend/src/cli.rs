use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "backend", version, about = "Backend CLI application")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Start the Axum WebSocket / HTTP server
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
}

#[derive(Args, Debug, Clone)]
pub struct ProjectArgs {
    /// Name of the project (can also be set via PROJECT_NAME or NLE_PROJECT_NAME env var)
    #[arg(
        short,
        long,
        env = "PROJECT_NAME",
        default_value = "smart-home-dock"
    )]
    pub name: String,

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
        if self.name == "smart-home-dock" {
            if let Ok(val) = std::env::var("NLE_PROJECT_NAME") {
                if !val.trim().is_empty() {
                    return val;
                }
            }
        }
        self.name.clone()
    }
}
