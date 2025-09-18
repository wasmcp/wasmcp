/// HTTP-level authentication and discovery endpoints
use anyhow::Result;
use serde_json::json;
use spin_sdk::http::{Request, Response};

use super::types::{AuthContext, AuthResponse};
use super::{authorize};
use crate::bindings::wasmcp::mcp::jwt_validation::{get_validation_config};
use crate::bindings::wasmcp::mcp::jwks_cache::{get_jwks_uri};
use crate::error::{ErrorCode, ErrorCodeExt, McpError};

/// Authorize an HTTP request using the authorization component
pub async fn authorize_request(
    req: &Request,
) -> Result<AuthContext, McpError> {
    // Extract bearer token from Authorization header
    let token = req
        .header("Authorization")
        .and_then(|auth_header| auth_header.as_str().and_then(|auth| auth.strip_prefix("Bearer ")))
        .ok_or_else(|| McpError {
            code: ErrorCode::Unauthorized,
            message: "Missing or invalid Authorization header".to_string(),
            data: None,
        })?;

    // Collect request headers
    let headers: Vec<(String, String)> = req
        .headers()
        .map(|(name, value)| (name.to_string(), value.as_str().unwrap_or("").to_string()))
        .collect();

    let validation_config = get_validation_config();
    let jwks_uri = get_jwks_uri();

    // Validate that the provider gave us valid auth config
    if validation_config.issuer.is_empty() {
        return Err(McpError {
            code: ErrorCode::InternalError,
            message: "Provider returned invalid auth config: issuer cannot be empty"
                .to_string(),
            data: None,
        });
    }
    if validation_config.audience.is_empty() {
        return Err(McpError {
            code: ErrorCode::InternalError,
            message: "Provider returned invalid auth config: audience cannot be empty"
                .to_string(),
            data: None,
        });
    }
    if jwks_uri.is_empty() {
        return Err(McpError {
            code: ErrorCode::InternalError,
            message: "Provider returned invalid auth config: jwks_uri cannot be empty".to_string(),
            data: None,
        });
    }

    // // Build authorization request with provider's required configuration
    // let auth_request = AuthRequest {
    //     token: token.to_string(),
    //     method: req.method().to_string(),
    //     path: req.uri().to_string(),
    //     headers,
    //     body: Some(req.body().to_vec()),
    //     expected_issuer: validation_config.issuer.clone(),
    //     expected_audiences: validation_config.audience.clone(),
    //     expected_subject: validation_config.subject.clone(),
    //     jwks_uri: jwks_uri.clone(),
    //     policy: validation_config.policy.clone(),
    //     policy_data: validation_config.policy_data.clone(),
    //     pass_jwt: validation_config.pass_jwt,
    // };

    // Call the internal authorization function
    match authorize(token.to_string(), validation_config) {
        AuthResponse::Authorized(context) => Ok(context),
        AuthResponse::Unauthorized(error) => Err(McpError {
            code: if error.status == 403 {
                ErrorCode::Unauthorized
            } else {
                ErrorCode::InvalidRequest
            },
            message: error.description,
            data: error.www_authenticate,
        }),
    }
}

/// Create an HTTP error response for auth failures
pub fn create_auth_error_response(error: McpError) -> Response {
    let status = if error.code.to_code() == -32005 {
        401
    } else {
        403
    };
    let body = json!({
        "jsonrpc": "2.0",
        "error": {
            "code": error.code.to_code(),
            "message": error.message,
            "data": error.data
        }
    });

    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body.to_string())
        .build()
}

