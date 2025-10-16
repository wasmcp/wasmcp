mod compose;
mod pkg;
mod scaffold;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "wasmcp",
    about = "CLI for scaffolding and composing Model Context Protocol servers as WebAssembly components",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// Create a new MCP server handler component
    New {
        /// Project name (e.g., my-server)
        name: String,

        /// Programming language
        #[arg(long, short = 'l', value_name = "LANG")]
        language: Language,

        /// wasmcp version to use for WIT dependencies
        #[arg(long, default_value = "0.4.0")]
        version: String,

        /// Overwrite existing directory
        #[arg(long)]
        force: bool,

        /// Output directory (defaults to current directory)
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },

    /// Compose handler components into a complete MCP server
    ///
    /// Components are composed in a linear middleware pipeline:
    ///   transport → component₁ → component₂ → ... → method-not-found
    ///
    /// Each component can handle specific MCP methods and delegates unknown
    /// requests to the next component in the chain.
    ///
    /// Components can be specified as:
    ///   - Local paths: ./my-handler.wasm or /abs/path/handler.wasm
    ///   - Package specs: wasmcp:calculator@0.1.0 or namespace:name@version
    ///
    /// Example:
    ///   wasmcp compose ./string-tools.wasm wasmcp:calculator@0.1.0 -o server.wasm
    Compose {
        /// Handler components in pipeline order (paths or package specs)
        ///
        /// Components are composed left-to-right into a middleware chain.
        /// Each component processes requests and delegates unknowns downstream.
        #[arg(required = true)]
        components: Vec<String>,

        /// Transport type (http or stdio)
        #[arg(long, short = 't', default_value = "http")]
        transport: Transport,

        /// Output path for the composed server
        #[arg(long, short = 'o', default_value = "mcp-server.wasm")]
        output: PathBuf,

        /// wasmcp version for framework dependencies
        #[arg(long, default_value = "0.4.0")]
        version: String,

        /// Override transport component (path or package spec)
        #[arg(long)]
        override_transport: Option<String>,

        /// Override method-not-found component (path or package spec)
        #[arg(long)]
        override_method_not_found: Option<String>,

        /// Directory for dependency components
        #[arg(long, default_value = "deps")]
        deps_dir: PathBuf,

        /// Skip downloading dependencies (use existing)
        #[arg(long)]
        skip_download: bool,

        /// Overwrite existing output file
        #[arg(long)]
        force: bool,
    },

    /// WIT dependency management commands
    Wit {
        #[command(subcommand)]
        command: WitCommand,
    },
}

#[derive(Parser)]
enum WitCommand {
    /// Fetch WIT dependencies for a project
    ///
    /// This downloads all transitive WIT dependencies declared in your
    /// wit/deps.toml file to wit/deps/, similar to `wkg wit fetch`.
    Fetch {
        /// Directory containing wit/ folder
        #[arg(long, default_value = ".")]
        dir: PathBuf,

        /// Update dependencies to latest compatible versions
        #[arg(long)]
        update: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
#[value(rename_all = "lowercase")]
enum Language {
    Rust,
    Python,
    TypeScript,
    // Go template coming soon (blocked on wit-bindgen-go bug)
    // Go,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "rust"),
            Language::Python => write!(f, "python"),
            Language::TypeScript => write!(f, "typescript"),
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "lowercase")]
enum Transport {
    Http,
    Stdio,
}

impl std::fmt::Display for Transport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Transport::Http => write!(f, "http"),
            Transport::Stdio => write!(f, "stdio"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::New {
            name,
            language,
            version,
            force,
            output,
        } => {
            // Validate project name
            validate_project_name(&name)?;

            // Determine output directory
            let output_dir = if let Some(output) = output {
                output.join(&name)
            } else {
                PathBuf::from(&name)
            };

            // Check if directory exists
            if output_dir.exists() && !force {
                anyhow::bail!(
                    "Directory '{}' already exists. Use --force to overwrite.",
                    output_dir.display()
                );
            }

            // Scaffold the project
            scaffold::create_project(&output_dir, &name, language, &version)
                .await
                .context("Failed to create project")?;

            println!("Created {} handler in {}", language, name);

            // Determine the output path based on language
            let component_path = match language {
                Language::Python => format!("{}.wasm", name),
                Language::Rust => format!("target/wasm32-wasip2/release/{}.wasm", name),
                Language::TypeScript => format!("dist/{}.wasm", name),
            };

            println!("\nNext steps:");
            println!("  cd {}", name);
            println!("  make          # Build the component");
            println!("  wasmcp compose {} -o server.wasm", component_path);

            Ok(())
        }

        Command::Compose {
            components,
            transport,
            output,
            version,
            override_transport,
            override_method_not_found,
            deps_dir,
            skip_download,
            force,
        } => {
            // Create compose options
            let options = compose::ComposeOptions {
                components,
                transport: transport.to_string(),
                output,
                version,
                override_transport,
                override_method_not_found,
                deps_dir,
                skip_download,
                force,
            };

            compose::compose(options).await
        }

        Command::Wit { command } => match command {
            WitCommand::Fetch { dir, update } => {
                // Validate directory exists
                if !dir.exists() {
                    anyhow::bail!("Directory '{}' does not exist", dir.display());
                }

                // Check if wit/ directory exists
                let wit_dir = dir.join("wit");
                if !wit_dir.exists() {
                    anyhow::bail!(
                        "Directory '{}' does not contain a wit/ folder",
                        dir.display()
                    );
                }

                // Fetch WIT dependencies
                pkg::fetch_wit_dependencies(&dir, update)
                    .await
                    .context("Failed to fetch WIT dependencies")?;

                Ok(())
            }
        },
    }
}

/// Validate that a project name is acceptable
///
/// Project names must:
/// - Be non-empty
/// - Contain only alphanumeric characters, hyphens, and underscores
/// - Not start with a hyphen or underscore
fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Project name cannot be empty");
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!(
            "Project name can only contain alphanumeric characters, hyphens, and underscores"
        );
    }

    if name.starts_with('-') || name.starts_with('_') {
        anyhow::bail!("Project name cannot start with a hyphen or underscore");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_project_name() {
        // Valid names
        assert!(validate_project_name("my-server").is_ok());
        assert!(validate_project_name("my_server").is_ok());
        assert!(validate_project_name("myserver123").is_ok());
        assert!(validate_project_name("MyServer").is_ok());

        // Invalid names
        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("-server").is_err());
        assert!(validate_project_name("_server").is_err());
        assert!(validate_project_name("my server").is_err());
        assert!(validate_project_name("my@server").is_err());
        assert!(validate_project_name("my.server").is_err());
    }
}
