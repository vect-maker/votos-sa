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
}

#[derive(Args, Debug)]
pub struct ServeArgs {
    /// Host address to bind the server to
    #[arg(short = 'H', long, default_value = "0.0.0.0")]
    pub host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 3000)]
    pub port: u16,
}
