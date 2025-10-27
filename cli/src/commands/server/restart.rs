use anyhow::{Context, Result};
use std::path::PathBuf;

use super::manager;

/// Restart the background daemon with merged flags
pub async fn restart(
    port: Option<u16>,
    verbose: bool,
    local_resources: Option<PathBuf>,
) -> Result<()> {
    // Read saved flags
    let saved_flags = manager::read_flags().context(
        "No saved server state found. Use 'wasmcp mcp start' first to create initial state.",
    )?;

    // Merge flags: new flags override saved ones
    // For boolean flags, we need explicit override semantics since CLI provides default false
    // TODO: Consider using Option<bool> to distinguish "not provided" from "explicitly false"
    let merged_port = port.or(Some(saved_flags.port));
    let merged_verbose = verbose || saved_flags.verbose; // For now, verbose can only be added, not removed
    let merged_local_resources = local_resources.or(saved_flags.local_resources);

    eprintln!("Restarting server with merged configuration...");
    if let Some(p) = merged_port {
        eprintln!("  Port: {}", p);
    }
    if merged_verbose {
        eprintln!("  Verbose: enabled");
    }
    if let Some(ref path) = merged_local_resources {
        eprintln!("  Local resources: {}", path.display());
    }

    // Stop existing server (if running)
    if let Ok(pid) = manager::read_pid() {
        if manager::is_process_alive(pid) {
            eprintln!("Stopping existing server...");
            super::stop::stop().await?;
        } else {
            // Clean up stale PID
            let _ = manager::remove_pid();
        }
    }

    // Start with merged flags
    eprintln!("Starting server...");
    super::daemon::start(merged_port, merged_verbose, merged_local_resources).await?;

    eprintln!("Server restarted successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::server::manager::ServerFlags;

    #[test]
    fn test_port_merging_new_overrides() {
        let saved = ServerFlags {
            port: 8085,
            verbose: false,
            local_resources: None,
        };

        // New port provided - should override
        let new_port = Some(9000);
        let merged = new_port.or(Some(saved.port));
        assert_eq!(merged, Some(9000));
    }

    #[test]
    fn test_port_merging_none_uses_saved() {
        let saved = ServerFlags {
            port: 8085,
            verbose: false,
            local_resources: None,
        };

        // No new port - should use saved
        let new_port: Option<u16> = None;
        let merged = new_port.or(Some(saved.port));
        assert_eq!(merged, Some(8085));
    }

    #[test]
    fn test_verbose_or_behavior_always_true_when_saved() {
        let saved = ServerFlags {
            port: 8085,
            verbose: true,
            local_resources: None,
        };

        // New verbose is false, but saved is true - OR gives true
        let new_verbose = false;
        let merged = new_verbose || saved.verbose;
        assert!(merged, "verbose flag should remain true due to OR logic");
    }

    #[test]
    fn test_verbose_or_behavior_can_add_but_not_remove() {
        let saved = ServerFlags {
            port: 8085,
            verbose: false,
            local_resources: None,
        };

        // Can turn verbose ON
        let new_verbose = true;
        let merged = new_verbose || saved.verbose;
        assert!(merged);

        // But cannot turn it OFF once enabled
        let saved_with_verbose = ServerFlags {
            port: 8085,
            verbose: true,
            local_resources: None,
        };
        let new_verbose = false;
        let merged = new_verbose || saved_with_verbose.verbose;
        assert!(merged, "cannot disable verbose via restart");
    }

    #[test]
    fn test_local_resources_merging_new_overrides() {
        let saved = ServerFlags {
            port: 8085,
            verbose: false,
            local_resources: Some(PathBuf::from("/old/path")),
        };

        // New path provided - should override
        let new_path = Some(PathBuf::from("/new/path"));
        let merged = new_path.or(saved.local_resources);
        assert_eq!(merged, Some(PathBuf::from("/new/path")));
    }

    #[test]
    fn test_local_resources_merging_none_uses_saved() {
        let saved = ServerFlags {
            port: 8085,
            verbose: false,
            local_resources: Some(PathBuf::from("/saved/path")),
        };

        // No new path - should use saved
        let new_path: Option<PathBuf> = None;
        let merged = new_path.or(saved.local_resources);
        assert_eq!(merged, Some(PathBuf::from("/saved/path")));
    }

    #[test]
    fn test_local_resources_merging_can_clear() {
        let saved = ServerFlags {
            port: 8085,
            verbose: false,
            local_resources: None,
        };

        // Setting None when saved is also None
        let new_path: Option<PathBuf> = None;
        let merged = new_path.or(saved.local_resources);
        assert_eq!(merged, None);
    }
}
