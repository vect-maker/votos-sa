mod cli;
pub mod constants;
mod project;
pub mod seed;
mod server;
mod simulator;
pub mod state;

use clap::Parser;
use cli::{Cli, Commands, ProjectCommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file (first current dir, then parent dir)
    let _ = dotenvy::dotenv();
    let _ = dotenvy::from_filename("../.env");

    let cli = Cli::parse();

    match &cli.command {
        Commands::Simulate(_) => {
            tui_logger::init_logger(tui_logger::LevelFilter::Trace)?;
            tui_logger::set_default_level(tui_logger::LevelFilter::Info);
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "info,backend=trace".into()),
                )
                .with(tui_logger::TuiTracingSubscriberLayer)
                .init();
        }
        _ => {
            tracing_subscriber::registry()
                .with(
                    tracing_subscriber::EnvFilter::try_from_default_env()
                        .unwrap_or_else(|_| "info,axum=debug".into()),
                )
                .with(tracing_subscriber::fmt::layer())
                .init();
        }
    }

    match cli.command {
        Commands::Serve(args) => {
            server::run(args).await?;
        }
        Commands::Project(sub) => match sub {
            ProjectCommand::Provision(args) => {
                project::provision(&args).await?;
            }
            ProjectCommand::Delete(args) => {
                project::delete(&args).await?;
            }
        },
        Commands::Provision(args) => {
            project::provision(&args).await?;
        }
        Commands::Delete(args) => {
            project::delete(&args).await?;
        }
        Commands::Simulate(args) => {
            simulator::run(args).await?;
        }
    }

    Ok(())
}
