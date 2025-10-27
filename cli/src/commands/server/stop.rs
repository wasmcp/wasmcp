use anyhow::{Context, Result};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use std::thread;
use std::time::Duration;

use super::manager;

/// Stop the background daemon
pub async fn stop() -> Result<()> {
    // Read PID file
    let pid = match manager::read_pid() {
        Ok(pid) => pid,
        Err(_) => {
            eprintln!("Server is not running (no PID file found)");
            return Ok(()); // Desired state achieved - server is stopped
        }
    };

    // Check if process is alive
    if !manager::is_process_alive(pid) {
        eprintln!("Server is not running (stale PID file detected)");
        // Clean up stale PID
        manager::remove_pid().context("Failed to remove stale PID file")?;
        return Ok(());
    }

    eprintln!("Stopping server (PID: {})...", pid);

    // Send SIGTERM for graceful shutdown
    let pid_nix = Pid::from_raw(pid as i32);
    kill(pid_nix, Signal::SIGTERM).context("Failed to send SIGTERM to process")?;

    // Wait for process to exit (max 10 seconds)
    let max_wait = Duration::from_secs(10);
    let poll_interval = Duration::from_millis(500);
    let start = std::time::Instant::now();

    while start.elapsed() < max_wait {
        if !manager::is_process_alive(pid) {
            eprintln!("Server stopped gracefully");
            manager::remove_pid().context("Failed to remove PID file")?;
            return Ok(());
        }
        thread::sleep(poll_interval);
    }

    // Process didn't respond to SIGTERM - force kill
    eprintln!("Server did not stop gracefully, forcing shutdown...");
    kill(pid_nix, Signal::SIGKILL).context("Failed to send SIGKILL to process")?;

    // Wait a bit more
    thread::sleep(Duration::from_millis(500));

    if !manager::is_process_alive(pid) {
        eprintln!("Server stopped (forced)");
        manager::remove_pid().context("Failed to remove PID file")?;
        Ok(())
    } else {
        anyhow::bail!("Failed to stop server process (PID: {})", pid)
    }
}
