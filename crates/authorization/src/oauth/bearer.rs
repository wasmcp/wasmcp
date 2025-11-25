//! OAuth 2.0 Bearer Token Extraction (RFC 6750)

use crate::bindings::exports::wasmcp::auth::bearer::BearerMethod;
use crate::bindings::exports::wasmcp::auth::errors::{ErrorCode, OauthError};

/// Extract bearer token from HTTP request
pub fn extract_bearer_token(
    headers: Vec<(String, String)>,
    body_params: Option<Vec<(String, String)>>,
    query_params: Option<Vec<(String, String)>>,
) -> Result<(String, BearerMethod), OauthError> {
    let mut token: Option<(String, BearerMethod)> = None;

    // Check Authorization header (RFC 6750 §2.1)
    for (key, value) in &headers {
        if key.to_lowercase() == "authorization" {
            // Parse "Bearer <token>" format
            if let Some(bearer_token) = value.strip_prefix("Bearer ") {
                if token.is_some() {
                    return Err(OauthError {
                        error: ErrorCode::InvalidRequest,
                        error_description: Some("Multiple bearer tokens presented".to_string()),
                        error_uri: None,
                    });
                }
                token = Some((bearer_token.trim().to_string(), BearerMethod::Header));
            }
        }
    }

    // Check body parameters (RFC 6750 §2.2)
    if let Some(params) = body_params {
        for (key, value) in params {
            if key == "access_token" {
                if token.is_some() {
                    return Err(OauthError {
                        error: ErrorCode::InvalidRequest,
                        error_description: Some("Multiple bearer tokens presented".to_string()),
                        error_uri: None,
                    });
                }
                token = Some((value, BearerMethod::Body));
            }
        }
    }

    // MCP spec line 251: Access tokens MUST NOT be included in URI query string
    // Reject requests with tokens in query parameters
    if let Some(params) = query_params {
        for (key, _) in params {
            if key == "access_token" {
                return Err(OauthError {
                    error: ErrorCode::InvalidRequest,
                    error_description: Some(
                        "Access tokens in query parameters are forbidden per MCP spec (line 251)"
                            .to_string(),
                    ),
                    error_uri: None,
                });
            }
        }
    }

    match token {
        Some((t, m)) => {
            // Validate token format
            if !is_valid_bearer_token_format(&t) {
                return Err(OauthError {
                    error: ErrorCode::InvalidRequest,
                    error_description: Some("Invalid bearer token format".to_string()),
                    error_uri: None,
                });
            }
            Ok((t, m))
        }
        None => Err(OauthError {
            error: ErrorCode::InvalidToken,
            error_description: Some("No bearer token found in request".to_string()),
            error_uri: None,
        }),
    }
}

/// Check if bearer method is allowed
pub fn is_method_allowed(method: &BearerMethod, allowed_methods: &[BearerMethod]) -> bool {
    allowed_methods.contains(method)
}

/// Validate bearer token format per RFC 6750
/// b64token = 1*( ALPHA / DIGIT / "-" / "." / "_" / "~" / "+" / "/" ) *"="
pub fn is_valid_bearer_token_format(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }

    // Split on '=' to handle base64 padding separately
    let (main, padding) = match token.rfind(|c| c != '=') {
        Some(pos) => (&token[..=pos], &token[pos + 1..]),
        None => return false, // All '=' is invalid
    };

    // Check main part contains valid b64token characters
    if !main
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_' | '~' | '+' | '/'))
    {
        return false;
    }

    // Check padding is only '=' characters
    padding.chars().all(|c| c == '=')
}
