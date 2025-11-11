//! HTTP session management
//!
//! This module provides session lifecycle management for HTTP transport:
//! - Session validation and retrieval from headers
//! - Session initialization during connection setup
//! - Session deletion on client disconnect
//! - Session requirement enforcement for non-initialize requests

use crate::bindings::wasi::http::types::IncomingRequest;
use crate::bindings::wasmcp::mcp_v20250618::session_manager::{
    SessionError, delete_session as manager_delete_session, initialize as manager_initialize,
    validate as manager_validate,
};
use crate::config::SessionConfig;
use crate::error::TransportError;
use crate::http::validation;

/// Validate and retrieve session ID from request
///
/// Returns:
/// - Ok(Some(session_id)) if session header present and valid
/// - Ok(None) if no session header or sessions disabled
/// - Err(TransportError) if validation fails
pub fn validate_session_from_request(
    request: &IncomingRequest,
    session_config: &SessionConfig,
) -> Result<Option<String>, TransportError> {
    // Extract session ID from header
    let session_id_raw = validation::extract_session_id_header(request)?;

    if let Some(session_str) = session_id_raw {
        // Only validate if sessions are enabled
        if session_config.enabled {
            let bucket = session_config.get_bucket();

            match manager_validate(&session_str, bucket) {
                Ok(true) => Ok(Some(session_str)),
                Ok(false) => Err(TransportError::session("Session terminated")),
                Err(SessionError::NoSuchSession) => {
                    Err(TransportError::session("Session not found"))
                }
                Err(_) => Err(TransportError::session("Session validation error")),
            }
        } else {
            // Sessions disabled but client sent session ID - ignore it
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Check if session is required for the current request
///
/// For non-initialize requests when sessions are enabled, a session ID must be present.
/// Returns true if session requirement is satisfied, false otherwise.
pub fn check_session_required(session_config: &SessionConfig, session_id: Option<&str>) -> bool {
    // If sessions enabled, session ID must be present for non-initialize requests
    !(session_config.enabled && session_id.is_none())
}

/// Initialize a new session during connection setup
///
/// Returns session ID if sessions are enabled and initialization succeeds,
/// None otherwise.
pub fn initialize_session(session_config: &SessionConfig) -> Option<String> {
    if session_config.enabled {
        let bucket = session_config.get_bucket();
        manager_initialize(bucket).ok()
    } else {
        None
    }
}

/// Delete session by ID
///
/// Returns:
/// - Ok(()) if session deleted successfully
/// - Err(TransportError) with appropriate error message
pub fn delete_session_by_id(
    session_id: &str,
    session_config: &SessionConfig,
) -> Result<(), TransportError> {
    let bucket = session_config.get_bucket();

    match manager_delete_session(session_id, bucket) {
        Ok(_) => Ok(()),
        Err(SessionError::NoSuchSession) => Err(TransportError::session("Session not found")),
        Err(_) => Err(TransportError::session("Failed to delete session")),
    }
}

/// Bind JWT identity to session during initialization
///
/// Stores JWT claims in session storage for:
/// - Identity persistence across requests
/// - Session TTL matching JWT expiration
/// - Authorization without re-validating JWT
pub fn bind_identity_to_session(
    session_id: &str,
    identity: &crate::bindings::wasmcp::mcp_v20250618::mcp::Identity,
    session_config: &SessionConfig,
) {
    use crate::bindings::wasmcp::mcp_v20250618::sessions::{self, Session};

    let bucket = session_config.get_bucket();

    // Open session resource
    let session = match Session::open(session_id, bucket) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "[transport:session] Failed to open session for identity binding: {:?}",
                e
            );
            return;
        }
    };

    // Store subject (user ID) from JWT claims
    let subject = sessions::get_subject(&identity.claims);
    if let Err(e) = session.set("jwt:sub", subject.as_bytes()) {
        eprintln!("[transport:session] Failed to store JWT subject: {:?}", e);
    }

    // Store issuer
    if let Some(issuer) = sessions::get_issuer(&identity.claims)
        && let Err(e) = session.set("jwt:iss", issuer.as_bytes())
    {
        eprintln!("[transport:session] Failed to store JWT issuer: {:?}", e);
    }

    // Store scopes as comma-separated list
    let scopes = sessions::get_scopes(&identity.claims);
    let scopes_str = scopes.join(",");
    if let Err(e) = session.set("jwt:scopes", scopes_str.as_bytes()) {
        eprintln!("[transport:session] Failed to store JWT scopes: {:?}", e);
    }

    // Store audiences as comma-separated list
    let audiences = sessions::get_audiences(&identity.claims);
    let audiences_str = audiences.join(",");
    if let Err(e) = session.set("jwt:audiences", audiences_str.as_bytes()) {
        eprintln!("[transport:session] Failed to store JWT audiences: {:?}", e);
    }

    // Store expiration timestamp if available and set session TTL
    if let Some(exp) = identity.claims.expiration {
        let exp_str = exp.to_string();
        if let Err(e) = session.set("jwt:exp", exp_str.as_bytes()) {
            eprintln!(
                "[transport:session] Failed to store JWT expiration: {:?}",
                e
            );
        }

        // Set session expiration to match JWT expiration
        use crate::bindings::wasmcp::mcp_v20250618::session_manager;
        if let Err(e) = session_manager::set_expiration(session_id, bucket, exp) {
            eprintln!(
                "[transport:session] Failed to set session expiration: {:?}",
                e
            );
        } else {
            eprintln!("[transport:session] Session will expire at {}", exp);
        }
    }

    // Store issued-at timestamp if available
    if let Some(iat) = identity.claims.issued_at {
        let iat_str = iat.to_string();
        if let Err(e) = session.set("jwt:iat", iat_str.as_bytes()) {
            eprintln!("[transport:session] Failed to store JWT issued-at: {:?}", e);
        }
    }

    eprintln!(
        "[transport:session] Bound identity to session: sub={}, scopes={}",
        subject, scopes_str
    );
}
