//! OAuth 2.1 Error Handling
//!
//! RFC-compliant error codes and response formatting.

use crate::bindings::exports::wasmcp::oauth::errors::{ErrorCode, OauthError};

/// Convert error code to RFC-standard string
pub fn error_code_to_string(code: &ErrorCode) -> String {
    match code {
        // Authorization errors
        ErrorCode::InvalidRequest => "invalid_request",
        ErrorCode::UnauthorizedClient => "unauthorized_client",
        ErrorCode::AccessDenied => "access_denied",
        ErrorCode::UnsupportedResponseType => "unsupported_response_type",
        ErrorCode::InvalidScope => "invalid_scope",
        ErrorCode::ServerError => "server_error",
        ErrorCode::TemporarilyUnavailable => "temporarily_unavailable",

        // Token errors
        ErrorCode::InvalidClient => "invalid_client",
        ErrorCode::InvalidGrant => "invalid_grant",
        ErrorCode::UnsupportedGrantType => "unsupported_grant_type",

        // Resource access errors
        ErrorCode::InvalidToken => "invalid_token",
        ErrorCode::InsufficientScope => "insufficient_scope",

        // Registration errors
        ErrorCode::InvalidRedirectUri => "invalid_redirect_uri",
        ErrorCode::InvalidClientMetadata => "invalid_client_metadata",
        ErrorCode::InvalidSoftwareStatement => "invalid_software_statement",
        ErrorCode::UnapprovedSoftwareStatement => "unapproved_software_statement",

        // Extension
        ErrorCode::Extension(s) => s,
    }
    .to_string()
}

/// Parse RFC-standard error string to error code
pub fn parse_error_code(error_string: &str) -> Option<ErrorCode> {
    match error_string {
        // Authorization errors
        "invalid_request" => Some(ErrorCode::InvalidRequest),
        "unauthorized_client" => Some(ErrorCode::UnauthorizedClient),
        "access_denied" => Some(ErrorCode::AccessDenied),
        "unsupported_response_type" => Some(ErrorCode::UnsupportedResponseType),
        "invalid_scope" => Some(ErrorCode::InvalidScope),
        "server_error" => Some(ErrorCode::ServerError),
        "temporarily_unavailable" => Some(ErrorCode::TemporarilyUnavailable),

        // Token errors
        "invalid_client" => Some(ErrorCode::InvalidClient),
        "invalid_grant" => Some(ErrorCode::InvalidGrant),
        "unsupported_grant_type" => Some(ErrorCode::UnsupportedGrantType),

        // Resource access errors
        "invalid_token" => Some(ErrorCode::InvalidToken),
        "insufficient_scope" => Some(ErrorCode::InsufficientScope),

        // Registration errors
        "invalid_redirect_uri" => Some(ErrorCode::InvalidRedirectUri),
        "invalid_client_metadata" => Some(ErrorCode::InvalidClientMetadata),
        "invalid_software_statement" => Some(ErrorCode::InvalidSoftwareStatement),
        "unapproved_software_statement" => Some(ErrorCode::UnapprovedSoftwareStatement),

        // Unknown - return None
        _ => None,
    }
}

/// Create WWW-Authenticate bearer challenge header value
pub fn create_bearer_challenge(
    realm: Option<String>,
    error: Option<ErrorCode>,
    error_description: Option<String>,
    scope: Vec<String>,
) -> String {
    let mut parts = vec!["Bearer".to_string()];
    let mut params = Vec::new();

    if let Some(realm) = realm {
        params.push(format!("realm=\"{}\"", realm));
    }

    if let Some(error) = error {
        let error_str = error_code_to_string(&error);
        params.push(format!("error=\"{}\"", error_str));
    }

    if let Some(desc) = error_description {
        // Escape quotes in description
        let escaped = desc.replace('\"', "\\\"");
        params.push(format!("error_description=\"{}\"", escaped));
    }

    if !scope.is_empty() {
        let scope_str = scope.join(" ");
        params.push(format!("scope=\"{}\"", scope_str));
    }

    if !params.is_empty() {
        parts.push(params.join(", "));
        parts.join(" ")
    } else {
        "Bearer".to_string()
    }
}

/// Create JSON error response for token endpoint
pub fn create_token_error_response(error: &OauthError) -> String {
    let mut json = serde_json::json!({
        "error": error_code_to_string(&error.error),
    });

    if let Some(ref desc) = error.error_description {
        json["error_description"] = serde_json::json!(desc);
    }

    if let Some(ref uri) = error.error_uri {
        json["error_uri"] = serde_json::json!(uri);
    }

    serde_json::to_string(&json).unwrap_or_else(|_| r#"{"error":"server_error"}"#.to_string())
}
