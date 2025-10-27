use anyhow::{Context, Result};
use std::path::PathBuf;

use super::manager::{self, ServerFlags};
use crate::state;

/// Start the MCP server as a background daemon
pub async fn start(
    port: Option<u16>,
    verbose: bool,
    local_resources: Option<PathBuf>,
) -> Result<()> {
    // Check if server is already running
    if let Ok(pid) = manager::read_pid() {
        if manager::is_process_alive(pid) {
            anyhow::bail!(
                "Server already running (PID: {}). Use 'wasmcp mcp stop' or 'wasmcp mcp restart'.",
                pid
            );
        } else {
            // Stale PID file - clean it up
            let _ = manager::remove_pid();
        }
    }

    // Determine actual port (default 8085 if None)
    let actual_port = port.unwrap_or(8085);

    // Create flags to save
    let flags = ServerFlags {
        port: actual_port,
        local_resources: local_resources.clone(),
        verbose,
    };

    // Save flags for restart
    manager::write_flags(&flags).context("Failed to save server flags")?;

    // Ensure state directory exists
    state::ensure_state_dir().context("Failed to create state directory")?;

    // Get file paths
    let pid_file = manager::get_pid_file()?;
    let log_file = manager::get_log_file()?;

    eprintln!("Starting wasmcp MCP server as daemon...");
    eprintln!("  Port: {}", actual_port);
    eprintln!("  PID file: {}", pid_file.display());
    eprintln!("  Log file: {}", log_file.display());

    // macOS has issues with fork() after tokio runtime initialization
    // Use spawn approach instead of daemonize crate on macOS
    #[cfg(target_os = "macos")]
    {
        spawn_daemon(actual_port, verbose, local_resources)?;
    }

    // On Linux, use traditional daemonize which is more robust
    #[cfg(not(target_os = "macos"))]
    {
        use daemonize::Daemonize;

        let daemonize = Daemonize::new()
            .pid_file(pid_file)
            .working_directory(std::env::current_dir()?)
            .stdout(std::fs::File::create(&log_file)?)
            .stderr(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_file)?,
            );

        daemonize.start().context("Failed to daemonize process")?;

        // Now we're in the daemon process - start the server
        super::serve::start_server(Some(actual_port), verbose, local_resources).await?;
    }

    Ok(())
}

/// Spawn a daemon process (macOS-safe alternative to fork)
///
/// This avoids the macOS fork() issues with Objective-C runtime by spawning
/// a completely new process instead of forking.
#[cfg(target_os = "macos")]
fn spawn_daemon(port: u16, verbose: bool, local_resources: Option<PathBuf>) -> Result<()> {
    use std::process::{Command, Stdio};

    // Build arguments for the daemon process
    let mut args = vec![
        "__internal_daemon__".to_string(),
        port.to_string(),
        verbose.to_string(),
    ];

    if let Some(ref path) = local_resources {
        args.push(path.to_string_lossy().to_string());
    }

    let log_file = manager::get_log_file()?;

    // Spawn a new process that will run the server
    let child = Command::new(std::env::current_exe()?)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(std::fs::File::create(&log_file)?))
        .stderr(Stdio::from(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)?,
        ))
        .spawn()
        .context("Failed to spawn daemon process")?;

    // Write PID file
    manager::write_pid(child.id())?;

    eprintln!("✓ Server started (PID: {})", child.id());

    Ok(())
}

/// Internal daemon entry point (called by spawned process on macOS)
///
/// This function is called when the binary is invoked with the __internal_daemon__ argument.
/// It runs the server in the background.
pub async fn daemon_entry(
    port: u16,
    verbose: bool,
    local_resources: Option<PathBuf>,
) -> Result<()> {
    // We're now in a fresh process - safe to initialize tokio and start server
    super::serve::start_server(Some(port), verbose, local_resources).await
}
