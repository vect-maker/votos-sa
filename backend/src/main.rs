mod cli;
pub mod constants;
mod project;
mod server;

use clap::Parser;
use cli::{Cli, Commands, ProjectCommand};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables from .env file (first current dir, then parent dir)
    let _ = dotenvy::dotenv();
    let _ = dotenvy::from_filename("../.env");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve(args) => {
            server::run(&args.host, args.port).await?;
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
    }

    Ok(())
}
