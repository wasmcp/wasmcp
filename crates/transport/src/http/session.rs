//! HTTP session management
//!
//! This module provides session lifecycle management for HTTP transport:
//! - Session validation and retrieval from headers
//! - Session initialization during connection setup
//! - Session termination (soft delete) on client disconnect
//! - Session requirement enforcement for non-initialize requests

use crate::bindings::wasi::http::types::IncomingRequest;
use crate::bindings::wasmcp::mcp_v20250618::session_manager::{
    SessionError, initialize as manager_initialize, mark_terminated as manager_mark_terminated,
    validate as manager_validate,
};
use crate::config::TransportConfig;
use crate::error::TransportError;
use crate::http::validation;
use crate::session_keys;

/// Validate and retrieve session ID from request
///
/// Returns:
/// - Ok(Some(session_id)) if session header present and valid
/// - Ok(None) if no session header or sessions disabled
/// - Err(TransportError) if validation fails
pub fn validate_session_from_request(
    request: &IncomingRequest,
    session_config: &TransportConfig,
) -> Result<Option<String>, TransportError> {
    // Extract session ID from header
    let session_id_raw = validation::extract_session_id_header(request)?;

    if let Some(session_str) = session_id_raw {
        // Only validate if sessions are enabled
        if session_config.session_enabled {
            let bucket = session_config.get_session_bucket();

            match manager_validate(&session_str, bucket) {
                Ok(true) => Ok(Some(session_str)),
                Ok(false) => Err(TransportError::session_terminated()),
                Err(SessionError::NoSuchSession) => Err(TransportError::session_not_found()),
                Err(e) => Err(TransportError::session(
                    crate::error::SessionError::ValidationFailed(format!("{:?}", e)),
                )),
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
pub fn check_session_required(session_config: &TransportConfig, session_id: Option<&str>) -> bool {
    // If sessions enabled, session ID must be present for non-initialize requests
    !(session_config.session_enabled && session_id.is_none())
}

/// Initialize a new session during connection setup
///
/// Returns session ID if sessions are enabled and initialization succeeds,
/// None otherwise.
pub fn initialize_session(session_config: &TransportConfig) -> Option<String> {
    if session_config.session_enabled {
        let bucket = session_config.get_session_bucket();
        manager_initialize(bucket).ok()
    } else {
        None
    }
}

/// Terminate session by ID (soft delete)
///
/// Marks the session as terminated without removing data.
/// Background cleanup processes will hard-delete terminated sessions later.
///
/// Returns:
/// - Ok(()) if session terminated successfully
/// - Err(TransportError) with appropriate error message
pub fn delete_session_by_id(
    session_id: &str,
    session_config: &TransportConfig,
) -> Result<(), TransportError> {
    let bucket = session_config.get_session_bucket();

    match manager_mark_terminated(session_id, bucket, Some("Client requested deletion")) {
        Ok(_) => Ok(()),
        Err(SessionError::NoSuchSession) => Err(TransportError::session_not_found()),
        Err(e) => Err(TransportError::session(
            crate::error::SessionError::StorageFailed(format!(
                "Failed to terminate session: {:?}",
                e
            )),
        )),
    }
}

/// Bind JWT identity to session during initialization
///
/// Stores JWT claims in session storage for:
/// - Identity persistence across requests
/// - Session TTL matching JWT expiration
/// - Authorization without re-validating JWT
///
/// Returns Ok(()) if all claims were successfully stored, Err otherwise.
pub fn bind_identity_to_session(
    session_id: &str,
    identity: &crate::bindings::wasmcp::mcp_v20250618::mcp::Identity,
    session_config: &TransportConfig,
) -> Result<(), TransportError> {
    use crate::bindings::wasmcp::auth::helpers;
    use crate::bindings::wasmcp::keyvalue::store::TypedValue;
    use crate::bindings::wasmcp::mcp_v20250618::sessions::Session;

    let bucket = session_config.get_session_bucket();

    // Open session resource
    let session = Session::open(session_id, bucket).map_err(|e| {
        eprintln!(
            "[transport:session] Failed to open session {} for identity binding: {:?}",
            session_id, e
        );
        TransportError::session(crate::error::SessionError::StorageFailed(
            "Failed to open session for identity binding".to_string(),
        ))
    })?;

    // Store subject (user ID) from JWT claims
    let subject = helpers::get_subject(&identity.claims);
    session
        .set(
            session_keys::JWT_SUBJECT,
            &TypedValue::AsBytes(subject.as_bytes().to_vec()),
        )
        .map_err(|e| {
            eprintln!("[transport:session] Failed to store JWT subject: {:?}", e);
            TransportError::session(crate::error::SessionError::StorageFailed(
                "Failed to store JWT subject".to_string(),
            ))
        })?;

    // Store issuer
    if let Some(issuer) = helpers::get_issuer(&identity.claims) {
        session
            .set(
                session_keys::JWT_ISSUER,
                &TypedValue::AsBytes(issuer.as_bytes().to_vec()),
            )
            .map_err(|e| {
                eprintln!("[transport:session] Failed to store JWT issuer: {:?}", e);
                TransportError::session(crate::error::SessionError::StorageFailed(
                    "Failed to store JWT issuer".to_string(),
                ))
            })?;
    }

    // Store scopes as comma-separated list
    let scopes = helpers::get_scopes(&identity.claims);
    let scopes_str = scopes.join(",");
    session
        .set(
            session_keys::JWT_SCOPES,
            &TypedValue::AsBytes(scopes_str.as_bytes().to_vec()),
        )
        .map_err(|e| {
            eprintln!("[transport:session] Failed to store JWT scopes: {:?}", e);
            TransportError::session(crate::error::SessionError::StorageFailed(
                "Failed to store JWT scopes".to_string(),
            ))
        })?;

    // Store audiences as comma-separated list
    let audiences = helpers::get_audiences(&identity.claims);
    let audiences_str = audiences.join(",");
    session
        .set(
            session_keys::JWT_AUDIENCES,
            &TypedValue::AsBytes(audiences_str.as_bytes().to_vec()),
        )
        .map_err(|e| {
            eprintln!("[transport:session] Failed to store JWT audiences: {:?}", e);
            TransportError::session(crate::error::SessionError::StorageFailed(
                "Failed to store JWT audiences".to_string(),
            ))
        })?;

    // Store expiration timestamp if available and set session TTL
    if let Some(exp) = identity.claims.expiration {
        let exp_str = exp.to_string();
        session
            .set(
                session_keys::JWT_EXPIRATION,
                &TypedValue::AsBytes(exp_str.as_bytes().to_vec()),
            )
            .map_err(|e| {
                eprintln!(
                    "[transport:session] Failed to store JWT expiration: {:?}",
                    e
                );
                TransportError::session(crate::error::SessionError::StorageFailed(
                    "Failed to store JWT expiration".to_string(),
                ))
            })?;

        // Set session expiration to match JWT expiration
        use crate::bindings::wasmcp::mcp_v20250618::session_manager;
        session_manager::set_expiration(session_id, bucket, exp).map_err(|e| {
            eprintln!(
                "[transport:session] Failed to set session expiration: {:?}",
                e
            );
            TransportError::session(crate::error::SessionError::StorageFailed(
                "Failed to set session expiration".to_string(),
            ))
        })?;
    }

    // Store issued-at timestamp if available
    if let Some(iat) = identity.claims.issued_at {
        let iat_str = iat.to_string();
        session
            .set(
                session_keys::JWT_ISSUED_AT,
                &TypedValue::AsBytes(iat_str.as_bytes().to_vec()),
            )
            .map_err(|e| {
                eprintln!("[transport:session] Failed to store JWT issued-at: {:?}", e);
                TransportError::session(crate::error::SessionError::StorageFailed(
                    "Failed to store JWT issued-at".to_string(),
                ))
            })?;
    }

    Ok(())
}

/// Extract stored string value from session
///
/// Helper to reduce nested Option handling when retrieving string values from session storage.
fn get_stored_string(
    session: &crate::bindings::wasmcp::mcp_v20250618::sessions::Session,
    key: &str,
    field_name: &str,
) -> Result<String, TransportError> {
    use crate::bindings::wasmcp::keyvalue::store::TypedValue;

    let stored_value = session.get(key).map_err(|e| {
        eprintln!(
            "[transport:session] Failed to retrieve {} from session: {:?}",
            field_name, e
        );
        TransportError::session(crate::error::SessionError::StorageFailed(format!(
            "Failed to retrieve session {}",
            field_name
        )))
    })?;

    match stored_value {
        Some(TypedValue::AsBytes(bytes)) => String::from_utf8(bytes).map_err(|e| {
            eprintln!(
                "[transport:session] Invalid UTF-8 in stored {}: {:?}",
                field_name, e
            );
            TransportError::session(crate::error::SessionError::StorageFailed(
                "Session identity data corrupted".to_string(),
            ))
        }),
        Some(_) => {
            eprintln!(
                "[transport:session] {} has wrong type in storage",
                field_name
            );
            Err(TransportError::session(
                crate::error::SessionError::StorageFailed(
                    "Session identity data corrupted (wrong type)".to_string(),
                ),
            ))
        }
        None => {
            eprintln!("[transport:session] Session has no bound {}", field_name);
            Err(TransportError::session(
                crate::error::SessionError::IdentityMismatch(format!(
                    "Session has no bound {}",
                    field_name
                )),
            ))
        }
    }
}

/// Validate that the current JWT identity matches the session's bound identity
///
/// This prevents session hijacking where a valid JWT from User B could be used
/// to access User A's session by providing User A's session ID.
///
/// Compares:
/// - JWT subject (sub claim) with stored jwt:sub
/// - JWT issuer (iss claim) with stored jwt:iss
///
/// Returns:
/// - Ok(()) if identity matches session
/// - Err(TransportError::SessionIdentityMismatch) if identity doesn't match
pub fn validate_session_identity(
    session_id: &str,
    identity: &crate::bindings::wasmcp::mcp_v20250618::mcp::Identity,
    session_config: &TransportConfig,
) -> Result<(), TransportError> {
    use crate::bindings::wasmcp::auth::helpers;
    use crate::bindings::wasmcp::mcp_v20250618::sessions::Session;

    let bucket = session_config.get_session_bucket();

    // Open session resource
    let session = Session::open(session_id, bucket).map_err(|e| {
        eprintln!(
            "[transport:session] Failed to open session {} for identity validation: {:?}",
            session_id, e
        );
        TransportError::session(crate::error::SessionError::StorageFailed(
            "Failed to open session for identity validation".to_string(),
        ))
    })?;

    // Get current JWT subject and issuer
    let current_subject = helpers::get_subject(&identity.claims);
    let current_issuer = helpers::get_issuer(&identity.claims);

    // Retrieve and validate stored subject
    let stored_subject = get_stored_string(&session, session_keys::JWT_SUBJECT, "identity")?;

    // Compare subjects
    if current_subject != stored_subject {
        eprintln!("[transport:session] Session identity mismatch detected (subject)");
        return Err(TransportError::session(
            crate::error::SessionError::IdentityMismatch(
                "JWT identity does not match session".to_string(),
            ),
        ));
    }

    // Retrieve and compare issuer (if stored)
    if let Ok(Some(crate::bindings::wasmcp::keyvalue::store::TypedValue::AsBytes(bytes))) =
        session.get(session_keys::JWT_ISSUER)
        && let Ok(stored_issuer) = String::from_utf8(bytes)
    {
        // Only compare if current JWT has issuer
        if let Some(curr_iss) = current_issuer
            && curr_iss != stored_issuer
        {
            eprintln!("[transport:session] Session identity mismatch detected (issuer)");
            return Err(TransportError::session(
                crate::error::SessionError::IdentityMismatch(
                    "JWT identity does not match session".to_string(),
                ),
            ));
        }
    }

    Ok(())
}
