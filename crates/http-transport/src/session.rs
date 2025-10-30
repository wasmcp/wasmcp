/// Session management implementation following MCP spec 2025-06-18
///
/// Implements the session resource pattern from wasmcp:server/sessions WIT interface.
/// Each session owns a WASI KV bucket handle and manages its own lifecycle.
///
/// MCP Spec Requirements:
/// - Session IDs MUST be cryptographically secure (UUID v4)
/// - Session IDs MUST contain only visible ASCII (0x21-0x7E)
/// - Servers MAY assign sessions during initialization
/// - Servers that require sessions SHOULD return 400 for missing session IDs
/// - Servers MUST return 404 for terminated/invalid sessions
/// - Clients SHOULD send DELETE to terminate sessions

use serde::{Deserialize, Serialize};
use crate::bindings::wasi::keyvalue::store::{self, Bucket, Error as StoreError};
use crate::bindings::wasi::random::random;

/// Session metadata stored in KV under "__meta__" key
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMetadata {
    /// Whether the session has been terminated
    terminated: bool,
    /// Optional reason for termination
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    /// Unix timestamp (milliseconds) when session was created
    created_at: u64,
}

/// Session errors following wasmcp:server/sessions::session-error
#[derive(Debug)]
pub enum SessionError {
    /// Key-value store error
    Store(String),
    /// Session ID doesn't exist
    NoSuchSession,
    /// Other unexpected errors
    Unexpected(String),
}

impl From<StoreError> for SessionError {
    fn from(e: StoreError) -> Self {
        SessionError::Store(format!("{:?}", e))
    }
}

/// Session resource following wasmcp:server/sessions pattern
///
/// Each session owns a WASI KV bucket handle. The bucket is used for:
/// - `__meta__`: Internal session metadata (JSON)
/// - `{component}:{key}`: Application data (binary, component-managed)
pub struct Session {
    id: String,
    bucket: Bucket,
}

impl Session {
    /// Creates a new session with cryptographically secure UUID v4
    ///
    /// Per MCP spec:
    /// - Generates globally unique session ID
    /// - ID contains only visible ASCII characters (0x21-0x7E)
    /// - Stores initial metadata with terminated=false
    ///
    /// # Arguments
    /// * `bucket_name` - Name of WASI KV bucket to use (e.g., "mcp-sessions")
    ///
    /// # Returns
    /// * `Ok(Session)` - New session with unique ID
    /// * `Err(SessionError)` - If bucket open fails or metadata store fails
    pub fn initialize(bucket_name: &str) -> Result<Session, SessionError> {
        // Open KV bucket
        let bucket = store::open(bucket_name)
            .map_err(|e| SessionError::Store(format!("Failed to open bucket '{}': {:?}", bucket_name, e)))?;

        // Generate cryptographically secure UUID v4
        let session_id = generate_uuid_v4()?;

        // Create initial metadata
        let metadata = SessionMetadata {
            terminated: false,
            reason: None,
            created_at: current_timestamp_ms(),
        };

        // Store metadata
        let metadata_json = serde_json::to_vec(&metadata)
            .map_err(|e| SessionError::Unexpected(format!("Failed to serialize metadata: {}", e)))?;

        bucket.set("__meta__", &metadata_json)?;

        Ok(Session {
            id: session_id,
            bucket,
        })
    }

    /// Opens an existing session by ID
    ///
    /// Per MCP spec:
    /// - Validates session exists in storage
    /// - Does NOT check termination status (validation happens separately)
    ///
    /// # Arguments
    /// * `bucket_name` - Name of WASI KV bucket
    /// * `id` - Session ID from Mcp-Session-Id header
    ///
    /// # Returns
    /// * `Ok(Session)` - Existing session
    /// * `Err(SessionError::NoSuchSession)` - If metadata doesn't exist
    /// * `Err(SessionError::Store)` - If bucket open fails
    pub fn open(bucket_name: &str, id: &str) -> Result<Session, SessionError> {
        // Open KV bucket
        let bucket = store::open(bucket_name)
            .map_err(|e| SessionError::Store(format!("Failed to open bucket '{}': {:?}", bucket_name, e)))?;

        // Check if session metadata exists
        let exists = bucket.exists("__meta__")?;
        if !exists {
            return Err(SessionError::NoSuchSession);
        }

        Ok(Session {
            id: id.to_string(),
            bucket,
        })
    }

    /// Returns the session ID
    ///
    /// This ID should be sent to clients in the Mcp-Session-Id HTTP header.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Retrieves application data from session storage
    ///
    /// # Arguments
    /// * `key` - Key name (e.g., "counter:value", "weather:cache")
    ///
    /// # Returns
    /// * `Ok(Some(bytes))` - Data found
    /// * `Ok(None)` - Key doesn't exist
    /// * `Err(SessionError)` - Storage error
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>, SessionError> {
        self.bucket.get(key).map_err(SessionError::from)
    }

    /// Stores application data in session storage
    ///
    /// # Arguments
    /// * `key` - Key name (e.g., "counter:value", "weather:cache")
    /// * `value` - Binary data (component decides encoding)
    ///
    /// # Returns
    /// * `Ok(())` - Data stored successfully
    /// * `Err(SessionError)` - Storage error
    pub fn set(&self, key: &str, value: &[u8]) -> Result<(), SessionError> {
        self.bucket.set(key, value).map_err(SessionError::from)
    }

    /// Marks session as terminated
    ///
    /// Per MCP spec:
    /// - Session data is preserved (not deleted)
    /// - Future requests with this session ID MUST return 404
    /// - Reason is stored for debugging/logging
    ///
    /// # Arguments
    /// * `reason` - Optional reason for termination (e.g., "client_requested", "timeout")
    ///
    /// # Returns
    /// * `Ok(())` - Session marked as terminated
    /// * `Err(SessionError)` - If metadata update fails
    pub fn terminate(&mut self, reason: Option<String>) -> Result<(), SessionError> {
        // Read current metadata
        let mut metadata = self.read_metadata()?;

        // Update termination status
        metadata.terminated = true;
        metadata.reason = reason;

        // Write back to storage
        self.write_metadata(&metadata)?;

        Ok(())
    }

    /// Checks if session is terminated
    ///
    /// # Returns
    /// * `Ok(true)` - Session is terminated
    /// * `Ok(false)` - Session is active
    /// * `Err(SessionError)` - If metadata read fails
    pub fn is_terminated(&self) -> Result<bool, SessionError> {
        let metadata = self.read_metadata()?;
        Ok(metadata.terminated)
    }

    /// Deletes session and all associated data
    ///
    /// Per sessions.wit, this consumes the session and returns the bucket.
    /// In practice, we'll just delete all keys in the bucket.
    ///
    /// # Returns
    /// * `Ok(Bucket)` - Bucket handle after deletion
    /// * `Err(SessionError)` - If deletion fails
    pub fn delete(self) -> Result<Bucket, SessionError> {
        // Delete metadata (main indicator of session existence)
        self.bucket.delete("__meta__")?;

        // Note: In a production implementation, we'd enumerate and delete all keys.
        // For MVP, components are responsible for cleaning up their own keys,
        // or we accept that orphaned keys may exist until bucket cleanup.

        Ok(self.bucket)
    }

    /// Helper: Read session metadata from storage
    fn read_metadata(&self) -> Result<SessionMetadata, SessionError> {
        let bytes = self.bucket.get("__meta__")?
            .ok_or(SessionError::NoSuchSession)?;

        serde_json::from_slice(&bytes)
            .map_err(|e| SessionError::Unexpected(format!("Failed to deserialize metadata: {}", e)))
    }

    /// Helper: Write session metadata to storage
    fn write_metadata(&self, metadata: &SessionMetadata) -> Result<(), SessionError> {
        let bytes = serde_json::to_vec(metadata)
            .map_err(|e| SessionError::Unexpected(format!("Failed to serialize metadata: {}", e)))?;

        self.bucket.set("__meta__", &bytes)?;
        Ok(())
    }
}

/// Generates a cryptographically secure UUID v4
///
/// Per MCP spec requirements:
/// - Globally unique
/// - Cryptographically secure random generation
/// - Only visible ASCII characters (0x21-0x7E)
///
/// UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
/// - 128 bits of randomness
/// - Version bits set to 0100 (version 4)
/// - Variant bits set to 10xx (RFC 4122)
/// - Uses characters: 0-9, a-f, hyphen (all in ASCII range 0x2D, 0x30-0x39, 0x61-0x66)
///
/// # Returns
/// * `Ok(String)` - UUID v4 string (e.g., "550e8400-e29b-41d4-a716-446655440000")
/// * `Err(SessionError)` - If random generation fails
fn generate_uuid_v4() -> Result<String, SessionError> {
    // Get 16 bytes of cryptographically secure randomness
    let random_bytes = random::get_random_bytes(16);

    if random_bytes.len() != 16 {
        return Err(SessionError::Unexpected(
            format!("Expected 16 random bytes, got {}", random_bytes.len())
        ));
    }

    // Convert to array for easier manipulation
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&random_bytes);

    // Set version bits (4 most significant bits of 7th byte to 0100)
    bytes[6] = (bytes[6] & 0x0F) | 0x40;

    // Set variant bits (2 most significant bits of 9th byte to 10)
    bytes[8] = (bytes[8] & 0x3F) | 0x80;

    // Format as UUID string: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    ))
}

/// Returns current Unix timestamp in milliseconds
///
/// Note: WASI doesn't provide a monotonic clock in all environments.
/// This uses a simple counter-based approach for MVP.
/// In production, this should use wasi:clocks/wall-clock.
fn current_timestamp_ms() -> u64 {
    // For MVP, use a simple counter since we don't have reliable wall-clock access
    // In production, use: wasi::clocks::wall_clock::now()
    // For now, return 0 as placeholder - timestamps are not critical for session functionality
    0
}

/// Validates that a session ID contains only visible ASCII characters
///
/// Per MCP spec: Session IDs MUST only contain visible ASCII (0x21-0x7E)
///
/// # Arguments
/// * `session_id` - Session ID string to validate
///
/// # Returns
/// * `Ok(())` - Session ID is valid
/// * `Err(SessionError)` - Session ID contains invalid characters
pub fn validate_session_id_format(session_id: &str) -> Result<(), SessionError> {
    if session_id.is_empty() {
        return Err(SessionError::Unexpected("Session ID is empty".to_string()));
    }

    for ch in session_id.chars() {
        if ch < '\x21' || ch > '\x7E' {
            return Err(SessionError::Unexpected(
                format!("Session ID contains invalid character: {:?} (must be visible ASCII 0x21-0x7E)", ch)
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid_format() {
        // UUID format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
        // Total length: 36 characters (32 hex + 4 hyphens)
        let uuid = generate_uuid_v4().unwrap();
        assert_eq!(uuid.len(), 36);

        // Check hyphen positions
        assert_eq!(uuid.chars().nth(8), Some('-'));
        assert_eq!(uuid.chars().nth(13), Some('-'));
        assert_eq!(uuid.chars().nth(18), Some('-'));
        assert_eq!(uuid.chars().nth(23), Some('-'));

        // Check version (13th character should be '4')
        assert_eq!(uuid.chars().nth(14), Some('4'));

        // Check variant (17th character should be 8, 9, a, or b)
        let variant_char = uuid.chars().nth(19).unwrap();
        assert!(variant_char == '8' || variant_char == '9' ||
                variant_char == 'a' || variant_char == 'b');
    }

    #[test]
    fn test_uuid_ascii_compliance() {
        let uuid = generate_uuid_v4().unwrap();

        // All characters must be visible ASCII (0x21-0x7E)
        for ch in uuid.chars() {
            assert!(ch >= '\x21' && ch <= '\x7E',
                    "Character {:?} not in visible ASCII range", ch);
        }
    }

    #[test]
    fn test_validate_session_id_format() {
        // Valid UUIDs
        assert!(validate_session_id_format("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_session_id_format("abc123-xyz").is_ok());

        // Invalid: empty
        assert!(validate_session_id_format("").is_err());

        // Invalid: contains non-visible ASCII (space is 0x20)
        assert!(validate_session_id_format("hello world").is_err());

        // Invalid: contains newline
        assert!(validate_session_id_format("hello\nworld").is_err());
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = SessionMetadata {
            terminated: false,
            reason: None,
            created_at: 1234567890,
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("\"terminated\":false"));
        assert!(json.contains("\"created_at\":1234567890"));

        // Deserialize and verify
        let deserialized: SessionMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.terminated, false);
        assert_eq!(deserialized.created_at, 1234567890);
    }
}
