//! Project scaffolding for MCP server handler components
//!
//! This module provides functionality to generate new handler component projects
//! from embedded templates. Templates are included at compile-time using include_dir.

use crate::versioning::VersionResolver;
use crate::{Language, TemplateType, commands::pkg};
use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};
use liquid::ParserBuilder;
use std::fs;
use std::path::Path;

// Embed templates at compile time
static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// Create a new MCP handler component project
///
/// This scaffolds a complete project with:
/// - WIT interface definitions
/// - Language-specific handler implementation template
/// - Build configuration (Cargo.toml or equivalent)
/// - Makefile for building
/// - README with usage instructions
///
/// The generated handler uses the universal `server-handler` interface,
/// making it composable with any other handler components.
pub async fn create_project(
    output_dir: &Path,
    name: &str,
    language: Language,
    template_type: TemplateType,
) -> Result<()> {
    // Create output directory
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    // Get wasmcp version from version resolver
    let resolver = VersionResolver::new().context("Failed to create version resolver")?;
    let wasmcp_version = resolver
        .get_version("mcp-v20250618")
        .context("Failed to get mcp-v20250618 version")?;

    // Create template context
    let package_name = name.replace('-', "_");

    let context = liquid::object!({
        "project_name": name,
        "package_name": package_name,
        "wasmcp_version": wasmcp_version,
        "language": language.to_string(),
    });

    // Create liquid parser
    let parser = ParserBuilder::with_stdlib().build()?;

    // Get template directory for the language and type
    let template_path = format!("{}-{}", language, template_type);
    let template_dir = TEMPLATES.get_dir(&template_path).ok_or_else(|| {
        anyhow::anyhow!(
            "template not found for language '{}' and type '{}'",
            language,
            template_type
        )
    })?;

    // Render the template directory
    render_embedded_dir(template_dir, output_dir, &parser, &context)?;

    // Download WIT dependencies
    println!("📦 Fetching WIT dependencies...");
    pkg::fetch_wit_dependencies(output_dir, false).await?;

    Ok(())
}

/// Recursively render an embedded directory to the filesystem
///
/// This processes all files and subdirectories, applying liquid template
/// rendering to each file using the provided context.
fn render_embedded_dir(
    dir: &Dir,
    output_base: &Path,
    parser: &liquid::Parser,
    context: &liquid::Object,
) -> Result<()> {
    // Process all files in this directory
    for file in dir.files() {
        let file_name = file
            .path()
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("invalid file name"))?;

        let content = file.contents_utf8().ok_or_else(|| {
            anyhow::anyhow!("file is not valid UTF-8: '{}'", file.path().display())
        })?;

        // Render template
        let template = parser.parse(content).context(format!(
            "Failed to parse template: {}",
            file.path().display()
        ))?;

        let rendered = template.render(context).context(format!(
            "Failed to render template: {}",
            file.path().display()
        ))?;

        // Strip .template suffix from output filename if present
        let file_name_str = file_name
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("file name is not valid UTF-8"))?;
        let output_file_name = file_name_str
            .strip_suffix(".template")
            .unwrap_or(file_name_str);

        // Write output file
        let output_path = output_base.join(output_file_name);
        fs::write(&output_path, rendered)
            .context(format!("Failed to write file: {}", output_path.display()))?;
    }

    // Process all subdirectories recursively
    for subdir in dir.dirs() {
        let subdir_name = subdir
            .path()
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("invalid directory name"))?;

        let output_subdir = output_base.join(subdir_name);
        fs::create_dir_all(&output_subdir).context(format!(
            "Failed to create directory: {}",
            output_subdir.display()
        ))?;

        render_embedded_dir(subdir, &output_subdir, parser, context)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_name_conversion() {
        let name = "my-server";
        let package_name = name.replace('-', "_");
        assert_eq!(package_name, "my_server");
    }

    #[test]
    fn test_templates_embedded() {
        // Verify that tools templates are embedded
        assert!(TEMPLATES.get_dir("rust-tools").is_some());
        assert!(TEMPLATES.get_dir("python-tools").is_some());
        assert!(TEMPLATES.get_dir("typescript-tools").is_some());

        // Verify that resources templates are embedded
        assert!(TEMPLATES.get_dir("rust-resources").is_some());
        assert!(TEMPLATES.get_dir("python-resources").is_some());
        assert!(TEMPLATES.get_dir("typescript-resources").is_some());

        // Verify that routing-config template is embedded
        assert!(TEMPLATES.get_dir("rust-routing-config").is_some());
    }

    /// Test template path construction
    #[test]
    fn test_template_path_format() {
        let path1 = format!("{}-{}", Language::Rust, TemplateType::Tools);
        assert_eq!(path1, "rust-tools");

        let path2 = format!("{}-{}", Language::Python, TemplateType::Resources);
        assert_eq!(path2, "python-resources");

        let path3 = format!("{}-{}", Language::TypeScript, TemplateType::Prompts);
        assert_eq!(path3, "typescript-prompts");

        let path4 = format!("{}-{}", Language::Rust, TemplateType::RoutingConfig);
        assert_eq!(path4, "rust-routing-config");
    }

    /// Test liquid context creation
    #[test]
    fn test_template_context() {
        let context = liquid::object!({
            "project_name": "my-server",
            "package_name": "my_server",
            "wasmcp_version": "0.1.0",
            "language": "rust",
        });

        assert_eq!(context["project_name"], "my-server");
        assert_eq!(context["package_name"], "my_server");
        assert_eq!(context["wasmcp_version"], "0.1.0");
        assert_eq!(context["language"], "rust");
    }

    /// Test package name conversion edge cases
    #[test]
    fn test_package_name_edge_cases() {
        // Multiple hyphens
        assert_eq!("my-mcp-server".replace('-', "_"), "my_mcp_server");

        // No hyphens
        assert_eq!("myserver".replace('-', "_"), "myserver");

        // Leading/trailing hyphens
        assert_eq!("-server-".replace('-', "_"), "_server_");
    }

    /// Test parser creation
    #[test]
    fn test_liquid_parser_creation() {
        let parser = ParserBuilder::with_stdlib().build();
        assert!(parser.is_ok());
    }

    /// Test template rendering with context
    #[test]
    fn test_template_rendering() {
        let parser = ParserBuilder::with_stdlib().build().unwrap();
        let template = parser.parse("Hello {{name}}!").unwrap();

        let context = liquid::object!({
            "name": "World"
        });

        let rendered = template.render(&context).unwrap();
        assert_eq!(rendered, "Hello World!");
    }

    /// Test error message for missing template
    #[test]
    fn test_missing_template_error() {
        let error = format!(
            "template not found for language '{}' and type '{}'",
            "invalid-lang", "invalid-type"
        );
        assert!(error.contains("template not found"));
        assert!(error.contains("invalid-lang"));
        assert!(error.contains("invalid-type"));
    }

    /// Test WIT dependency message
    #[test]
    fn test_wit_dependency_message() {
        let msg = "📦 Fetching WIT dependencies...";
        assert!(msg.contains("Fetching WIT dependencies"));
    }

    /// Test all template types exist for Rust
    #[test]
    fn test_rust_templates_exist() {
        assert!(TEMPLATES.get_dir("rust-tools").is_some());
        assert!(TEMPLATES.get_dir("rust-resources").is_some());
        assert!(TEMPLATES.get_dir("rust-prompts").is_some());
        assert!(TEMPLATES.get_dir("rust-routing-config").is_some());
    }

    /// Test all template types exist for Python
    #[test]
    fn test_python_templates_exist() {
        assert!(TEMPLATES.get_dir("python-tools").is_some());
        assert!(TEMPLATES.get_dir("python-resources").is_some());
        assert!(TEMPLATES.get_dir("python-prompts").is_some());
    }

    /// Test all template types exist for TypeScript
    #[test]
    fn test_typescript_templates_exist() {
        assert!(TEMPLATES.get_dir("typescript-tools").is_some());
        assert!(TEMPLATES.get_dir("typescript-resources").is_some());
        assert!(TEMPLATES.get_dir("typescript-prompts").is_some());
    }
}
