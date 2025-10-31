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

/// Session data stored in KV with session ID as key
///
/// The entire session is stored as a single JSON object containing
/// both metadata (__meta__) and application data (data).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionData {
    /// Session metadata (termination status, timestamps, etc.)
    #[serde(rename = "__meta__")]
    meta: SessionMetadata,
    /// Application data storage (arbitrary JSON object)
    #[serde(default)]
    data: serde_json::Value,
}

/// Session metadata
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
        SessionError::Store(format!("{:?}", e))
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
        eprintln!("[SESSION_INIT] Initializing new session with bucket '{}'", bucket_name);
        // Open KV bucket
        let bucket = store::open(bucket_name)
            .map_err(|e| {
                eprintln!("[SESSION_INIT] Failed to open bucket: {:?}", e);
                SessionError::Store(format!("Failed to open bucket '{}': {:?}", bucket_name, e))
            })?;
        eprintln!("[SESSION_INIT] Bucket opened successfully");

        // Generate cryptographically secure UUID v4
        let session_id = generate_uuid_v4()?;
        eprintln!("[SESSION_INIT] Generated session ID: {}", session_id);

        // Create initial session data
        let session_data = SessionData {
            meta: SessionMetadata {
                terminated: false,
                reason: None,
                created_at: current_timestamp_ms(),
            },
            data: serde_json::Value::Object(serde_json::Map::new()), // Empty data object
        };

        // Serialize and store with session ID as the key
        let session_json = serde_json::to_vec(&session_data)
            .map_err(|e| SessionError::Unexpected(format!("Failed to serialize session data: {}", e)))?;
        eprintln!("[SESSION_INIT] Serialized session data, writing to key '{}'", session_id);

        bucket.set(&session_id, &session_json)?;
        eprintln!("[SESSION_INIT] Session data written successfully");

        // Verify it was written
        let exists = bucket.exists(&session_id)?;
        eprintln!("[SESSION_INIT] Verification: session '{}' exists = {}", session_id, exists);

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
        eprintln!("[SESSION_OPEN] Opening session '{}' with bucket '{}'", id, bucket_name);
        // Open KV bucket
        let bucket = store::open(bucket_name)
            .map_err(|e| {
                eprintln!("[SESSION_OPEN] Failed to open bucket: {:?}", e);
                SessionError::Store(format!("Failed to open bucket '{}': {:?}", bucket_name, e))
            })?;
        eprintln!("[SESSION_OPEN] Bucket opened successfully");

        // Check if session exists using session ID as key
        eprintln!("[SESSION_OPEN] Checking if session '{}' exists", id);
        let exists = bucket.exists(id)?;
        eprintln!("[SESSION_OPEN] Session '{}' exists: {}", id, exists);
        if !exists {
            eprintln!("[SESSION_OPEN] Session '{}' not found, returning NoSuchSession", id);
            return Err(SessionError::NoSuchSession);
        }

        eprintln!("[SESSION_OPEN] Session opened successfully");
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
        let session_data = self.read_session_data()?;
        Ok(session_data.meta.terminated)
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

    /// Helper: Read session data from storage
    fn read_session_data(&self) -> Result<SessionData, SessionError> {
        let bytes = self.bucket.get(&self.id)?
            .ok_or(SessionError::NoSuchSession)?;

        serde_json::from_slice(&bytes)
            .map_err(|e| SessionError::Unexpected(format!("Failed to deserialize session data: {}", e)))
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
    fn test_session_data_serialization() {
        let session_data = SessionData {
            meta: SessionMetadata {
                terminated: false,
                reason: None,
                created_at: 1234567890,
            },
            data: serde_json::json!({}),
        };

        let json = serde_json::to_string(&session_data).unwrap();
        assert!(json.contains("\"__meta__\""));
        assert!(json.contains("\"terminated\":false"));
        assert!(json.contains("\"created_at\":1234567890"));
        assert!(json.contains("\"data\""));

        // Deserialize and verify
        let deserialized: SessionData = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.meta.terminated, false);
        assert_eq!(deserialized.meta.created_at, 1234567890);
    }
}
