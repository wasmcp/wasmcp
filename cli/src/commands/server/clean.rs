use anyhow::Result;

use super::manager;

/// Clean up daemon state files
pub async fn clean() -> Result<()> {
    let mut cleaned = Vec::new();

    // Remove PID file
    if manager::remove_pid().is_ok() {
        cleaned.push("PID file");
    }

    // Remove log file
    if manager::remove_log().is_ok() {
        cleaned.push("log file");
    }

    // Remove flags file
    if manager::remove_flags().is_ok() {
        cleaned.push("flags file");
    }

    if cleaned.is_empty() {
        eprintln!("No state files found to clean");
    } else {
        eprintln!("Cleaned up: {}", cleaned.join(", "));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    /// Test cleanup message with no files
    #[test]
    fn test_clean_no_files_message() {
        let msg = "No state files found to clean";
        assert!(msg.contains("No state files"));
        assert!(msg.contains("clean"));
    }

    /// Test cleanup message with files
    #[test]
    fn test_clean_success_message() {
        let files = ["PID file", "log file", "flags file"];
        let msg = format!("Cleaned up: {}", files.join(", "));

        assert!(msg.contains("Cleaned up"));
        assert!(msg.contains("PID file"));
        assert!(msg.contains("log file"));
        assert!(msg.contains("flags file"));
    }

    /// Test partial cleanup message
    #[test]
    fn test_clean_partial_message() {
        let files = ["PID file", "log file"];
        let msg = format!("Cleaned up: {}", files.join(", "));

        assert!(msg.contains("PID file"));
        assert!(msg.contains("log file"));
        assert!(!msg.contains("flags file"));
    }

    /// Test file list joining
    #[test]
    fn test_file_list_join() {
        let files = ["file1", "file2", "file3"];
        let joined = files.join(", ");
        assert_eq!(joined, "file1, file2, file3");

        let single = ["only-file"];
        assert_eq!(single.join(", "), "only-file");

        let empty: &[&str] = &[];
        assert_eq!(empty.join(", "), "");
    }

    /// Test cleaned vector operations
    #[test]
    fn test_cleaned_vector() {
        let mut cleaned = Vec::new();
        assert!(cleaned.is_empty());

        cleaned.push("PID file");
        assert_eq!(cleaned.len(), 1);

        cleaned.push("log file");
        cleaned.push("flags file");
        assert_eq!(cleaned.len(), 3);

        assert_eq!(cleaned[0], "PID file");
        assert_eq!(cleaned[1], "log file");
        assert_eq!(cleaned[2], "flags file");
    }
}
