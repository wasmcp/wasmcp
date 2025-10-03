mod compose;
mod pkg;
mod scaffold;

use anyhow::{Context, Result};
use clap::{Args, Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "wasmcp",
    about = "CLI for scaffolding Model Context Protocol servers as WebAssembly components",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Component override arguments for testing with local or custom components
#[derive(Args, Debug)]
struct ComponentOverrideArgs {
    /// Override request component (path or package spec)
    #[arg(long = "override-request")]
    override_request: Option<String>,

    /// Override transport component (path or package spec)
    #[arg(long = "override-transport")]
    override_transport: Option<String>,

    /// Override initialize-handler component (path or package spec)
    #[arg(long = "override-initialize-handler")]
    override_initialize_handler: Option<String>,

    /// Override initialize-writer component (path or package spec)
    #[arg(long = "override-initialize-writer")]
    override_initialize_writer: Option<String>,

    /// Override error-writer component (path or package spec)
    #[arg(long = "override-error-writer")]
    override_error_writer: Option<String>,

    /// Override tools-writer component (path or package spec)
    #[arg(long = "override-tools-writer")]
    override_tools_writer: Option<String>,

    /// Override resources-writer component (path or package spec)
    #[arg(long = "override-resources-writer")]
    override_resources_writer: Option<String>,

    /// Override prompts-writer component (path or package spec)
    #[arg(long = "override-prompts-writer")]
    override_prompts_writer: Option<String>,

    /// Override completion-writer component (path or package spec)
    #[arg(long = "override-completion-writer")]
    override_completion_writer: Option<String>,
}

impl From<ComponentOverrideArgs> for compose::ComponentOverrides {
    fn from(args: ComponentOverrideArgs) -> Self {
        compose::ComponentOverrides {
            request: args.override_request,
            transport: args.override_transport,
            initialize_handler: args.override_initialize_handler,
            initialize_writer: args.override_initialize_writer,
            error_writer: args.override_error_writer,
            tools_writer: args.override_tools_writer,
            resources_writer: args.override_resources_writer,
            prompts_writer: args.override_prompts_writer,
            completion_writer: args.override_completion_writer,
        }
    }
}

#[derive(Parser)]
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Create a new MCP server project
    New {
        /// Project name (e.g., my-server)
        name: String,

        /// Handler type (tools, resources, prompts, or completion)
        #[arg(long, short = 't', value_name = "TYPE")]
        r#type: HandlerTypeArg,

        /// Programming language (rust, go, typescript, or python)
        #[arg(long, short = 'l', value_name = "LANG")]
        language: Language,

        /// wasmcp version to use
        #[arg(long, default_value = "0.3.0")]
        version: String,

        /// Overwrite existing directory
        #[arg(long)]
        force: bool,

        /// Output directory (defaults to current directory)
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },

    /// Compose a complete MCP server from handler components
    Compose {
        /// Middleware components (path or package spec like 'namespace:name@version')
        #[arg(long)]
        middleware: Vec<String>,

        /// Tools handlers (path or package spec like 'namespace:name@version')
        #[arg(long)]
        tools: Vec<String>,

        /// Resources handlers (path or package spec like 'namespace:name@version')
        #[arg(long)]
        resources: Vec<String>,

        /// Prompts handlers (path or package spec like 'namespace:name@version')
        #[arg(long)]
        prompts: Vec<String>,

        /// Completion handlers (path or package spec like 'namespace:name@version')
        #[arg(long)]
        completion: Vec<String>,

        /// Transport type (http or stdio)
        #[arg(long, short = 't', default_value = "http")]
        transport: Transport,

        /// Output path for the composed server
        #[arg(long, short = 'o', default_value = "mcp-server.wasm")]
        output: PathBuf,

        /// wasmcp version for dependencies
        #[arg(long, default_value = "0.3.0")]
        version: String,

        /// Directory for dependency components
        #[arg(long, default_value = "deps")]
        deps_dir: PathBuf,

        /// Skip downloading dependencies (use existing)
        #[arg(long)]
        skip_download: bool,

        /// Overwrite existing output file
        #[arg(long)]
        force: bool,

        /// Component overrides for testing with local or custom components
        #[command(flatten)]
        overrides: ComponentOverrideArgs,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "lowercase")]
enum Language {
    Rust,
    Go,
    TypeScript,
    Python,
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

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "lowercase")]
enum HandlerTypeArg {
    Middleware,
    Tools,
    Resources,
    Prompts,
    Completion,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "rust"),
            Language::Go => write!(f, "go"),
            Language::TypeScript => write!(f, "typescript"),
            Language::Python => write!(f, "python"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HandlerType {
    Middleware,
    Tools,
    Resources,
    Prompts,
    Completion,
}

impl std::fmt::Display for HandlerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerType::Middleware => write!(f, "middleware"),
            HandlerType::Tools => write!(f, "tools"),
            HandlerType::Resources => write!(f, "resources"),
            HandlerType::Prompts => write!(f, "prompts"),
            HandlerType::Completion => write!(f, "completion"),
        }
    }
}

impl HandlerType {
    /// Get the interface name for WIT (completion is singular, others are plural)
    pub fn interface_name(&self) -> &str {
        match self {
            HandlerType::Middleware => "middleware",
            HandlerType::Tools => "tools",
            HandlerType::Resources => "resources",
            HandlerType::Prompts => "prompts",
            HandlerType::Completion => "completion",
        }
    }
}

impl From<HandlerTypeArg> for HandlerType {
    fn from(arg: HandlerTypeArg) -> Self {
        match arg {
            HandlerTypeArg::Middleware => HandlerType::Middleware,
            HandlerTypeArg::Tools => HandlerType::Tools,
            HandlerTypeArg::Resources => HandlerType::Resources,
            HandlerTypeArg::Prompts => HandlerType::Prompts,
            HandlerTypeArg::Completion => HandlerType::Completion,
        }
    }
}

impl From<HandlerType> for compose::HandlerType {
    fn from(ht: HandlerType) -> Self {
        match ht {
            HandlerType::Middleware => compose::HandlerType::Middleware,
            HandlerType::Tools => compose::HandlerType::Tools,
            HandlerType::Resources => compose::HandlerType::Resources,
            HandlerType::Prompts => compose::HandlerType::Prompts,
            HandlerType::Completion => compose::HandlerType::Completion,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::New {
            name,
            r#type,
            language,
            version,
            force,
            output,
        } => {
            let handler_type = HandlerType::from(r#type);

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
            scaffold::create_project(&output_dir, &name, handler_type, language, &version)
                .context("Failed to create project")?;

            println!("Created {} {} handler in {}", language, handler_type, name);
            println!("\nNext steps:");
            println!("  cd {}", name);
            println!("  make build");

            Ok(())
        }

        Command::Compose {
            middleware,
            tools,
            resources,
            prompts,
            completion,
            transport,
            output,
            version,
            deps_dir,
            skip_download,
            force,
            overrides,
        } => {
            // Build ordered list of handlers if any explicit flags are provided
            let handlers = if middleware.is_empty()
                && tools.is_empty()
                && resources.is_empty()
                && prompts.is_empty()
                && completion.is_empty()
            {
                // Auto-discovery mode - pass empty vec
                Vec::new()
            } else {
                // Reconstruct the order from command line arguments
                let args: Vec<String> = std::env::args().collect();
                let mut handlers: Vec<(usize, HandlerType, String)> = Vec::new();

                for (i, arg) in args.iter().enumerate() {
                    let handler_type = match arg.as_str() {
                        "--middleware" => Some(HandlerType::Middleware),
                        "--tools" => Some(HandlerType::Tools),
                        "--resources" => Some(HandlerType::Resources),
                        "--prompts" => Some(HandlerType::Prompts),
                        "--completion" => Some(HandlerType::Completion),
                        _ => None,
                    };

                    if let Some(htype) = handler_type {
                        if let Some(value) = args.get(i + 1) {
                            if !value.starts_with("--") {
                                handlers.push((i, htype, value.clone()));
                            }
                        }
                    }
                }

                // Sort by position to preserve command line order
                handlers.sort_by_key(|(pos, _, _)| *pos);

                // Convert to compose::HandlerType and extract tuples
                handlers
                    .into_iter()
                    .map(|(_, ht, s)| (compose::HandlerType::from(ht), s))
                    .collect()
            };

            // Create compose options
            let options = compose::ComposeOptions {
                handlers,
                transport: transport.to_string(),
                output,
                version,
                deps_dir,
                skip_download,
                force,
                overrides: overrides.into(),
            };

            compose::compose(options).await
        }
    }
}

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
        assert!(validate_project_name("my-server").is_ok());
        assert!(validate_project_name("my_server").is_ok());
        assert!(validate_project_name("myserver123").is_ok());

        assert!(validate_project_name("").is_err());
        assert!(validate_project_name("-server").is_err());
        assert!(validate_project_name("_server").is_err());
        assert!(validate_project_name("my server").is_err());
        assert!(validate_project_name("my@server").is_err());
    }
}
