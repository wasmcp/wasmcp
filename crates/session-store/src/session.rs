//! Session implementation for MCP
//!
//! Provides session management with WASI KV backend storage.
//! Implements both user-facing data access (sessions interface) and
//! transport-layer lifecycle management (session-manager interface).

use crate::bindings::exports::wasmcp::mcp_v20250618::session_manager::SessionError as ManagerError;
use crate::bindings::exports::wasmcp::mcp_v20250618::sessions::{
    ElicitRequest, ElicitResult, GuestFutureElicitResult, GuestSession, Session, SessionError,
};
use crate::bindings::wasi::io::poll::Pollable;
use crate::bindings::wasi::io::streams::OutputStream;
use crate::bindings::wasi::keyvalue::store::{self as kv_store, Bucket};
use serde::{Deserialize, Serialize};

// ============================================================================
// Internal Storage Types (NOT exposed via WIT)
// ============================================================================

/// Internal metadata - NOT exposed via WIT
///
/// This structure is stored in the __meta__ field of the session storage.
/// It contains transport-layer metadata that users should not directly access.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionMetadata {
    /// Whether the session has been terminated (soft delete)
    terminated: bool,

    /// Unix timestamp in milliseconds when session was created
    created_at: u64,

    /// Optional reason for termination (if terminated)
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            terminated: false,
            created_at: current_timestamp_ms(),
            reason: None,
        }
    }
}

/// Complete session storage schema
///
/// This is the actual format stored in WASI KV. NOT exposed via WIT.
///
/// Storage structure:
/// {
///   "__meta__": {
///     "terminated": false,
///     "created_at": 1698765432000,
///     "reason": null
///   },
///   "data": {
///     "user_key_1": "base64_encoded_value",
///     "user_key_2": "base64_encoded_value"
///   }
/// }
#[derive(Debug, Serialize, Deserialize)]
struct SessionStorage {
    #[serde(rename = "__meta__")]
    meta: SessionMetadata,
    data: serde_json::Map<String, serde_json::Value>,
}

impl Default for SessionStorage {
    fn default() -> Self {
        Self {
            meta: SessionMetadata::default(),
            data: serde_json::Map::new(),
        }
    }
}

// ============================================================================
// Session Manager Implementation (Transport-Facing)
// ============================================================================

pub struct SessionManager;

impl SessionManager {
    /// Initialize a new session
    ///
    /// Creates a new session with generated UUID, stores initial metadata,
    /// and returns the session ID for inclusion in response headers.
    pub fn initialize(store_id: String) -> Result<String, ManagerError> {
        // Generate new session ID
        let session_id = generate_uuid_v4();

        eprintln!("[SessionManager] Initializing new session: {}", session_id);

        // Open bucket
        let bucket = kv_store::open(&store_id).map_err(|e| {
            ManagerError::Store(format!("Failed to open store '{}': {:?}", store_id, e))
        })?;

        // Create initial storage structure with empty data
        let storage = SessionStorage::default();
        let storage_json = serde_json::to_string(&storage)
            .map_err(|e| ManagerError::Unexpected(format!("Failed to serialize storage: {}", e)))?;

        // Store in KV
        bucket
            .set(&session_id, storage_json.as_bytes())
            .map_err(|e| ManagerError::Store(format!("Failed to store session: {:?}", e)))?;

        eprintln!(
            "[SessionManager] Session {} created successfully",
            session_id
        );
        Ok(session_id)
    }

    /// Validate session exists and is not terminated
    ///
    /// Checks that:
    /// 1. Session exists in storage
    /// 2. Session is not marked as terminated
    pub fn validate(session_id: String, store_id: String) -> Result<bool, ManagerError> {
        eprintln!("[SessionManager] Validating session: {}", session_id);

        let bucket = kv_store::open(&store_id).map_err(|e| {
            ManagerError::Store(format!("Failed to open store '{}': {:?}", store_id, e))
        })?;

        // Check if session exists
        if !bucket
            .exists(&session_id)
            .map_err(|e| ManagerError::Store(format!("Failed to check existence: {:?}", e)))?
        {
            eprintln!("[SessionManager] Session {} does not exist", session_id);
            return Err(ManagerError::NoSuchSession);
        }

        // Read storage and check termination status
        let data = bucket
            .get(&session_id)
            .map_err(|e| ManagerError::Store(format!("Failed to read session: {:?}", e)))?
            .ok_or_else(|| {
                eprintln!(
                    "[SessionManager] Session {} exists but has no data",
                    session_id
                );
                ManagerError::NoSuchSession
            })?;

        let storage: SessionStorage = serde_json::from_slice(&data)
            .map_err(|e| ManagerError::Unexpected(format!("Failed to parse storage: {}", e)))?;

        let is_active = !storage.meta.terminated;
        eprintln!(
            "[SessionManager] Session {} is {}",
            session_id,
            if is_active { "active" } else { "terminated" }
        );

        Ok(is_active)
    }

    /// Mark session as terminated (soft delete)
    ///
    /// Updates session metadata to mark as terminated with optional reason.
    /// Data remains in storage but session cannot be used for new requests.
    pub fn mark_terminated(
        session_id: String,
        store_id: String,
        reason: Option<String>,
    ) -> Result<(), ManagerError> {
        eprintln!(
            "[SessionManager] Marking session {} as terminated",
            session_id
        );
        if let Some(ref r) = reason {
            eprintln!("[SessionManager] Reason: {}", r);
        }

        let bucket = kv_store::open(&store_id).map_err(|e| {
            ManagerError::Store(format!("Failed to open store '{}': {:?}", store_id, e))
        })?;

        // Read current storage
        let data = bucket
            .get(&session_id)
            .map_err(|e| ManagerError::Store(format!("Failed to read session: {:?}", e)))?
            .ok_or(ManagerError::NoSuchSession)?;

        let mut storage: SessionStorage = serde_json::from_slice(&data)
            .map_err(|e| ManagerError::Unexpected(format!("Failed to parse storage: {}", e)))?;

        // Update metadata
        storage.meta.terminated = true;
        storage.meta.reason = reason;

        // Write back
        let storage_json = serde_json::to_string(&storage)
            .map_err(|e| ManagerError::Unexpected(format!("Failed to serialize storage: {}", e)))?;

        bucket
            .set(&session_id, storage_json.as_bytes())
            .map_err(|e| ManagerError::Store(format!("Failed to update session: {:?}", e)))?;

        eprintln!(
            "[SessionManager] Session {} marked as terminated",
            session_id
        );
        Ok(())
    }

    /// Delete session from storage (hard delete)
    ///
    /// Completely removes session and all associated data from storage.
    /// This is a destructive operation that cannot be undone.
    pub fn delete_session(session_id: String, store_id: String) -> Result<(), ManagerError> {
        eprintln!("[SessionManager] Deleting session {}", session_id);

        let bucket = kv_store::open(&store_id).map_err(|e| {
            ManagerError::Store(format!("Failed to open store '{}': {:?}", store_id, e))
        })?;

        bucket
            .delete(&session_id)
            .map_err(|e| ManagerError::Store(format!("Failed to delete session: {:?}", e)))?;

        eprintln!(
            "[SessionManager] Session {} deleted successfully",
            session_id
        );
        Ok(())
    }
}

// ============================================================================
// Session Resource Implementation (User-Facing)
// ============================================================================

/// Session resource that manages stateful data in WASI KV
pub struct SessionImpl {
    bucket: Bucket,
    session_id: String,
    store_id: String, // Needed for terminate() to call session-manager
}

/// Reserved key names that user tools cannot use
const RESERVED_KEYS: &[&str] = &["__meta__", "__metadata__", "metadata", "meta"];

/// Maximum size for a single key (1KB)
const MAX_KEY_SIZE: usize = 1024;

/// Maximum size for a single value (1MB)
const MAX_VALUE_SIZE: usize = 1024 * 1024;

impl GuestSession for SessionImpl {
    fn open(session_id: String, store_id: String) -> Result<Session, SessionError> {
        let bucket = kv_store::open(&store_id)
            .map_err(|e| SessionError::Store(format!("Failed to open store: {:?}", e)))?;

        // Validate session exists
        if !bucket
            .exists(&session_id)
            .map_err(|e| SessionError::Store(format!("Failed to check existence: {:?}", e)))?
        {
            return Err(SessionError::NoSuchSession);
        }

        Ok(Session::new(SessionImpl {
            bucket,
            session_id,
            store_id, // Store for later use in terminate()
        }))
    }

    fn id(&self) -> String {
        self.session_id.clone()
    }

    fn get(&self, key: String) -> Result<Option<Vec<u8>>, SessionError> {
        // Validate key before accessing storage
        validate_user_key(&key)?;

        // Read storage
        let data = self
            .bucket
            .get(&self.session_id)
            .map_err(|e| SessionError::Store(format!("Failed to read session: {:?}", e)))?
            .ok_or(SessionError::NoSuchSession)?;

        let storage: SessionStorage = serde_json::from_slice(&data)
            .map_err(|e| SessionError::Unexpected(format!("Failed to parse storage: {}", e)))?;

        // Access data field
        if let Some(value) = storage.data.get(&key) {
            if let Some(s) = value.as_str() {
                let decoded = base64_decode(s)?;
                Ok(Some(decoded))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn set(&self, key: String, value: Vec<u8>) -> Result<(), SessionError> {
        // Validate key and value
        validate_user_key(&key)?;
        validate_value_size(&value)?;

        // Read current storage
        let data = self
            .bucket
            .get(&self.session_id)
            .map_err(|e| SessionError::Store(format!("Failed to read session: {:?}", e)))?
            .ok_or(SessionError::NoSuchSession)?;

        let mut storage: SessionStorage = serde_json::from_slice(&data)
            .map_err(|e| SessionError::Unexpected(format!("Failed to parse storage: {}", e)))?;

        // Update data field (preserving __meta__)
        let encoded = base64_encode(&value);
        storage.data.insert(key, serde_json::Value::String(encoded));

        // Write back
        let storage_json = serde_json::to_string(&storage)
            .map_err(|e| SessionError::Unexpected(format!("Failed to serialize storage: {}", e)))?;

        self.bucket
            .set(&self.session_id, storage_json.as_bytes())
            .map_err(|e| SessionError::Store(format!("Failed to update session: {:?}", e)))?;

        Ok(())
    }

    fn elicit(
        &self,
        _client: &OutputStream,
        _elicitation: ElicitRequest,
    ) -> Result<
        crate::bindings::exports::wasmcp::mcp_v20250618::sessions::FutureElicitResult,
        SessionError,
    > {
        // MVP: Not implemented yet
        Err(SessionError::Unexpected(
            "elicit not implemented in MVP".to_string(),
        ))
    }

    fn terminate(&self, reason: Option<String>) -> Result<(), SessionError> {
        eprintln!(
            "[Session] Terminating session {} (user-initiated)",
            self.session_id
        );

        // Call internal SessionManager implementation directly
        SessionManager::mark_terminated(self.session_id.clone(), self.store_id.clone(), reason)
            .map_err(|e| match e {
                ManagerError::Store(msg) => SessionError::Store(msg),
                ManagerError::NoSuchSession => SessionError::NoSuchSession,
                ManagerError::Unexpected(msg) => SessionError::Unexpected(msg),
                ManagerError::Io(_) => SessionError::Unexpected("IO error".to_string()),
            })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate that a user-provided key is safe
///
/// Rejects:
/// - Empty keys
/// - Keys containing ':' (could escape session boundary)
/// - Reserved key names (metadata, meta, etc.)
/// - Keys exceeding size limits
fn validate_user_key(key: &str) -> Result<(), SessionError> {
    if key.is_empty() {
        return Err(SessionError::Unexpected("Key cannot be empty".to_string()));
    }

    if key.len() > MAX_KEY_SIZE {
        return Err(SessionError::Unexpected(format!(
            "Key exceeds maximum size of {} bytes",
            MAX_KEY_SIZE
        )));
    }

    if key.contains(':') {
        return Err(SessionError::Unexpected(
            "Key cannot contain ':' character".to_string(),
        ));
    }

    // Check against reserved names (case-insensitive)
    let key_lower = key.to_lowercase();
    if RESERVED_KEYS.iter().any(|r| key_lower == *r) {
        return Err(SessionError::Unexpected(format!(
            "Key '{}' is reserved and cannot be used",
            key
        )));
    }

    Ok(())
}

/// Validate value size
fn validate_value_size(value: &[u8]) -> Result<(), SessionError> {
    if value.len() > MAX_VALUE_SIZE {
        return Err(SessionError::Unexpected(format!(
            "Value exceeds maximum size of {} bytes",
            MAX_VALUE_SIZE
        )));
    }
    Ok(())
}

/// Base64 encode bytes for storage in JSON
fn base64_encode(bytes: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.encode(bytes)
}

/// Base64 decode string from JSON storage
fn base64_decode(s: &str) -> Result<Vec<u8>, SessionError> {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD
        .decode(s)
        .map_err(|e| SessionError::Unexpected(format!("Failed to decode base64: {}", e)))
}

/// Generate UUID v4 using wasi:random
///
/// Creates a cryptographically secure random UUID v4 identifier.
/// Format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx (lowercase)
fn generate_uuid_v4() -> String {
    use crate::bindings::wasi::random::random;

    let bytes = random::get_random_bytes(16);

    // Set version bits (0100 for v4)
    let mut uuid_bytes = [0u8; 16];
    uuid_bytes.copy_from_slice(&bytes);
    uuid_bytes[6] = (uuid_bytes[6] & 0x0F) | 0x40; // Version 4
    uuid_bytes[8] = (uuid_bytes[8] & 0x3F) | 0x80; // Variant 10

    // Format as lowercase hex with hyphens
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        uuid_bytes[0],
        uuid_bytes[1],
        uuid_bytes[2],
        uuid_bytes[3],
        uuid_bytes[4],
        uuid_bytes[5],
        uuid_bytes[6],
        uuid_bytes[7],
        uuid_bytes[8],
        uuid_bytes[9],
        uuid_bytes[10],
        uuid_bytes[11],
        uuid_bytes[12],
        uuid_bytes[13],
        uuid_bytes[14],
        uuid_bytes[15]
    )
}

/// Get current Unix timestamp in milliseconds
fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

// ============================================================================
// Future Elicit Result (MVP Stub)
// ============================================================================

/// Future for elicit results - MVP stub
pub struct FutureElicitResultImpl;

impl GuestFutureElicitResult for FutureElicitResultImpl {
    fn subscribe(&self) -> Pollable {
        // MVP: Return a pollable that never becomes ready
        panic!("FutureElicitResult::subscribe not implemented in MVP")
    }

    fn elicit_result(&self) -> ElicitResult {
        // MVP: Panic if called (shouldn't be called since subscribe never returns ready)
        panic!("FutureElicitResult::elicit_result not implemented in MVP")
    }
}
