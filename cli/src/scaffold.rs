use crate::{HandlerType, Language};
use anyhow::{Context, Result};
use include_dir::{include_dir, Dir};
use liquid::ParserBuilder;
use std::fs;
use std::path::Path;

// Embed templates at compile time
static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

pub fn create_project(
    output_dir: &Path,
    name: &str,
    handler_type: HandlerType,
    language: Language,
    wasmcp_version: &str,
) -> Result<()> {
    // Create output directory
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    // Create template context
    let package_name = name.replace('-', "_");
    let world_name = if handler_type == HandlerType::Middleware {
        "middleware".to_string()
    } else {
        format!("{}-handler", handler_type)
    };
    let interface_name = handler_type.interface_name();
    let writer_interface = format!("write-{}-result", interface_name);
    let writer_component = format!("{}-writer", interface_name);

    let mut context = liquid::object!({
        "project_name": name,
        "package_name": package_name,
        "handler_type": handler_type.to_string(),
        "handler_type_capitalized": capitalize_first(&handler_type.to_string()),
        "world_name": world_name,
        "writer_interface": writer_interface,
        "writer_component": writer_component,
        "wasmcp_version": wasmcp_version,
        "language": language.to_string(),
    });

    // Add language-specific context
    match language {
        Language::Go => {
            context.insert(
                "needs_wasi_cli".into(),
                liquid::model::Value::Scalar(true.into()),
            );
            context.insert(
                "generated_version_path".into(),
                liquid::model::Value::Scalar("v0.3.0".into()),
            );
        }
        _ => {
            context.insert(
                "needs_wasi_cli".into(),
                liquid::model::Value::Scalar(false.into()),
            );
        }
    }

    // Create liquid parser
    let parser = ParserBuilder::with_stdlib().build()?;

    // Get template directory path
    let template_path = get_template_path(language, handler_type);

    // Get the embedded directory
    let template_dir = TEMPLATES
        .get_dir(&template_path)
        .ok_or_else(|| anyhow::anyhow!("Template not found: {}", template_path))?;

    // Render the template directory
    render_embedded_dir(template_dir, output_dir, &parser, &context)?;

    Ok(())
}

fn get_template_path(language: Language, handler_type: HandlerType) -> String {
    format!("{}/{}", language, handler_type)
}

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
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;
        let content = file
            .contents_utf8()
            .ok_or_else(|| anyhow::anyhow!("File is not valid UTF-8: {:?}", file.path()))?;

        // Render template
        let rendered = parser
            .parse(content)
            .context(format!("Failed to parse template: {:?}", file.path()))?
            .render(context)
            .context(format!("Failed to render template: {:?}", file.path()))?;

        // Write output file
        let output_path = output_base.join(file_name);
        fs::write(&output_path, rendered)
            .context(format!("Failed to write file: {}", output_path.display()))?;
    }

    // Process all subdirectories
    for subdir in dir.dirs() {
        let subdir_name = subdir
            .path()
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid directory name"))?;
        let output_subdir = output_base.join(subdir_name);
        fs::create_dir_all(&output_subdir)?;
        render_embedded_dir(subdir, &output_subdir, parser, context)?;
    }

    Ok(())
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("tools"), "Tools");
        assert_eq!(capitalize_first("resources"), "Resources");
        assert_eq!(capitalize_first(""), "");
    }
}
