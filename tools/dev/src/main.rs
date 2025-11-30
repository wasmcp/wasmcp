use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod build;
mod compose;
mod deps;

#[derive(Parser)]
#[command(name = "dev-tools")]
#[command(about = "Local development toolkit for wasmcp", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage WIT dependencies (deps.toml)
    Deps {
        #[command(subcommand)]
        action: DepsAction,
    },
    /// Build all workspace components
    Build {
        /// Build only specific components (comma-separated)
        #[arg(short, long)]
        only: Option<String>,
    },
    /// Compose components with local overrides
    Compose {
        /// Component wasm files to compose
        components: Vec<String>,

        /// Output path for composed wasm
        #[arg(short, long, default_value = ".agent/composed.wasm")]
        output: PathBuf,

        /// Force overwrite existing output
        #[arg(short, long)]
        force: bool,

        /// Add local overrides for all core components
        #[arg(long)]
        local_overrides: bool,

        /// Additional wasmcp compose arguments (e.g., --runtime, --override-*, etc.)
        #[arg(last = true)]
        extra_args: Vec<String>,
    },
    /// Run the composed component
    Run {
        /// Runtime to use
        #[arg(value_enum)]
        runtime: Runtime,

        /// Path to the composed wasm
        #[arg(short, long, default_value = ".agent/composed.wasm")]
        wasm: PathBuf,
    },
}

#[derive(Subcommand)]
enum DepsAction {
    /// Patch deps.toml files to use local paths
    Local {
        /// Local WIT directory path (default: ./wit-local)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// Restore original deps.toml files from backups
    Restore,
    /// Show status of deps.toml files
    Status,
}

#[derive(clap::ValueEnum, Clone)]
enum Runtime {
    Spin,
    Wasmtime,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deps { action } => match action {
            DepsAction::Local { path } => {
                let local_path = path.unwrap_or_else(|| PathBuf::from("./wit-local"));
                deps::patch_to_local(&local_path)?;
            }
            DepsAction::Restore => {
                deps::restore_from_backup()?;
            }
            DepsAction::Status => {
                deps::show_status()?;
            }
        },
        Commands::Build { only } => {
            build::build_components(only)?;
        }
        Commands::Compose {
            components,
            output,
            force,
            local_overrides,
            extra_args,
        } => {
            compose::compose_components(components, &output, force, local_overrides, extra_args)?;
        }
        Commands::Run { runtime, wasm } => {
            compose::run_component(runtime, &wasm)?;
        }
    }

    Ok(())
}
