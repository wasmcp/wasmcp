use anyhow::{Context, Result};
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging for the wasmcp CLI
///
/// Logs are written to:
/// - XDG_DATA_HOME/wasmcp/logs/ on Unix (typically ~/.local/share/wasmcp/logs/)
/// - ~/Library/Application Support/wasmcp/logs/ on macOS
/// - {FOLDERID_LocalAppData}/wasmcp/logs/ on Windows
///
/// Log files are rotated daily with the pattern: wasmcp-YYYY-MM-DD.log
///
/// The log level can be controlled via the RUST_LOG environment variable:
/// - RUST_LOG=debug wasmcp mcp serve  (verbose logging)
/// - RUST_LOG=info wasmcp mcp serve   (default level)
/// - RUST_LOG=error wasmcp mcp serve  (errors only)
pub fn init() -> Result<()> {
    let log_dir = get_log_dir()?;

    // Ensure log directory exists
    std::fs::create_dir_all(&log_dir)
        .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;

    // Create file appender with daily rotation
    let file_appender = tracing_appender::rolling::daily(&log_dir, "wasmcp.log");

    // Configure filter from environment or default to info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("wasmcp=info,rmcp=info"));

    // Create subscriber with both file and stdout
    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_appender)
                .with_ansi(false) // No ANSI colors in log files
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true) // ANSI colors for terminal
                .with_target(false)
                .compact(),
        )
        .try_init()
        .context("Failed to initialize tracing subscriber")?;

    tracing::info!("Logging initialized to {}", log_dir.display());

    Ok(())
}

/// Get the log directory path using XDG conventions
fn get_log_dir() -> Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .context("Failed to determine data directory (XDG_DATA_HOME or platform equivalent)")?;

    Ok(data_dir.join("wasmcp").join("logs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_log_dir() {
        let log_dir = get_log_dir().expect("Should get log dir");
        assert!(log_dir.ends_with("wasmcp/logs"));
    }
}
