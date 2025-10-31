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
    store_id: String,
}

impl SessionImpl {
    /// Create a new session by opening the KV bucket
    pub fn new(session_id: String, store_id: String) -> Result<Self, SessionError> {
        // Open the KV bucket
        let bucket = kv_store::open(&store_id).map_err(map_kv_error)?;

        // Session IDs come pre-validated from http-transport
        Ok(SessionImpl {
            bucket,
            session_id,
            store_id,
        })
    }

    /// Delete session data
    pub fn cleanup(self) -> Result<(), SessionError> {
        // Delete terminated metadata if it exists
        let metadata_key = format!("session:{}:terminated", self.session_id);
        let _ = self.bucket.delete(&metadata_key);
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
        // Check if session is terminated
        if self.is_terminated()? {
            return Err(SessionError::Store(
                "session is terminated".to_string(),
            ));
        }

        let storage_key = format!("session:{}:{}", self.session_id, key);
        self.bucket.get(&storage_key).map_err(map_kv_error)
    }

    fn set(&self, key: String, value: Vec<u8>) -> Result<(), SessionError> {
        // Check if session is terminated
        if self.is_terminated()? {
            return Err(SessionError::Store(
                "session is terminated".to_string(),
            ));
        }

        let storage_key = format!("session:{}:{}", self.session_id, key);
        self.bucket.set(&storage_key, &value).map_err(map_kv_error)
    }

    fn elicit(
        &self,
        _client: &OutputStream,
        _elicitation: ElicitRequest,
    ) -> Result<crate::bindings::exports::wasmcp::mcp_v20250618::sessions::FutureElicitResult, SessionError> {
        // MVP: Not implemented yet
        Err(SessionError::Unexpected(
            "elicit not implemented in MVP".to_string(),
        ))
    }

    fn terminate(&self, reason: Option<String>) -> Result<(), SessionError> {
        // Mark session as terminated in KV store
        let metadata_key = format!("session:{}:terminated", self.session_id);
        let terminated_value = reason.unwrap_or_else(|| "terminated".to_string());

        self.bucket
            .set(&metadata_key, terminated_value.as_bytes())
            .map_err(map_kv_error)
    }

    fn is_terminated(&self) -> Result<bool, SessionError> {
        // Check KV store for terminated flag
        let metadata_key = format!("session:{}:terminated", self.session_id);

        match self.bucket.exists(&metadata_key) {
            Ok(exists) => Ok(exists),
            Err(e) => Err(map_kv_error(e)),
        }
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
