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

impl SessionImpl {
    /// Create a new session by opening the KV bucket
    pub fn new(session_id: String, store_id: String) -> Result<Self, SessionError> {
        // Open the KV bucket (store_id only used here, not stored)
        let bucket = kv_store::open(&store_id).map_err(map_kv_error)?;

        // Session IDs come pre-validated from http-transport
        Ok(SessionImpl {
            bucket,
            session_id,
        })
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
        // Transport validates session before calling downstream
        // No need to check termination here
        let storage_key = format!("session:{}:{}", self.session_id, key);
        self.bucket.get(&storage_key).map_err(map_kv_error)
    }

    fn set(&self, key: String, value: Vec<u8>) -> Result<(), SessionError> {
        // Transport validates session before calling downstream
        // No need to check termination here
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
