// Use modules from the library crate
use wasmcp::{commands, config, types};

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;
use types::{Language, TemplateType, Transport};

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

        /// Template type (tools or resources)
        #[arg(long, short = 't', value_name = "TYPE", default_value = "tools")]
        template_type: TemplateType,

        /// Overwrite existing directory
        #[arg(long)]
        force: bool,

        /// Output directory (defaults to current directory)
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },

    /// Compose handler components
    ///
    /// Choose the appropriate subcommand:
    ///   - Use 'server' when you want a runnable MCP server
    ///   - Use 'handler' when building reusable middleware
    ///
    /// Subcommands:
    ///   server   - Compose a complete MCP server (with transport and terminal handler)
    ///   handler  - Compose a handler component (without transport/terminal)
    ///
    /// Components can be specified in multiple formats:
    ///
    ///   Registry Packages (OCI):
    ///     namespace:name[@version]  - Downloaded from OCI registry
    ///     wasmcp:calculator@0.1.0   - With version (recommended)
    ///     wasmcp:calculator         - Latest version
    ///     Note: Colon (:) is the key identifier for registry packages
    ///
    ///   Local Paths:
    ///     ./my-handler.wasm         - Relative path
    ///     ../target/handler.wasm    - Parent directory
    ///     /abs/path/handler.wasm    - Absolute path
    ///     ~/handler.wasm            - Home directory
    ///     handler.wasm              - Current directory
    ///
    ///   Aliases:
    ///     calc                      - Resolves via ~/.config/wasmcp/wasmcp.toml
    ///
    ///   Profiles:
    ///     dev-server                - Expands to multiple components
    ///
    /// Resolution order: profile → alias → path → registry package
    /// Detection: Contains ':' = registry, contains '/' or ends '.wasm' = path
    ///
    /// Examples:
    ///   wasmcp compose server wasmcp:calculator@0.1.0   # Complete server
    ///   wasmcp compose handler calc.wasm math.wasm      # Handler component
    ///   wasmcp compose server calc strings              # Multiple handlers
    #[command(subcommand)]
    Compose(ComposeCommand),

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

    /// Model Context Protocol (MCP) server commands
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
}

#[derive(Parser)]
enum ComposeCommand {
    /// Compose a complete MCP server
    ///
    /// Creates a runnable MCP server with transport layer and terminal handler:
    ///   transport → component₁ → component₂ → ... → method-not-found
    ///
    /// Output path resolution (highest priority wins):
    ///   1. Explicit -o flag: Always used when provided
    ///   2. Profile output: Uses the last profile's configured output path
    ///   3. Default: "mcp-server.wasm" when no profile or -o flag is specified
    ///
    /// Examples:
    ///   wasmcp compose server wasmcp:calculator@0.1.0
    ///   wasmcp compose server dev                        # Profile
    ///   wasmcp compose server calc strings               # Aliases
    ///   wasmcp compose server ./handler.wasm
    Server {
        /// (Optional) Profile(s) for backward compatibility with -p flag
        ///
        /// NOTE: Profiles can also be specified directly in the components list.
        /// This flag exists for backward compatibility.
        /// Multiple profiles: -p base -p auth (prepended to components list)
        #[arg(long, short = 'p', value_name = "NAME")]
        profile: Vec<String>,

        /// Components to compose (profile names, aliases, paths, or package specs)
        ///
        /// Each spec is resolved as: profile → alias → path → registry package.
        /// Profile names expand in-place to their components.
        /// Order is preserved: component order matches the pipeline order.
        #[arg(required_unless_present = "profile")]
        components: Vec<String>,

        /// Transport type (http or stdio)
        #[arg(long, short = 't', default_value = "http")]
        transport: Transport,

        /// Output path for the composed server
        ///
        /// Relative paths are resolved from the current working directory.
        /// If not specified, uses the profile's output setting (saved in ~/.config/wasmcp/composed/).
        /// Otherwise defaults to "mcp-server.wasm" in the current directory.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        /// Version overrides for specific components (e.g., --version-override transport=0.2.0)
        #[arg(long = "version-override", value_name = "COMPONENT=VERSION")]
        version_overrides: Vec<String>,

        /// Override transport component (path or package spec)
        #[arg(long)]
        override_transport: Option<String>,

        /// Override server-io component (path or package spec)
        #[arg(long)]
        override_server_io: Option<String>,

        /// Override session-store component (path or package spec)
        #[arg(long)]
        override_session_store: Option<String>,

        /// Override method-not-found component (path or package spec)
        #[arg(long)]
        override_method_not_found: Option<String>,

        /// Override tools-middleware component (path or package spec)
        #[arg(long)]
        override_tools_middleware: Option<String>,

        /// Override resources-middleware component (path or package spec)
        #[arg(long)]
        override_resources_middleware: Option<String>,

        /// Override prompts-middleware component (path or package spec)
        #[arg(long)]
        override_prompts_middleware: Option<String>,

        /// Directory for dependency components
        #[arg(long, default_value_os_t = default_deps_dir())]
        deps_dir: PathBuf,

        /// Skip downloading dependencies (use existing)
        #[arg(long)]
        skip_download: bool,

        /// Overwrite existing output file
        #[arg(long)]
        force: bool,

        /// Enable verbose output (show detailed resolution and composition steps)
        #[arg(long, short = 'v')]
        verbose: bool,
    },

    /// Compose a handler component (composable middleware without transport)
    ///
    /// Creates an intermediate handler component that exports wasmcp:server/handler
    /// but does NOT include transport or terminal components. This is useful for:
    ///   - Building reusable middleware layers
    ///   - Creating multi-component tools that can be composed into servers
    ///   - Orchestrating multiple downstream components
    ///
    /// Component chain structure:
    ///   component₁ → component₂ → ... → componentₙ
    ///
    /// Unlike 'compose server', this does NOT create a runnable server.
    /// The output must be composed into a server with 'compose server'.
    ///
    /// Examples:
    ///   # Build distance-calculator with math tools
    ///   wasmcp compose handler distance-calc.wasm wasmcp:math@0.1.0 -o dist-calc.wasm
    ///
    ///   # Then use in a server
    ///   wasmcp compose server dist-calc.wasm other-tools.wasm
    Handler {
        /// (Optional) Profile(s) for backward compatibility with -p flag
        #[arg(long, short = 'p', value_name = "NAME")]
        profile: Vec<String>,

        /// Components to compose (profile names, aliases, paths, or package specs)
        #[arg(required_unless_present = "profile")]
        components: Vec<String>,

        /// Output path for the composed handler
        ///
        /// Defaults to "handler.wasm" in the current directory.
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,

        /// Directory for dependency components
        #[arg(long, default_value_os_t = default_deps_dir())]
        deps_dir: PathBuf,

        /// Overwrite existing output file
        #[arg(long)]
        force: bool,

        /// Enable verbose output (show detailed resolution and composition steps)
        #[arg(long, short = 'v')]
        verbose: bool,
    },
}

#[derive(Parser)]
enum McpCommand {
    /// Run MCP server for AI-assisted wasmcp development
    ///
    /// Runs the server in foreground mode. Logs appear in the terminal.
    /// Press Ctrl+C to stop. Does not create PID files.
    Serve(commands::server::ServerArgs),

    /// Start MCP server as background daemon
    ///
    /// Server runs in the background with PID and log files.
    /// Use 'wasmcp mcp status' to check health.
    Start(commands::server::ServerArgs),

    /// Stop background daemon
    Stop,

    /// Restart background daemon with merged flags
    ///
    /// Merges new flags with saved flags. New flags override saved ones.
    Restart(commands::server::ServerArgs),

    /// Show daemon status and health
    Status,

    /// View daemon logs
    Logs {
        /// Follow log output (like tail -f)
        #[arg(short = 'f', long)]
        follow: bool,
    },

    /// Clean up daemon state files
    Clean,
}

#[derive(Parser)]
enum WitCommand {
    /// Fetch WIT dependencies for a project
    ///
    /// This downloads all WIT dependencies declared in your wit/deps.toml
    /// file to wit/deps/. Uses the embedded wit-deps library to fetch from
    /// GitHub URLs or other sources.
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

    /// Show registry information, components, and profiles
    ///
    /// Displays configuration paths, statistics, and lists all registered
    /// components and profiles. Use --components or --profiles to filter.
    Info {
        /// Show only component aliases
        #[arg(long, short = 'c', conflicts_with = "profiles")]
        components: bool,

        /// Show only profiles
        #[arg(long, short = 'p', conflicts_with = "components")]
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
    /// When a base profile is specified with -b, components are inherited:
    /// the base profile's components are included first, followed by the
    /// components specified in this profile. This allows building on top
    /// of common base configurations.
    ///
    /// Examples:
    ///   # Create a base profile:
    ///   wasmcp registry profile add dev-server one two -o dev.wasm
    ///
    ///   # Create a profile that inherits from dev-server:
    ///   # Final pipeline: one → two → three → four
    ///   wasmcp registry profile add prod three four -o prod.wasm -b dev-server
    Add {
        /// Profile name
        name: String,

        /// Components in this profile (aliases, paths, or registry specs)
        ///
        /// These components are appended after any inherited components
        /// from the base profile (if specified with -b).
        components: Vec<String>,

        /// Output filename (saved to ~/.config/wasmcp/composed/ by default)
        #[arg(long, short = 'o')]
        output: String,

        /// Optional base profile to inherit from
        ///
        /// When specified, components from the base profile are included
        /// first in the pipeline, followed by this profile's components.
        /// This allows extending existing profiles with additional components.
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

// Types moved to types.rs module

#[tokio::main]
async fn main() -> Result<()> {
    // Handle internal daemon entry point (macOS spawned process)
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "__internal_daemon__" {
        let port: u16 = args[2].parse().context("Invalid port in daemon args")?;
        let verbose: bool = args[3]
            .parse()
            .context("Invalid verbose flag in daemon args")?;
        let local_resources = if args.len() > 4 {
            Some(PathBuf::from(&args[4]))
        } else {
            None
        };
        return commands::server::daemon::daemon_entry(port, verbose, local_resources).await;
    }

    let cli = Cli::parse();

    match cli.command {
        Command::New {
            name,
            language,
            template_type,
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
                    "directory '{}' already exists, use --force to overwrite",
                    output_dir.display()
                );
            }

            // Scaffold the project
            commands::scaffold::create_project(&output_dir, &name, language, template_type)
                .await
                .context("Failed to create project")?;

            println!(
                "Created {} {} component in {}",
                language, template_type, name
            );

            // Determine the output path based on language
            // Note: Rust converts hyphens to underscores in output filenames
            let component_path = match language {
                Language::Python => format!("{}.wasm", name),
                Language::Rust => {
                    let rust_name = name.replace('-', "_");
                    format!("target/wasm32-wasip2/release/{}.wasm", rust_name)
                }
                Language::TypeScript => format!("dist/{}.wasm", name),
            };

            println!("\nNext steps:");
            println!("  cd {}", name);
            println!("  make          # Build the component");
            println!("  wasmcp compose server {} -o server.wasm", component_path);

            Ok(())
        }

        Command::Compose(compose_cmd) => match compose_cmd {
            ComposeCommand::Server {
                profile,
                components,
                transport,
                output,
                version_overrides,
                override_transport,
                override_server_io,
                override_session_store,
                override_method_not_found,
                override_tools_middleware,
                override_resources_middleware,
                override_prompts_middleware,
                deps_dir,
                skip_download,
                force,
                verbose,
            } => {
                // Merge components from both sources (new unified approach)
                // If -p flags are used, they're prepended to components list for backward compatibility
                let mut all_specs = Vec::new();
                all_specs.extend(profile.iter().cloned());
                all_specs.extend(components.iter().cloned());

                // Expand any profiles found in the specs (in-place expansion)
                let (resolved_components, profile_settings) =
                    commands::compose::expand_profile_specs(&all_specs)?;

                // Determine output path: CLI flag > profile setting > default
                let final_output = match output {
                    Some(path) => {
                        // Explicit -o flag: use relative to current working directory
                        path
                    }
                    None => {
                        if let Some(ref settings) = profile_settings {
                            // Profile setting: use composed directory
                            let composed_dir = config::get_composed_dir()?;
                            composed_dir.join(&settings.output)
                        } else {
                            // Default: use current working directory
                            PathBuf::from("mcp-server.wasm")
                        }
                    }
                };

                // Use other settings as-is
                let final_transport = transport.to_string();
                let final_override_transport = override_transport;
                let final_override_server_io = override_server_io;
                let final_override_session_store = override_session_store;
                let final_override_method_not_found = override_method_not_found;
                let final_force = force;

                // Create version resolver with overrides
                let mut version_resolver = wasmcp::versioning::VersionResolver::new()
                    .context("Failed to create version resolver")?;
                version_resolver
                    .apply_overrides(version_overrides)
                    .context("Failed to apply version overrides")?;

                // Create compose options
                let options = commands::compose::ComposeOptions {
                    components: resolved_components,
                    transport: final_transport,
                    output: final_output,
                    version_resolver,
                    override_transport: final_override_transport,
                    override_server_io: final_override_server_io,
                    override_session_store: final_override_session_store,
                    override_method_not_found: final_override_method_not_found,
                    override_tools_middleware,
                    override_resources_middleware,
                    override_prompts_middleware,
                    deps_dir,
                    skip_download,
                    force: final_force,
                    verbose,
                    mode: commands::compose::CompositionMode::Server,
                };

                commands::compose::compose(options).await
            }

            ComposeCommand::Handler {
                profile,
                components,
                output,
                deps_dir,
                force,
                verbose,
            } => {
                // Merge components from both sources
                let mut all_specs = Vec::new();
                all_specs.extend(profile.iter().cloned());
                all_specs.extend(components.iter().cloned());

                // Expand any profiles found in the specs
                let (resolved_components, _profile_settings) =
                    commands::compose::expand_profile_specs(&all_specs)?;

                // Determine output path: CLI flag > default
                let final_output = output.unwrap_or_else(|| PathBuf::from("handler.wasm"));

                // Create version resolver (uses versions from versions.toml)
                let version_resolver = wasmcp::versioning::VersionResolver::new()
                    .context("Failed to create version resolver")?;

                // Create compose options for handler mode
                let options = commands::compose::ComposeOptions {
                    components: resolved_components,
                    transport: String::new(), // Not used in handler mode
                    output: final_output,
                    version_resolver,
                    override_transport: None,
                    override_server_io: None,
                    override_session_store: None,
                    override_method_not_found: None,
                    override_tools_middleware: None,
                    override_resources_middleware: None,
                    override_prompts_middleware: None,
                    deps_dir,
                    skip_download: false, // Not applicable to handler mode
                    force,
                    verbose,
                    mode: commands::compose::CompositionMode::Handler,
                };

                commands::compose::compose(options).await
            }
        },

        Command::Wit { command } => match command {
            WitCommand::Fetch { dir, update } => {
                // Validate directory exists
                if !dir.exists() {
                    anyhow::bail!("directory '{}' does not exist", dir.display());
                }

                // Check if wit/ directory exists
                let wit_dir = dir.join("wit");
                if !wit_dir.exists() {
                    anyhow::bail!(
                        "directory '{}' does not contain a wit/ folder",
                        dir.display()
                    );
                }

                // Fetch WIT dependencies
                commands::pkg::fetch_wit_dependencies(&dir, update)
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
                    let cfg = config::load_config().context("Failed to load config")?;

                    print_components_list(&cfg);

                    Ok(())
                }
            },

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

                    config::create_profile(&name, profile).context("Failed to create profile")?;

                    println!("✅ Created profile: {}", name);
                    Ok(())
                }

                ProfileCommand::Remove { name } => {
                    config::delete_profile(&name).context("Failed to delete profile")?;

                    println!("✅ Deleted profile: {}", name);
                    Ok(())
                }

                ProfileCommand::List => {
                    let cfg = config::load_config().context("Failed to load config")?;

                    print_profiles_list(&cfg);

                    Ok(())
                }
            },

            RegistryCommand::Info {
                components,
                profiles,
            } => {
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

                // Load config and show everything
                let cfg = config::load_config().context("Failed to load config")?;

                println!();
                println!("Statistics:");
                println!("  Components: {}", cfg.components.len());
                println!("  Profiles:   {}", cfg.profiles.len());

                // Determine what to show based on flags
                let show_components = !profiles; // Show components unless --profiles is set
                let show_profiles = !components; // Show profiles unless --components is set

                // Show components section
                if show_components {
                    println!();
                    print_components_list(&cfg);
                }

                // Show profiles section
                if show_profiles {
                    println!();
                    print_profiles_list(&cfg);
                }

                Ok(())
            }
        },

        Command::Mcp { command } => match command {
            McpCommand::Serve(args) => {
                let mut args = args;
                args.command = None; // Force serve mode
                commands::server::handle_server_command(args).await
            }
            McpCommand::Start(args) => {
                let mut args = args;
                args.command = Some(commands::server::ServerCommand::Start);
                commands::server::handle_server_command(args).await
            }
            McpCommand::Stop => {
                let args = commands::server::ServerArgs {
                    command: Some(commands::server::ServerCommand::Stop),
                    port: 8085,
                    stdio: false,
                    verbose: false,
                    local_resources: None,
                };
                commands::server::handle_server_command(args).await
            }
            McpCommand::Restart(args) => {
                let mut args = args;
                args.command = Some(commands::server::ServerCommand::Restart);
                commands::server::handle_server_command(args).await
            }
            McpCommand::Status => {
                let args = commands::server::ServerArgs {
                    command: Some(commands::server::ServerCommand::Status),
                    port: 8085,
                    stdio: false,
                    verbose: false,
                    local_resources: None,
                };
                commands::server::handle_server_command(args).await
            }
            McpCommand::Logs { follow } => {
                let args = commands::server::ServerArgs {
                    command: Some(commands::server::ServerCommand::Logs { follow }),
                    port: 8085,
                    stdio: false,
                    verbose: false,
                    local_resources: None,
                };
                commands::server::handle_server_command(args).await
            }
            McpCommand::Clean => {
                let args = commands::server::ServerArgs {
                    command: Some(commands::server::ServerCommand::Clean),
                    port: 8085,
                    stdio: false,
                    verbose: false,
                    local_resources: None,
                };
                commands::server::handle_server_command(args).await
            }
        },
    }
}

/// Print components list or empty state message
fn print_components_list(cfg: &config::WasmcpConfig) {
    if cfg.components.is_empty() {
        println!("No components registered.");
        println!("\nTo register components, use:");
        println!("  # From a registry package:");
        println!("  wasmcp registry component add calc wasmcp:calculator@0.1.0");
        println!();
        println!("  # From a local file:");
        println!(
            "  wasmcp registry component add myhandler ./target/wasm32-wasip2/release/handler.wasm"
        );
        println!();
        println!("  # From another alias:");
        println!("  wasmcp registry component add prod-calc calc");
    } else {
        println!("Components:");
        let mut aliases: Vec<_> = cfg.components.iter().collect();
        aliases.sort_by_key(|(name, _)| *name);
        for (alias, spec) in aliases {
            println!("  {} → {}", alias, spec);
        }
    }
}

/// Print profiles list or empty state message
fn print_profiles_list(cfg: &config::WasmcpConfig) {
    if cfg.profiles.is_empty() {
        println!("No profiles registered.");
        println!("\nTo create profiles, use:");
        println!("  # Simple profile:");
        println!("  wasmcp registry profile add dev-server calc strings -o dev.wasm");
        println!();
        println!("  # With inheritance from a base profile:");
        println!("  wasmcp registry profile add prod-server auth db -o prod.wasm -b dev-server");
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
}

/// Validate that a project name is acceptable
///
/// Project names must:
/// - Be non-empty
/// - Contain only alphanumeric characters, hyphens, and underscores
/// - Not start with a hyphen or underscore
fn validate_project_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("project name cannot be empty");
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!(
            "project name can only contain alphanumeric characters, hyphens, and underscores"
        );
    }

    if name.starts_with('-') || name.starts_with('_') {
        anyhow::bail!("project name cannot start with a hyphen or underscore");
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
