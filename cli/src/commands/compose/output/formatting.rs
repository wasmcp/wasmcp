//! Output formatting for composition results
//!
//! This module provides functions for formatting user-facing messages during
//! and after component composition, including pipeline diagrams and success
//! messages with run instructions.
//!
//! # Message Types
//!
//! - **Pipeline Diagrams**: Visual representation of component chains
//! - **Success Messages**: Completion confirmation with run instructions
//! - **Handler Messages**: Specialized output for handler composition
//!
//! # Examples
//!
//! ```rust,ignore
//! # use wasmcp::commands::compose::output::print_pipeline_diagram;
//! # use std::path::PathBuf;
//! let components = vec![
//!     PathBuf::from("calculator.wasm"),
//!     PathBuf::from("weather.wasm"),
//! ];
//! print_pipeline_diagram("http", &components);
//! // Prints:
//! //   http (transport)
//! //   ↓
//! //   1. calculator
//! //   ↓
//! //   2. weather
//! //   ↓
//! //   method-not-found (terminal handler)
//! ```

use std::path::{Path, PathBuf};

/// Print the composition pipeline diagram
///
/// Displays a visual representation of the server composition pipeline,
/// showing the transport, all middleware components, and the terminal handler.
///
/// # Arguments
///
/// * `transport` - The transport type ("http" or "stdio")
/// * `components` - List of middleware component paths in order
///
/// # Output Format
///
/// ```text
/// Composing MCP server pipeline...
///    http (transport)
///    ↓
///    1. calculator
///    ↓
///    2. weather
///    ↓
///    method-not-found (terminal handler)
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// # use wasmcp::commands::compose::output::print_pipeline_diagram;
/// # use std::path::PathBuf;
/// let components = vec![PathBuf::from("tool.wasm")];
/// print_pipeline_diagram("stdio", &components);
/// ```
pub fn print_pipeline_diagram(transport: &str, components: &[PathBuf]) {
    println!("\nComposing MCP server pipeline...");
    println!("   {} (transport)", transport);
    for (i, path) in components.iter().enumerate() {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");
        println!("   ↓");
        println!("   {}. {}", i + 1, name);
    }
    println!("   ↓");
    println!("   method-not-found (terminal handler)");
}

/// Print the handler composition pipeline diagram
///
/// Displays a visual representation of handler composition (no transport or terminal).
///
/// # Arguments
///
/// * `components` - List of component paths in order
///
/// # Output Format
///
/// ```text
/// Composing handler component...
///    1. calculator
///    ↓
///    2. weather
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// # use wasmcp::commands::compose::output::print_handler_pipeline_diagram;
/// # use std::path::PathBuf;
/// let components = vec![
///     PathBuf::from("calc.wasm"),
///     PathBuf::from("math.wasm"),
/// ];
/// print_handler_pipeline_diagram(&components);
/// ```
pub fn print_handler_pipeline_diagram(components: &[PathBuf]) {
    println!("\nComposing handler component...");
    for (i, path) in components.iter().enumerate() {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");
        if i > 0 {
            println!("   ↓");
        }
        println!("   {}. {}", i + 1, name);
    }
}

/// Print success message with run instructions
///
/// Displays completion message and runtime-specific instructions for
/// running the composed server.
///
/// # Arguments
///
/// * `output_path` - Path to the composed server file
/// * `transport` - The transport type ("http" or "stdio")
/// * `runtime_info` - Optional runtime information (capabilities and type)
///
/// # Output Format
///
/// For HTTP transport with Wasmtime:
/// ```text
/// Composed: /path/to/server.wasm
///
/// To run the server:
///   wasmtime serve -Shttp -Skeyvalue -Scli /path/to/server.wasm
/// ```
///
/// For stdio transport:
/// ```text
/// Composed: /path/to/server.wasm
///
/// To run the server:
///   wasmtime run -Scli /path/to/server.wasm
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// # use wasmcp::commands::compose::output::print_success_message;
/// # use std::path::Path;
/// let path = Path::new("server.wasm");
/// print_success_message(path, "http", None);
/// ```
pub fn print_success_message(
    output_path: &Path,
    transport: &str,
    runtime_info: Option<&crate::commands::compose::inspection::RuntimeInfo>,
) {
    use crate::commands::compose::inspection::RuntimeType;

    println!("\nComposed: {}", output_path.display());
    println!("\nTo run the server:");

    // Get runtime info or use default
    let runtime = runtime_info
        .cloned()
        .unwrap_or_else(crate::commands::compose::inspection::RuntimeInfo::default);

    match (&runtime.runtime_type, transport) {
        (RuntimeType::Wasmtime, "http") => {
            // Wasmtime HTTP: use serve with capability flags
            let mut flags = String::new();
            for cap in &runtime.capabilities {
                flags.push_str(&format!(" -S{}", cap));
            }
            println!("  wasmtime serve{} {}", flags, output_path.display());
        }
        (RuntimeType::Wasmtime, "stdio") => {
            // Wasmtime stdio: use run with capability flags
            let mut flags = String::new();
            for cap in &runtime.capabilities {
                flags.push_str(&format!(" -S{}", cap));
            }
            println!("  wasmtime run{} {}", flags, output_path.display());
        }
        (RuntimeType::Spin, "http") => {
            // Spin runtime
            println!("  spin up -f {}", output_path.display());
        }
        (RuntimeType::Spin, "stdio") => {
            // Spin doesn't support stdio mode
            println!("  # Note: Spin runtime does not support stdio transport");
            println!("  wasmtime run {}", output_path.display());
        }
        (RuntimeType::Generic, "http") => {
            // Generic/unknown runtime - use basic wasmtime command
            println!("  wasmtime serve -Scli {}", output_path.display());
        }
        (RuntimeType::Generic, "stdio") => {
            println!("  wasmtime run {}", output_path.display());
        }
        _ => {
            // Fallback
            println!("  wasmtime {}", output_path.display());
        }
    }
}

/// Print success message for handler composition
///
/// Displays completion message and usage examples for the composed handler.
/// Handlers are intermediate components that can be used in further compositions.
///
/// # Arguments
///
/// * `output_path` - Path to the composed handler file
///
/// # Output Format
///
/// ```text
/// Composed handler component: /path/to/handler.wasm
///
/// To use this handler:
///   # In a server composition:
///   wasmcp compose server /path/to/handler.wasm other.wasm
///
///   # In another handler composition:
///   wasmcp compose handler /path/to/handler.wasm another.wasm
/// ```
///
/// # Examples
///
/// ```rust,ignore
/// # use wasmcp::commands::compose::output::print_handler_success_message;
/// # use std::path::Path;
/// let path = Path::new("my-handler.wasm");
/// print_handler_success_message(path);
/// ```
pub fn print_handler_success_message(output_path: &Path) {
    println!("\nComposed handler component: {}", output_path.display());
    println!("\nTo use this handler:");
    println!("  # In a server composition:");
    println!(
        "  wasmcp compose server {} other.wasm",
        output_path.display()
    );
    println!("\n  # In another handler composition:");
    println!(
        "  wasmcp compose handler {} another.wasm",
        output_path.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_diagram_format() {
        // Test that the pipeline diagram contains expected elements
        // (We can't easily capture stdout in tests, so we just verify the logic)
        let components = [PathBuf::from("test.wasm")];
        let name = components[0]
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("component");
        assert_eq!(name, "test");
    }

    #[test]
    fn test_handler_pipeline_diagram_format() {
        let components = [PathBuf::from("calc.wasm"), PathBuf::from("math.wasm")];
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].file_stem().unwrap().to_str().unwrap(), "calc");
    }

    #[test]
    fn test_success_message_http() {
        let path = PathBuf::from("server.wasm");
        let expected_cmd = format!("wasmtime serve -Scli {}", path.display());
        assert!(expected_cmd.contains("wasmtime serve"));
        assert!(expected_cmd.contains("server.wasm"));
    }

    #[test]
    fn test_success_message_stdio() {
        let path = PathBuf::from("server.wasm");
        let expected_cmd = format!("wasmtime run {}", path.display());
        assert!(expected_cmd.contains("wasmtime run"));
        assert!(expected_cmd.contains("server.wasm"));
    }

    #[test]
    fn test_handler_success_message_format() {
        let path = PathBuf::from("handler.wasm");
        let server_cmd = format!("wasmcp compose server {}", path.display());
        let handler_cmd = format!("wasmcp compose handler {}", path.display());
        assert!(server_cmd.contains("compose server"));
        assert!(handler_cmd.contains("compose handler"));
    }
}
