//! Project scaffolding for MCP server handler components
//!
//! This module provides functionality to generate new handler component projects
//! from embedded templates. Templates are included at compile-time using include_dir.

use crate::{commands::pkg, Language, TemplateType};
use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};
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
    wasmcp_version: &str,
) -> Result<()> {
    // Create output directory
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

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

        // Write output file
        let output_path = output_base.join(file_name);
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
    }
}
