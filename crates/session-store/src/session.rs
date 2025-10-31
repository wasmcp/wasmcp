//! Session implementation for MCP
//!
//! Provides session management with WASI KV backend storage.
//! Session IDs are pre-validated by http-transport, no UUID generation here.

use crate::bindings::exports::wasmcp::mcp_v20250618::sessions::{
    ElicitRequest, ElicitResult, GuestFutureElicitResult, GuestSession, Session, SessionError,
};
use crate::bindings::wasi::io::poll::Pollable;
use crate::bindings::wasi::io::streams::OutputStream;
use crate::bindings::wasi::keyvalue::store::{self as kv_store, Bucket};

/// Session resource that manages stateful data in WASI KV
pub struct SessionImpl {
    bucket: Bucket,
    session_id: String,
}

/// Reserved key names that user tools cannot use
const RESERVED_KEYS: &[&str] = &["metadata", "meta", "__meta__", "__metadata__"];

/// Maximum size for a single key (1KB)
const MAX_KEY_SIZE: usize = 1024;

/// Maximum size for a single value (1MB)
const MAX_VALUE_SIZE: usize = 1024 * 1024;

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
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD.encode(bytes)
}

/// Base64 decode string from JSON storage
fn base64_decode(s: &str) -> Result<Vec<u8>, SessionError> {
    use base64::{engine::general_purpose, Engine as _};
    general_purpose::STANDARD
        .decode(s)
        .map_err(|e| SessionError::Unexpected(format!("Failed to decode base64: {}", e)))
}

impl SessionImpl {
    /// Create a new session by opening the KV bucket
    pub fn new(session_id: String, store_id: String) -> Result<Self, SessionError> {
        // Open the KV bucket (store_id only used here, not stored)
        let bucket = kv_store::open(&store_id).map_err(map_kv_error)?;

        // Session IDs come pre-validated from http-transport
        Ok(SessionImpl { bucket, session_id })
    }

    /// Delete session data
    pub fn cleanup(self) -> Result<(), SessionError> {
        // Middleware-level cleanup is a no-op
        // Transport owns session lifecycle and cleanup
        Ok(())
    }
}

impl GuestSession for SessionImpl {
    fn open(session_id: String, store_id: String) -> Result<Session, SessionError> {
        let impl_session = SessionImpl::new(session_id, store_id)?;
        Ok(Session::new(impl_session))
    }

    fn delete(_session: Session) -> Result<(), SessionError> {
        let impl_session: SessionImpl = _session.into_inner();
        impl_session.cleanup()
    }

    fn id(&self) -> String {
        self.session_id.clone()
    }

    fn get(&self, key: String) -> Result<Option<Vec<u8>>, SessionError> {
        // Validate key before accessing storage
        validate_user_key(&key)?;

        // Read unified session storage
        let storage_bytes = self.bucket.get(&self.session_id).map_err(map_kv_error)?;
        let Some(storage_bytes) = storage_bytes else {
            return Err(SessionError::NoSuchSession);
        };

        // Parse unified storage structure
        let storage: serde_json::Value = serde_json::from_slice(&storage_bytes).map_err(|e| {
            SessionError::Unexpected(format!("Failed to parse session storage: {}", e))
        })?;

        // Extract data field (JSON object as string)
        let data_str = storage
            .get("data")
            .and_then(|d| d.as_str())
            .ok_or_else(|| {
                SessionError::Unexpected("Missing data field in session storage".to_string())
            })?;

        // Parse data JSON object
        let data_obj: serde_json::Value = serde_json::from_str(data_str)
            .map_err(|e| SessionError::Unexpected(format!("Failed to parse data JSON: {}", e)))?;

        // Get the specific key from the data object
        data_obj
            .get(&key)
            .and_then(|v| v.as_str())
            .map(base64_decode)
            .transpose()
    }

    fn set(&self, key: String, value: Vec<u8>) -> Result<(), SessionError> {
        // Validate key and value before accessing storage
        validate_user_key(&key)?;
        validate_value_size(&value)?;

        // Read current unified session storage
        let storage_bytes = self.bucket.get(&self.session_id).map_err(map_kv_error)?;
        let Some(storage_bytes) = storage_bytes else {
            return Err(SessionError::NoSuchSession);
        };

        // Parse unified storage structure
        let mut storage: serde_json::Value =
            serde_json::from_slice(&storage_bytes).map_err(|e| {
                SessionError::Unexpected(format!("Failed to parse session storage: {}", e))
            })?;

        // Extract and parse data field
        let data_str = storage
            .get("data")
            .and_then(|d| d.as_str())
            .ok_or_else(|| {
                SessionError::Unexpected("Missing data field in session storage".to_string())
            })?;

        let mut data_obj: serde_json::Value = serde_json::from_str(data_str)
            .map_err(|e| SessionError::Unexpected(format!("Failed to parse data JSON: {}", e)))?;

        // Update the specific key in the data object
        let encoded_value = base64_encode(&value);
        if let Some(obj) = data_obj.as_object_mut() {
            obj.insert(key, serde_json::Value::String(encoded_value));
        } else {
            return Err(SessionError::Unexpected(
                "Data field is not a JSON object".to_string(),
            ));
        }

        // Serialize updated data back to string
        let updated_data_str = serde_json::to_string(&data_obj)
            .map_err(|e| SessionError::Unexpected(format!("Failed to serialize data: {}", e)))?;

        // Update the storage structure
        if let Some(obj) = storage.as_object_mut() {
            obj.insert(
                "data".to_string(),
                serde_json::Value::String(updated_data_str),
            );
        } else {
            return Err(SessionError::Unexpected(
                "Session storage is not a JSON object".to_string(),
            ));
        }

        // Write back to storage
        let updated_bytes = serde_json::to_vec(&storage)
            .map_err(|e| SessionError::Unexpected(format!("Failed to serialize storage: {}", e)))?;

        self.bucket
            .set(&self.session_id, &updated_bytes)
            .map_err(map_kv_error)
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

    fn terminate(&self, _reason: Option<String>) -> Result<(), SessionError> {
        // Transport owns session lifecycle - this is a no-op at middleware level
        // Termination state is managed by transport layer
        Ok(())
    }

    fn is_terminated(&self) -> Result<bool, SessionError> {
        // Transport validates session before calling downstream
        // This should never be called, but return false if it is
        Ok(false)
    }
}

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

/// Map WASI KV errors to session errors
fn map_kv_error(error: kv_store::Error) -> SessionError {
    match error {
        kv_store::Error::NoSuchStore => SessionError::NoSuchSession,
        kv_store::Error::AccessDenied => SessionError::Store("access denied".to_string()),
        kv_store::Error::Other(msg) => SessionError::Store(msg),
    }
}
