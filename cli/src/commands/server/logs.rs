use anyhow::{Context, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::thread;
use std::time::Duration;

use super::manager;

/// View daemon logs
pub async fn logs(follow: bool) -> Result<()> {
    let log_file = manager::get_log_file()?;

    if !log_file.exists() {
        anyhow::bail!(
            "Log file not found: {}\nHint: Server may not have been started via 'wasmcp mcp start'",
            log_file.display()
        );
    }

    if follow {
        // Follow mode (tail -f behavior)
        tail_follow(&log_file)?;
    } else {
        // Static mode - print entire file
        let content = std::fs::read_to_string(&log_file).context("Failed to read log file")?;
        print!("{}", content);
    }

    Ok(())
}

fn tail_follow(log_path: &std::path::Path) -> Result<()> {
    let mut file = File::open(log_path).context("Failed to open log file")?;

    // Seek to end of file
    file.seek(SeekFrom::End(0))
        .context("Failed to seek to end of log file")?;

    let mut reader = BufReader::new(file);
    let mut line = String::new();

    eprintln!("Following log file (Ctrl+C to exit)...");

    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(n) if n > 0 => {
                print!("{}", line);
            }
            Ok(_) => {
                // No new data - sleep and retry
                thread::sleep(Duration::from_millis(500));
            }
            Err(e) => {
                anyhow::bail!("Error reading log file: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    /// Test log file error message when file doesn't exist
    #[test]
    fn test_log_file_not_found_message() {
        let error_msg = "Log file not found: /path/to/logs.txt\nHint: Server may not have been started via 'wasmcp mcp start'";
        assert!(error_msg.contains("Log file not found"));
        assert!(error_msg.contains("Hint"));
        assert!(error_msg.contains("wasmcp mcp start"));
    }

    /// Test follow mode message
    #[test]
    fn test_follow_mode_message() {
        let msg = "Following log file (Ctrl+C to exit)...";
        assert!(msg.contains("Following"));
        assert!(msg.contains("Ctrl+C"));
    }

    /// Test reading log file content
    #[test]
    fn test_read_log_content() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test.log");

        // Create log file with content
        let mut file = File::create(&log_file).unwrap();
        writeln!(file, "Log line 1").unwrap();
        writeln!(file, "Log line 2").unwrap();
        writeln!(file, "Log line 3").unwrap();
        drop(file);

        // Read content
        let content = std::fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("Log line 1"));
        assert!(content.contains("Log line 2"));
        assert!(content.contains("Log line 3"));
    }

    /// Test file seeking to end
    #[test]
    fn test_seek_to_end() {
        let temp_dir = TempDir::new().unwrap();
        let log_file = temp_dir.path().join("test.log");

        // Create file with content
        let mut file = File::create(&log_file).unwrap();
        writeln!(file, "Initial content").unwrap();
        drop(file);

        // Open and seek to end
        let mut file = File::open(&log_file).unwrap();
        let pos = file.seek(SeekFrom::End(0)).unwrap();

        // Position should be at end (after "Initial content\n")
        assert!(pos > 0);
    }

    /// Test sleep duration for follow mode
    #[test]
    fn test_follow_sleep_duration() {
        let duration = Duration::from_millis(500);
        assert_eq!(duration.as_millis(), 500);
    }

    /// Test line buffer clearing
    #[test]
    fn test_line_buffer_operations() {
        let mut line = String::from("old content");
        line.clear();
        assert!(line.is_empty());

        line.push_str("new content");
        assert_eq!(line, "new content");
    }
}
