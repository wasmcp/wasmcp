use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod build;
mod compose;
mod deps;
mod release;
mod util;

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
    /// Check versions and trigger releases
    Release {
        #[command(subcommand)]
        action: ReleaseAction,
    },
    /// CLI-specific commands
    Cli {
        #[command(subcommand)]
        action: CliAction,
    },
}

#[derive(Subcommand)]
enum CliAction {
    /// Sync versions.toml with latest published component versions
    SyncVersions {
        /// Show what would be updated without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ReleaseAction {
    /// Check version consistency between Cargo.toml and world.wit
    Check {
        /// Specific component to check (default: all)
        component: Option<String>,
    },
    /// Check which components need releases (read-only)
    Status {
        /// Show all components including up-to-date ones
        #[arg(short, long)]
        verbose: bool,
    },
    /// Bump component version (Cargo.toml + world.wit package version)
    Bump {
        /// Component name
        component: String,

        /// Bump patch version (0.1.5 -> 0.1.6) [default]
        #[arg(long)]
        patch: bool,

        /// Bump minor version (0.1.5 -> 0.2.0)
        #[arg(long)]
        minor: bool,

        /// Bump major version (0.1.5 -> 1.0.0)
        #[arg(long)]
        major: bool,

        /// Set explicit version
        #[arg(long)]
        version: Option<String>,

        /// Show changes without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Update MCP WIT dependencies (world.wit imports/exports + deps.toml)
    UpdateDeps {
        /// Component name (optional - updates all if not specified)
        component: Option<String>,

        /// Set explicit MCP WIT version
        #[arg(long)]
        version: Option<String>,

        /// Update to latest published MCP WIT
        #[arg(long)]
        latest: bool,

        /// Show changes without modifying files
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Publish releases (auto-detect or manual component list)
    Publish {
        /// Component names (if empty, auto-detect all that need releases)
        components: Vec<String>,

        /// Show what would be triggered without actually triggering
        #[arg(long)]
        dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,

        /// Show detailed release information for each component
        #[arg(short, long)]
        verbose: bool,
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
        Commands::Release { action } => match action {
            ReleaseAction::Check { component } => {
                release::check_versions(component)?;
            }
            ReleaseAction::Status { verbose } => {
                release::show_status(verbose)?;
            }
            ReleaseAction::Bump {
                component,
                patch,
                minor,
                major,
                version,
                dry_run,
                force,
            } => {
                release::bump_version(component, patch, minor, major, version, dry_run, force)?;
            }
            ReleaseAction::UpdateDeps {
                component,
                version,
                latest,
                dry_run,
                force,
            } => {
                release::update_deps(component, version, latest, dry_run, force)?;
            }
            ReleaseAction::Publish {
                components,
                dry_run,
                force,
                verbose,
            } => {
                release::publish_releases(components, dry_run, force, verbose)?;
            }
        },
        Commands::Cli { action } => match action {
            CliAction::SyncVersions { dry_run, force } => {
                release::sync_versions(dry_run, force)?;
            }
        },
    }

    Ok(())
}
