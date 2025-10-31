use crate::bindings::wasi::clocks::wall_clock;
use crate::bindings::wasi::keyvalue::store::{self, Bucket, Error as StoreError};
use crate::bindings::wasi::random::random;
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

/// Session metadata helper for working with the JSON-based metadata from WIT
///
/// This provides a typed interface to the metadata JSON blob defined in the WIT spec.
/// The WIT type uses arbitrary JSON to allow extensibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMetadataJson {
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
#[allow(dead_code)] // Error details used for debugging/logging
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
        // Use Display trait to avoid exposing internal structure
        match e {
            StoreError::NoSuchStore => SessionError::Store("key-value store not found".to_string()),
            StoreError::AccessDenied => {
                SessionError::Store("access denied to session store".to_string())
            }
            StoreError::Other(msg) => SessionError::Store(format!("storage error: {}", msg)),
        }
    }
}

/// Session manager for http-transport internal session lifecycle management
///
/// This manages session metadata in WASI KV for the transport layer.
/// Each session's metadata is stored using the session ID as the key.
/// All sessions share the same bucket (one bucket per transport instance).
///
/// Storage schema:
/// - Bucket: Shared bucket for all sessions (e.g., "mcp-sessions" or "default")
/// - Key: Session ID (e.g., "fef9e597-c392-41dd-bdec-d90223d5fd0a")
/// - Value: JSON-serialized SessionMetadata
///
/// Note: This is distinct from server_handler::Session which is the record
/// passed to downstream components via RequestCtx.
pub struct SessionManager {
    id: String,
    bucket: Bucket,
}

impl SessionManager {
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
    /// * `Ok(SessionManager)` - New session with unique ID
    /// * `Err(SessionError)` - If bucket open fails or metadata store fails
    pub fn initialize(bucket_name: &str) -> Result<SessionManager, SessionError> {
        // Open KV bucket
        let bucket = store::open(bucket_name)?;

        // Generate cryptographically secure UUID v4
        let session_id = generate_uuid_v4()?;

        // Create initial metadata (follows unified schema from WIT)
        let metadata_json = SessionMetadataJson {
            terminated: false,
            reason: None,
            created_at: current_timestamp_ms(),
        };

        // Serialize metadata to JSON string
        let metadata_str = serde_json::to_string(&metadata_json).map_err(|e| {
            SessionError::Unexpected(format!("Failed to serialize metadata: {}", e))
        })?;

        // Create unified session storage structure
        // {
        //   "metadata": {"json": "{\"terminated\":false,...}"},
        //   "data": "{}"
        // }
        let storage_value = serde_json::json!({
            "metadata": {
                "json": metadata_str
            },
            "data": "{}" // Empty JSON object for user data
        });

        let storage_bytes = serde_json::to_vec(&storage_value).map_err(|e| {
            SessionError::Unexpected(format!("Failed to serialize session storage: {}", e))
        })?;

        bucket.set(&session_id, &storage_bytes)?;

        Ok(SessionManager {
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
    /// * `Ok(SessionManager)` - Existing session
    /// * `Err(SessionError::NoSuchSession)` - If metadata doesn't exist
    /// * `Err(SessionError::Store)` - If bucket open fails
    pub fn open(bucket_name: &str, id: &str) -> Result<SessionManager, SessionError> {
        // Open KV bucket
        let bucket = store::open(bucket_name)?;

        // Check if session exists using session ID as key
        let exists = bucket.exists(id)?;
        if !exists {
            return Err(SessionError::NoSuchSession);
        }

        Ok(SessionManager {
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
    /// Removes the session ID key from the bucket, which contains all session data
    /// (both metadata and application data).
    ///
    /// # Returns
    /// * `Ok(Bucket)` - Bucket handle after deletion
    /// * `Err(SessionError)` - If deletion fails
    pub fn delete(self) -> Result<Bucket, SessionError> {
        // Delete the session by removing the session ID key
        // This removes both metadata and application data in one operation
        self.bucket.delete(&self.id)?;

        Ok(self.bucket)
    }

    /// Helper: Read metadata from unified session storage
    fn read_metadata(&self) -> Result<SessionMetadataJson, SessionError> {
        let bytes = self
            .bucket
            .get(&self.id)?
            .ok_or(SessionError::NoSuchSession)?;

        // Parse the unified storage structure
        let storage: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            SessionError::Unexpected(format!("Failed to parse session storage: {}", e))
        })?;

        // Extract metadata.json field
        let metadata_json_str = storage
            .get("metadata")
            .and_then(|m| m.get("json"))
            .and_then(|j| j.as_str())
            .ok_or_else(|| {
                SessionError::Unexpected(
                    "Missing metadata.json field in session storage".to_string(),
                )
            })?;

        // Deserialize the metadata JSON string
        serde_json::from_str(metadata_json_str).map_err(|e| {
            SessionError::Unexpected(format!("Failed to deserialize metadata JSON: {}", e))
        })
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
        return Err(SessionError::Unexpected(format!(
            "Expected 16 random bytes, got {}",
            random_bytes.len()
        )));
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
/// Uses WASI wall-clock to get the current time as seconds since Unix epoch.
/// The returned value is in milliseconds for consistency with JavaScript/web standards.
///
/// Note: Wall clock is not monotonic and may be affected by system time changes.
fn current_timestamp_ms() -> u64 {
    let datetime = wall_clock::now();
    // Convert to milliseconds: seconds * 1000 + nanoseconds / 1_000_000
    datetime.seconds * 1000 + (datetime.nanoseconds as u64 / 1_000_000)
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
            return Err(SessionError::Unexpected(format!(
                "Session ID contains invalid character: {:?} (must be visible ASCII 0x21-0x7E)",
                ch
            )));
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
        assert!(
            variant_char == '8'
                || variant_char == '9'
                || variant_char == 'a'
                || variant_char == 'b'
        );
    }

    #[test]
    fn test_uuid_ascii_compliance() {
        let uuid = generate_uuid_v4().unwrap();

        // All characters must be visible ASCII (0x21-0x7E)
        for ch in uuid.chars() {
            assert!(
                ch >= '\x21' && ch <= '\x7E',
                "Character {:?} not in visible ASCII range",
                ch
            );
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
    fn test_unified_schema_serialization() {
        // Test metadata JSON serialization
        let metadata = SessionMetadataJson {
            terminated: false,
            reason: None,
            created_at: 1234567890,
        };

        let metadata_str = serde_json::to_string(&metadata).unwrap();
        assert!(metadata_str.contains("\"terminated\":false"));
        assert!(metadata_str.contains("\"created_at\":1234567890"));

        // Test unified storage structure
        let storage = serde_json::json!({
            "metadata": {
                "json": metadata_str
            },
            "data": "{}"
        });

        let storage_str = serde_json::to_string(&storage).unwrap();
        assert!(storage_str.contains("\"metadata\""));
        assert!(storage_str.contains("\"json\""));
        assert!(storage_str.contains("\"data\""));
        assert!(storage_str.contains("\"terminated\":false"));

        // Verify we can deserialize back
        let parsed: serde_json::Value = serde_json::from_str(&storage_str).unwrap();
        let metadata_json_str = parsed["metadata"]["json"].as_str().unwrap();
        let deserialized: SessionMetadataJson = serde_json::from_str(metadata_json_str).unwrap();
        assert_eq!(deserialized.terminated, false);
        assert_eq!(deserialized.created_at, 1234567890);
    }
}
