mod compose;
mod config;
mod pkg;
mod scaffold;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

/// Get the default deps directory from config, or fall back to a relative path
fn default_deps_dir() -> PathBuf {
    config::get_deps_dir().unwrap_or_else(|_| PathBuf::from("deps"))
}

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
    ///   - Profiles: Use --profile to load component lists
    ///   - Aliases: Short names registered in config
    ///   - Local paths: ./my-handler.wasm or /abs/path/handler.wasm
    ///   - Package specs: wasmcp:calculator@0.1.0 or namespace:name@version
    ///
    /// Examples:
    ///   wasmcp compose -p dev-server
    ///   wasmcp compose -p base-server calc strings
    ///   wasmcp compose ./string-tools.wasm wasmcp:calculator@0.1.0 -o server.wasm
    Compose {
        /// Profile(s) to use for composition
        ///
        /// Profiles define reusable component pipelines with settings.
        /// Multiple profiles can be chained: -p base -p auth -p metrics
        #[arg(long, short = 'p', value_name = "NAME")]
        profile: Vec<String>,

        /// Handler components in pipeline order (paths, aliases, or package specs)
        ///
        /// Components are composed left-to-right into a middleware chain.
        /// Each component processes requests and delegates unknowns downstream.
        /// When used with --profile, these components are appended after profile components.
        #[arg(required_unless_present = "profile")]
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
        #[arg(long, default_value_os_t = default_deps_dir())]
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

    /// Registry management commands for component aliases and profiles
    Registry {
        #[command(subcommand)]
        command: RegistryCommand,
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

#[derive(Parser)]
enum RegistryCommand {
    /// Manage component aliases
    Component {
        #[command(subcommand)]
        command: ComponentCommand,
    },

    /// Manage compose profiles
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },

    /// Show registry configuration information
    ///
    /// Displays paths and statistics about the registry.
    Info,

    /// List registered components and profiles
    ///
    /// By default, shows both components and profiles.
    /// Use --components or --profiles to filter.
    List {
        /// Show only component aliases
        #[arg(long, conflicts_with = "profiles")]
        components: bool,

        /// Show only profiles
        #[arg(long, conflicts_with = "components")]
        profiles: bool,
    },
}

#[derive(Parser)]
enum ComponentCommand {
    /// Register a component alias for easier composition
    ///
    /// Aliases can reference:
    /// - Local paths: ./my-handler.wasm
    /// - Registry packages: wasmcp:calculator@0.1.0
    /// - Other aliases (will be resolved recursively)
    ///
    /// Examples:
    ///   wasmcp registry component add calc wasmcp:calculator@0.1.0
    ///   wasmcp registry component add my-calc ./calculator.wasm
    ///   wasmcp registry component add prod-calc calc
    Add {
        /// Alias name (e.g., "calc", "strings")
        alias: String,

        /// Component spec (path, package spec, or another alias)
        spec: String,
    },

    /// Unregister a component alias
    ///
    /// Example:
    ///   wasmcp registry component remove calc
    Remove {
        /// Alias name to remove
        alias: String,
    },

    /// List registered component aliases
    ///
    /// Example:
    ///   wasmcp registry component list
    List,
}

#[derive(Parser)]
enum ProfileCommand {
    /// Create a new profile
    ///
    /// Profiles define reusable component pipelines.
    ///
    /// Example:
    ///   wasmcp registry profile add dev-server one two -o dev.wasm
    ///   wasmcp registry profile add prod one two -o prod.wasm -b dev-server
    Add {
        /// Profile name
        name: String,

        /// Components in this profile (aliases, paths, or registry specs)
        components: Vec<String>,

        /// Output filename (saved to ~/.config/wasmcp/composed/ by default)
        #[arg(long, short = 'o')]
        output: String,

        /// Optional base profile to inherit from
        #[arg(long, short = 'b')]
        base: Option<String>,
    },

    /// Delete a profile
    ///
    /// Example:
    ///   wasmcp registry profile remove dev-server
    Remove {
        /// Profile name to remove
        name: String,
    },

    /// List all profiles
    ///
    /// Example:
    ///   wasmcp registry profile list
    List,
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
            println!("\nNext steps:");
            println!("  cd {}", name);
            println!("  make");
            println!("  wasmcp compose <your-handler.wasm> -o server.wasm");

            Ok(())
        }

        Command::Compose {
            profile,
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
            // If profiles are specified, resolve them with component mixing
            let (resolved_components, profile_settings) = if !profile.is_empty() {
                let (comps, settings) = compose::compose_profiles_and_components(&profile, &components)?;
                (comps, settings)
            } else {
                (components, None)
            };

            // Use profile settings as defaults if available, CLI args override
            let final_transport = transport.to_string();
            let final_output = output;
            let final_version = version;
            let final_override_transport = override_transport;
            let final_override_method_not_found = override_method_not_found;
            let final_force = force;

            // Create compose options
            let options = compose::ComposeOptions {
                components: resolved_components,
                transport: final_transport,
                output: final_output,
                version: final_version,
                override_transport: final_override_transport,
                override_method_not_found: final_override_method_not_found,
                deps_dir,
                skip_download,
                force: final_force,
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

        Command::Registry { command } => match command {
            RegistryCommand::Component { command } => match command {
                ComponentCommand::Add { alias, spec } => {
                    config::register_component(&alias, &spec)
                        .context("Failed to register component")?;

                    println!("✅ Registered alias: {} → {}", alias, spec);
                    Ok(())
                }

                ComponentCommand::Remove { alias } => {
                    config::unregister_component(&alias)
                        .context("Failed to unregister component")?;

                    println!("✅ Unregistered alias: {}", alias);
                    Ok(())
                }

                ComponentCommand::List => {
                    let cfg = config::load_config()
                        .context("Failed to load config")?;

                    if cfg.components.is_empty() {
                        println!("No components registered.");
                        println!("\nTo register a component:");
                        println!("  wasmcp registry component add <alias> <spec>");
                    } else {
                        println!("Components:");
                        let mut aliases: Vec<_> = cfg.components.iter().collect();
                        aliases.sort_by_key(|(name, _)| *name);
                        for (alias, spec) in aliases {
                            println!("  {} → {}", alias, spec);
                        }
                    }

                    Ok(())
                }
            }

            RegistryCommand::Profile { command } => match command {
                ProfileCommand::Add {
                    name,
                    components,
                    output,
                    base,
                } => {
                    let profile = config::Profile {
                        base,
                        components,
                        output,
                    };

                    config::create_profile(&name, profile)
                        .context("Failed to create profile")?;

                    println!("✅ Created profile: {}", name);
                    Ok(())
                }

                ProfileCommand::Remove { name } => {
                    config::delete_profile(&name)
                        .context("Failed to delete profile")?;

                    println!("✅ Deleted profile: {}", name);
                    Ok(())
                }

                ProfileCommand::List => {
                    let cfg = config::load_config()
                        .context("Failed to load config")?;

                    if cfg.profiles.is_empty() {
                        println!("No profiles registered.");
                        println!("\nTo create a profile:");
                        println!("  wasmcp registry profile add <name> <components...> -o <output>");
                    } else {
                        println!("Profiles:");
                        let mut profile_names: Vec<_> = cfg.profiles.keys().collect();
                        profile_names.sort();
                        for name in profile_names {
                            let profile = &cfg.profiles[name];
                            println!("\n  {}", name);
                            if let Some(base) = &profile.base {
                                println!("    Base: {}", base);
                            }
                            println!("    Components: {}", profile.components.join(", "));
                            println!("    Output: {}", profile.output);
                        }
                    }

                    Ok(())
                }
            }

            RegistryCommand::List {
                components,
                profiles,
            } => {
                let cfg = config::load_config()
                    .context("Failed to load config")?;

                let show_components = !profiles; // Show components unless --profiles is set
                let show_profiles = !components;  // Show profiles unless --components is set

                if show_components && !cfg.components.is_empty() {
                    println!("Components:");
                    let mut aliases: Vec<_> = cfg.components.iter().collect();
                    aliases.sort_by_key(|(name, _)| *name);
                    for (alias, spec) in aliases {
                        println!("  {} → {}", alias, spec);
                    }
                }

                if show_profiles && !cfg.profiles.is_empty() {
                    if show_components && !cfg.components.is_empty() {
                        println!(); // Blank line between sections
                    }
                    println!("Profiles:");
                    let mut profile_names: Vec<_> = cfg.profiles.keys().collect();
                    profile_names.sort();
                    for name in profile_names {
                        let profile = &cfg.profiles[name];
                        println!("  {}", name);
                        if let Some(base) = &profile.base {
                            println!("    Base: {}", base);
                        }
                        if !profile.components.is_empty() {
                            println!("    Components: {}", profile.components.join(", "));
                        }
                        println!("    Output: {}", profile.output);
                    }
                }

                if cfg.components.is_empty() && cfg.profiles.is_empty() {
                    println!("No components or profiles registered.");
                    println!("\nTo register a component:");
                    println!("  wasmcp registry component add <alias> <spec>");
                }

                Ok(())
            }

            RegistryCommand::Info => {
                let wasmcp_dir = config::get_wasmcp_dir()?;
                let config_path = config::get_config_path()?;
                let cache_dir = config::get_cache_dir()?;
                let deps_dir = config::get_deps_dir()?;
                let composed_dir = config::get_composed_dir()?;

                println!("wasmcp Registry Information");
                println!();
                println!("Config file:     {}", config_path.display());
                println!("Root directory:  {}", wasmcp_dir.display());
                println!("Cache directory: {}", cache_dir.display());
                println!("Deps directory:  {}", deps_dir.display());
                println!("Output directory: {}", composed_dir.display());

                // Show config stats
                match config::load_config() {
                    Ok(cfg) => {
                        println!();
                        println!("Statistics:");
                        println!("  Components: {}", cfg.components.len());
                        println!("  Profiles:   {}", cfg.profiles.len());
                    }
                    Err(_) => {
                        println!();
                        println!("No configuration file found.");
                    }
                }

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
