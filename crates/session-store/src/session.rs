//! Session implementation for MCP
//!
//! Provides session management with WASI KV backend storage.
//! Implements both user-facing data access (sessions interface) and
//! transport-layer lifecycle management (session-manager interface).

use crate::bindings::exports::wasmcp::mcp_v20250618::sessions::{
    ElicitRequest, ElicitResult, GuestFutureElicitResult, GuestSession, Session, SessionError,
};
use crate::bindings::wasi::io::poll::Pollable;
use crate::bindings::wasi::io::streams::OutputStream;
use crate::bindings::wasmcp::keyvalue::store::{
    self as kv_store, Bucket, Error as KvError, TypedValue,
};
use serde::{Deserialize, Serialize};

/// Convert KV store error to SessionError
fn kv_to_session_error(e: KvError) -> SessionError {
    match e {
        KvError::NoSuchStore => SessionError::Store("Store does not exist".to_string()),
        KvError::AccessDenied => SessionError::Store("Access denied to store".to_string()),
        KvError::Other(msg) => SessionError::Store(msg),
    }
}

// ============================================================================
// Internal Storage Types (NOT exposed via WIT)
// ============================================================================

/// Default session lifetime (24 hours) for sessions without JWT expiration
const DEFAULT_SESSION_LIFETIME_SECONDS: u64 = 24 * 60 * 60; // 24 hours

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

    /// Unix timestamp in seconds when session expires (from JWT exp claim)
    /// If None, session has no expiration
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<u64>,

    /// Optional reason for termination (if terminated)
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

impl Default for SessionMetadata {
    fn default() -> Self {
        let created_at = current_timestamp_ms();
        // Set default expiration to 24 hours from creation
        let expires_at = Some((created_at / 1000) + DEFAULT_SESSION_LIFETIME_SECONDS);

        Self {
            terminated: false,
            created_at,
            expires_at,
            reason: None,
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
    pub fn initialize(store_id: String) -> Result<String, SessionError> {
        // Generate new session ID
        let session_id = generate_uuid_v4();

        // Open bucket
        let bucket = kv_store::open(&store_id).map_err(kv_to_session_error)?;

        // Create initial metadata
        let metadata = SessionMetadata::default();
        let metadata_json = serde_json::to_string(&metadata).map_err(|e| {
            SessionError::Unexpected(format!("Failed to serialize metadata: {}", e))
        })?;

        // Store metadata at session_id:__meta__ as JSON string
        let kv_key = meta_key(&session_id);
        bucket.set_json(&kv_key, &metadata_json).map_err(|e| {
            eprintln!(
                "[SessionManager] CRITICAL: Failed to write session {} during initialization: {:?}",
                session_id, e
            );
            kv_to_session_error(e)
        })?;

        Ok(session_id)
    }

    /// Validate session is active
    ///
    /// Returns true if session exists, is not terminated, and is not expired.
    /// Returns false if session is inactive for any reason.
    pub fn validate(session_id: String, store_id: String) -> Result<bool, SessionError> {
        let bucket = kv_store::open(&store_id).map_err(kv_to_session_error)?;

        // Check if session is active (exists, not terminated, not expired)
        match is_session_active(&bucket, &session_id) {
            Ok(_) => Ok(true),
            Err(SessionError::NoSuchSession) => {
                // Session doesn't exist, is terminated, or is expired
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// Mark session as terminated (soft delete)
    ///
    /// Updates session metadata to mark as terminated with optional reason.
    /// Data remains in storage but session cannot be used for new requests.
    pub fn mark_terminated(
        session_id: String,
        store_id: String,
        reason: Option<String>,
    ) -> Result<(), SessionError> {
        let bucket = kv_store::open(&store_id).map_err(kv_to_session_error)?;

        // Read current metadata
        let kv_key = meta_key(&session_id);
        let metadata_json = bucket
            .get_json(&kv_key)
            .map_err(kv_to_session_error)?
            .ok_or(SessionError::NoSuchSession)?;

        let mut metadata: SessionMetadata = serde_json::from_str(&metadata_json).map_err(|e| {
            SessionError::Unexpected(format!(
                "Failed to parse metadata for session {}: {} - corrupt data: {}",
                session_id,
                e,
                &metadata_json[..metadata_json.len().min(200)]
            ))
        })?;

        // Update metadata
        metadata.terminated = true;
        metadata.reason = reason;

        // Write back
        let updated_json = serde_json::to_string(&metadata).map_err(|e| {
            SessionError::Unexpected(format!("Failed to serialize metadata: {}", e))
        })?;

        bucket.set_json(&kv_key, &updated_json).map_err(|e| {
            eprintln!(
                "[SessionManager] CRITICAL: Failed to write termination for session {}: {:?}",
                session_id, e
            );
            kv_to_session_error(e)
        })?;

        Ok(())
    }

    /// Delete session from storage (hard delete)
    ///
    /// Deletes all session data including metadata and user keys.
    /// Uses paginated list_keys() to find all keys with session_id prefix
    /// and deletes them in batches.
    pub fn delete_session(session_id: String, store_id: String) -> Result<(), SessionError> {
        let bucket = kv_store::open(&store_id).map_err(kv_to_session_error)?;

        // Build session prefix (session_id:)
        let session_prefix = format!("{}:", session_id);

        // Paginate through all keys and delete those matching this session
        let mut cursor: Option<String> = None;
        loop {
            let response = bucket
                .list_keys(cursor.as_deref())
                .map_err(kv_to_session_error)?;

            // Filter keys belonging to this session
            let session_keys: Vec<String> = response
                .keys
                .into_iter()
                .filter(|k| k.starts_with(&session_prefix))
                .collect();

            // Delete in batch if any found
            if !session_keys.is_empty() {
                bucket
                    .delete_many(&session_keys)
                    .map_err(kv_to_session_error)?;
            }

            // Check if more pages exist
            cursor = response.cursor;
            if cursor.is_none() {
                break;
            }
        }

        Ok(())
    }

    /// Set session expiration timestamp
    ///
    /// Updates session metadata to expire at the specified Unix timestamp (seconds).
    /// Sessions are automatically invalidated when current time >= expires_at.
    pub fn set_expiration(
        session_id: String,
        store_id: String,
        expires_at: u64,
    ) -> Result<(), SessionError> {
        let bucket = kv_store::open(&store_id).map_err(kv_to_session_error)?;

        // Read current metadata
        let kv_key = meta_key(&session_id);
        let metadata_json = bucket
            .get_json(&kv_key)
            .map_err(kv_to_session_error)?
            .ok_or(SessionError::NoSuchSession)?;

        let mut metadata: SessionMetadata = serde_json::from_str(&metadata_json).map_err(|e| {
            SessionError::Unexpected(format!(
                "Failed to parse metadata for session {}: {} - corrupt data: {}",
                session_id,
                e,
                &metadata_json[..metadata_json.len().min(200)]
            ))
        })?;

        // Update expiration
        metadata.expires_at = Some(expires_at);

        // Write back
        let updated_json = serde_json::to_string(&metadata).map_err(|e| {
            SessionError::Unexpected(format!("Failed to serialize metadata: {}", e))
        })?;

        bucket.set_json(&kv_key, &updated_json).map_err(|e| {
            eprintln!(
                "[SessionManager] CRITICAL: Failed to write expiration for session {}: {:?}",
                session_id, e
            );
            kv_to_session_error(e)
        })?;

        Ok(())
    }
}

// ============================================================================
// Session Resource Implementation (User-Facing)
// ============================================================================

/// Session resource that manages stateful data in WASI KV
///
/// Storage model: session_id:key pattern
/// - Metadata: session_id:__meta__
/// - User keys: session_id:user_key
pub struct SessionImpl {
    bucket: Bucket,
    session_id: String,
    store_id: String, // Needed for terminate() to call session-manager
}

/// Magic string for metadata field in session storage
const META_FIELD: &str = "__meta__";

/// Reserved key names that user tools cannot use
const RESERVED_KEYS: &[&str] = &[META_FIELD, "__metadata__", "metadata", "meta"];

/// Build KV key for session metadata
fn meta_key(session_id: &str) -> String {
    format!("{}:{}", session_id, META_FIELD)
}

/// Build KV key for user data
fn user_key(session_id: &str, key: &str) -> String {
    format!("{}:{}", session_id, key)
}

/// Maximum size for a single key (1KB)
const MAX_KEY_SIZE: usize = 1024;

/// Maximum size for a single value (1MB)
const MAX_VALUE_SIZE: usize = 1024 * 1024;

/// Validate that a session ID is a properly formatted UUID v4
///
/// UUID v4 format: 8-4-4-4-12 hex digits with hyphens (36 characters total)
/// Example: "550e8400-e29b-41d4-a716-446655440000"
fn validate_session_id(session_id: &str) -> Result<(), SessionError> {
    // Check length (UUID v4 is always 36 characters with hyphens)
    if session_id.len() != 36 {
        return Err(SessionError::Unexpected(format!(
            "Invalid session ID format: expected 36 characters, got {}",
            session_id.len()
        )));
    }

    // Check structure: 8-4-4-4-12 with hyphens at positions 8, 13, 18, 23
    let parts: Vec<&str> = session_id.split('-').collect();
    if parts.len() != 5 {
        return Err(SessionError::Unexpected(
            "Invalid session ID format: expected UUID format (8-4-4-4-12)".to_string(),
        ));
    }

    // Validate each part length and hex characters
    let expected_lengths = [8, 4, 4, 4, 12];
    for (i, (part, &expected_len)) in parts.iter().zip(&expected_lengths).enumerate() {
        if part.len() != expected_len {
            return Err(SessionError::Unexpected(format!(
                "Invalid session ID format: part {} has length {}, expected {}",
                i + 1,
                part.len(),
                expected_len
            )));
        }

        if !part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(SessionError::Unexpected(format!(
                "Invalid session ID format: part {} contains non-hex characters",
                i + 1
            )));
        }
    }

    Ok(())
}

impl GuestSession for SessionImpl {
    fn open(session_id: String, store_id: String) -> Result<Session, SessionError> {
        // Validate session ID format (UUID v4)
        validate_session_id(&session_id)?;

        let bucket = kv_store::open(&store_id).map_err(kv_to_session_error)?;

        // Validate session is active (checks metadata key exists and is valid)
        is_session_active(&bucket, &session_id)?;

        Ok(Session::new(SessionImpl {
            bucket,
            session_id,
            store_id, // Store for later use in terminate()
        }))
    }

    fn id(&self) -> String {
        self.session_id.clone()
    }

    fn get(&self, key: String) -> Result<Option<TypedValue>, SessionError> {
        // Validate key before accessing storage
        validate_user_key(&key)?;

        // Read typed value directly from KV using session_id:key pattern
        let kv_key = user_key(&self.session_id, &key);
        let value = self.bucket.get(&kv_key).map_err(kv_to_session_error)?;

        Ok(value)
    }

    fn set(&self, key: String, value: TypedValue) -> Result<(), SessionError> {
        // Validate key
        validate_user_key(&key)?;

        // Validate value size based on type
        validate_typed_value_size(&value)?;

        // Write typed value directly to KV using session_id:key pattern
        let kv_key = user_key(&self.session_id, &key);
        self.bucket
            .set(&kv_key, &value)
            .map_err(kv_to_session_error)?;

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
        // Call internal SessionManager implementation directly
        // SessionManager returns SessionError, which is the same type we return, so no mapping needed
        SessionManager::mark_terminated(self.session_id.clone(), self.store_id.clone(), reason)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Validate that a user-provided key is safe
///
/// Rejects:
/// - Empty keys
/// - Reserved key names (metadata, meta, etc.)
/// - Keys exceeding size limits
///
/// Note: Colons are now allowed in keys since the session_id prefix provides
/// proper namespace isolation (session_id:user_key format).
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

/// Validate typed value size
fn validate_typed_value_size(value: &TypedValue) -> Result<(), SessionError> {
    let size = match value {
        TypedValue::AsString(s) => s.len(),
        TypedValue::AsJson(j) => j.len(),
        TypedValue::AsU64(_) => 8,
        TypedValue::AsS64(_) => 8,
        TypedValue::AsBool(_) => 1,
        TypedValue::AsBytes(b) => b.len(),
    };

    if size > MAX_VALUE_SIZE {
        return Err(SessionError::Unexpected(format!(
            "Value exceeds maximum size of {} bytes",
            MAX_VALUE_SIZE
        )));
    }
    Ok(())
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
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)  // Handle time going backwards gracefully
        .as_millis() as u64
}

fn current_timestamp_s() -> u64 {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)  // Handle time going backwards gracefully
        .as_secs()
}

/// Check if a session is active (exists, not terminated, not expired)
///
/// This is the canonical validation function used by both SessionManager::validate()
/// and SessionImpl::open() to ensure consistent behavior.
///
/// Returns:
/// - Ok(()) if session is active
/// - Err(SessionError::NoSuchSession) if session doesn't exist, is terminated, or is expired
///
/// Per MCP spec, clients don't need to distinguish why a session is inactive - they all
/// result in HTTP 404 and require reinitialization.
///
/// # TOCTOU Race Condition
///
/// Note: There is a theoretical time-of-check/time-of-use (TOCTOU) race condition where
/// a session could expire or be terminated between validation and first use. In practice,
/// this is negligible in the WASM per-request model (microseconds between calls), and
/// cannot be fixed without compare-and-swap atomic operations in the KV store.
/// Session validation is "best effort" and not atomic.
fn is_session_active(bucket: &Bucket, session_id: &str) -> Result<(), SessionError> {
    let kv_key = meta_key(session_id);

    // Check 1: Metadata exists
    if !bucket.exists(&kv_key).map_err(kv_to_session_error)? {
        eprintln!(
            "[Session] Validation failed for {}: metadata not found",
            session_id
        );
        return Err(SessionError::NoSuchSession);
    }

    // Read metadata
    let json = bucket
        .get_json(&kv_key)
        .map_err(kv_to_session_error)?
        .ok_or_else(|| {
            eprintln!(
                "[Session] Validation failed for {}: metadata disappeared (race condition)",
                session_id
            );
            SessionError::NoSuchSession
        })?;

    let metadata: SessionMetadata = serde_json::from_str(&json).map_err(|e| {
        eprintln!(
            "[Session] Validation failed for {}: corrupt metadata - {}",
            session_id, e
        );
        SessionError::Unexpected(format!(
            "Failed to parse metadata for session {}: {} - corrupt data: {}",
            session_id,
            e,
            &json[..json.len().min(200)]
        ))
    })?;

    // Check 2: Not terminated
    if metadata.terminated {
        eprintln!(
            "[Session] Validation failed for {}: terminated (reason: {:?})",
            session_id, metadata.reason
        );
        return Err(SessionError::NoSuchSession);
    }

    // Check 3: Not expired
    if let Some(expires_at) = metadata.expires_at {
        let now = current_timestamp_s();
        if now >= expires_at {
            eprintln!(
                "[Session] Validation failed for {}: expired at {} (now: {})",
                session_id, expires_at, now
            );
            return Err(SessionError::NoSuchSession);
        }
    }

    Ok(())
}

// ============================================================================
// Future Elicit Result (MVP Stub)
// ============================================================================

/// Future for elicit results - MVP stub
///
/// NOTE: This is unreachable in MVP because Session::elicit() always returns an error.
/// These methods should never be called since the FutureElicitResult is never returned successfully.
pub struct FutureElicitResultImpl;

impl GuestFutureElicitResult for FutureElicitResultImpl {
    fn subscribe(&self) -> Pollable {
        // MVP: This should never be called since elicit() returns error
        // Using unimplemented!() instead of panic!() to document intentional non-implementation
        unimplemented!(
            "FutureElicitResult::subscribe not implemented in MVP - elicit() always errors"
        )
    }

    fn elicit_result(&self) -> ElicitResult {
        // MVP: This should never be called since elicit() returns error
        // Using unimplemented!() instead of panic!() to document intentional non-implementation
        unimplemented!(
            "FutureElicitResult::elicit_result not implemented in MVP - elicit() always errors"
        )
    }
}
