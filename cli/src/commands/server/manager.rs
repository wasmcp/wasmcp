//! MCP server state management
//!
//! Handles PID files, log files, and saved flags for daemon mode.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Saved server configuration flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFlags {
    pub port: u16,
    pub local_resources: Option<PathBuf>,
    pub verbose: bool,
}

impl Default for ServerFlags {
    fn default() -> Self {
        Self {
            port: 8085,
            local_resources: None,
            verbose: false,
        }
    }
}

/// Get the PID file path
pub fn get_pid_file() -> Result<PathBuf> {
    let state_dir = crate::state::ensure_state_dir()?;
    Ok(state_dir.join("mcp-server.pid"))
}

/// Get the log file path
pub fn get_log_file() -> Result<PathBuf> {
    let state_dir = crate::state::ensure_state_dir()?;
    Ok(state_dir.join("mcp-server.log"))
}

/// Get the flags file path
pub fn get_flags_file() -> Result<PathBuf> {
    let state_dir = crate::state::ensure_state_dir()?;
    Ok(state_dir.join("mcp-server.flags"))
}

/// Read PID from file
pub fn read_pid() -> Result<u32> {
    let pid_file = get_pid_file()?;

    let pid_str = fs::read_to_string(&pid_file)
        .with_context(|| format!("Failed to read PID file: {}", pid_file.display()))?;

    let pid: u32 = pid_str.trim().parse().context("Invalid PID in file")?;

    Ok(pid)
}

/// Write PID to file
#[allow(dead_code)] // Used by daemon.rs, but clippy doesn't track cross-module usage
pub fn write_pid(pid: u32) -> Result<()> {
    let pid_file = get_pid_file()?;
    fs::write(&pid_file, pid.to_string())
        .with_context(|| format!("Failed to write PID file: {}", pid_file.display()))?;
    Ok(())
}

/// Remove PID file
pub fn remove_pid() -> Result<()> {
    let pid_file = get_pid_file()?;
    if pid_file.exists() {
        fs::remove_file(&pid_file)
            .with_context(|| format!("Failed to remove PID file: {}", pid_file.display()))?;
    }
    Ok(())
}

/// Check if a process is alive (Unix only)
#[cfg(unix)]
pub fn is_process_alive(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    // Signal 0 (None) checks if process exists without sending a signal
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(unix))]
pub fn is_process_alive(_pid: u32) -> bool {
    // Windows support can be added later using sysinfo or WinAPI
    unimplemented!("Process checking not implemented on this platform")
}

/// Read saved flags from file
pub fn read_flags() -> Result<ServerFlags> {
    let flags_file = get_flags_file()?;

    let json = fs::read_to_string(&flags_file)
        .with_context(|| format!("Failed to read flags file: {}", flags_file.display()))?;

    let flags: ServerFlags = serde_json::from_str(&json).context("Failed to parse flags JSON")?;

    Ok(flags)
}

/// Write flags to file
pub fn write_flags(flags: &ServerFlags) -> Result<()> {
    let flags_file = get_flags_file()?;
    let json = serde_json::to_string_pretty(flags).context("Failed to serialize flags")?;

    fs::write(&flags_file, json)
        .with_context(|| format!("Failed to write flags file: {}", flags_file.display()))?;
    Ok(())
}

/// Remove flags file
pub fn remove_flags() -> Result<()> {
    let flags_file = get_flags_file()?;
    if flags_file.exists() {
        fs::remove_file(&flags_file)
            .with_context(|| format!("Failed to remove flags file: {}", flags_file.display()))?;
    }
    Ok(())
}

/// Remove log file
pub fn remove_log() -> Result<()> {
    let log_file = get_log_file()?;
    if log_file.exists() {
        fs::remove_file(&log_file)
            .with_context(|| format!("Failed to remove log file: {}", log_file.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flags_serialization() {
        let flags = ServerFlags {
            port: 9000,
            local_resources: Some(PathBuf::from("/tmp/test")),
            verbose: true,
        };

        let json = serde_json::to_string(&flags).unwrap();
        let deserialized: ServerFlags = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.port, 9000);
        assert_eq!(
            deserialized.local_resources,
            Some(PathBuf::from("/tmp/test"))
        );
        assert!(deserialized.verbose);
    }

    #[test]
    fn test_flags_default_values() {
        let flags = ServerFlags::default();
        assert_eq!(flags.port, 8085, "default port should be 8085");
        assert_eq!(
            flags.local_resources, None,
            "default local_resources should be None"
        );
        assert!(!flags.verbose, "default verbose should be false");
    }

    #[test]
    fn test_get_pid_file_path() {
        let pid_file = get_pid_file().expect("should get PID file path");
        assert!(
            pid_file.to_string_lossy().contains("mcp-server.pid"),
            "PID file path should contain mcp-server.pid"
        );
    }

    #[test]
    fn test_get_log_file_path() {
        let log_file = get_log_file().expect("should get log file path");
        assert!(
            log_file.to_string_lossy().contains("mcp-server.log"),
            "log file path should contain mcp-server.log"
        );
    }

    #[test]
    fn test_get_flags_file_path() {
        let flags_file = get_flags_file().expect("should get flags file path");
        assert!(
            flags_file.to_string_lossy().contains("mcp-server.flags"),
            "flags file path should contain mcp-server.flags"
        );
    }

    #[test]
    fn test_state_files_in_same_directory() {
        let pid_file = get_pid_file().expect("should get PID file");
        let log_file = get_log_file().expect("should get log file");
        let flags_file = get_flags_file().expect("should get flags file");

        let pid_dir = pid_file.parent().expect("PID file should have parent");
        let log_dir = log_file.parent().expect("log file should have parent");
        let flags_dir = flags_file.parent().expect("flags file should have parent");

        assert_eq!(
            pid_dir, log_dir,
            "PID and log files should be in same directory"
        );
        assert_eq!(
            log_dir, flags_dir,
            "log and flags files should be in same directory"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_is_process_alive_with_current_process() {
        // Current process should always be alive
        let current_pid = std::process::id();
        assert!(
            is_process_alive(current_pid),
            "current process should be detected as alive"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_is_process_alive_with_nonexistent_pid() {
        // Very high PID unlikely to exist
        assert!(
            !is_process_alive(999999),
            "nonexistent PID should not be detected as alive"
        );
    }
}
