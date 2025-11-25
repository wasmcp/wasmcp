use clap::ValueEnum;

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Python,
    TypeScript,
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

#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum TemplateType {
    Tools,
    Resources,
    Prompts,
    RoutingConfig,
}

impl std::fmt::Display for TemplateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateType::Tools => write!(f, "tools"),
            TemplateType::Resources => write!(f, "resources"),
            TemplateType::Prompts => write!(f, "prompts"),
            TemplateType::RoutingConfig => write!(f, "routing-config"),
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum Transport {
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
