/// HTTP-level authentication and discovery endpoints
use anyhow::Result;
use serde_json::json;
use spin_sdk::http::{Request, Response};

use super::types::{AuthContext, AuthRequest, AuthResponse};
use super::{authorize, get_resource_metadata, get_server_metadata};
use crate::bindings::wasmcp::mcp::authorization_types::ProviderAuthConfig;
use crate::error::{ErrorCode, ErrorCodeExt, McpError};

/// Authorize an HTTP request using the authorization component
pub async fn authorize_request(
    req: &Request,
    provider_config: &ProviderAuthConfig,
) -> Result<AuthContext, McpError> {
    // Extract bearer token from Authorization header
    let token = req
        .headers()
        .find(|(name, _)| name.eq_ignore_ascii_case("authorization"))
        .and_then(|(_, value)| value.as_str())
        .and_then(|auth| auth.strip_prefix("Bearer "))
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

    // Validate that the provider gave us valid auth config
    if provider_config.expected_issuer.is_empty() {
        return Err(McpError {
            code: ErrorCode::InternalError,
            message: "Provider returned invalid auth config: expected_issuer cannot be empty"
                .to_string(),
            data: None,
        });
    }
    if provider_config.expected_audiences.is_empty() {
        return Err(McpError {
            code: ErrorCode::InternalError,
            message: "Provider returned invalid auth config: expected_audiences cannot be empty"
                .to_string(),
            data: None,
        });
    }
    if provider_config.jwks_uri.is_empty() {
        return Err(McpError {
            code: ErrorCode::InternalError,
            message: "Provider returned invalid auth config: jwks_uri cannot be empty".to_string(),
            data: None,
        });
    }

    // Build authorization request with provider's required configuration
    let auth_request = AuthRequest {
        token: token.to_string(),
        method: req.method().to_string(),
        path: req.uri().to_string(),
        headers,
        body: Some(req.body().to_vec()),
        expected_issuer: provider_config.expected_issuer.clone(),
        expected_audiences: provider_config.expected_audiences.clone(),
        expected_subject: provider_config.expected_subject.clone(),
        jwks_uri: provider_config.jwks_uri.clone(),
        policy: provider_config.policy.clone(),
        policy_data: provider_config.policy_data.clone(),
        pass_jwt: provider_config.pass_jwt,
    };

    // Call the internal authorization function
    match authorize(auth_request) {
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

/// Handle OAuth resource metadata discovery endpoint
pub fn handle_resource_metadata(
    request_uri: &str,
    provider_config: &ProviderAuthConfig,
) -> Result<Response> {
    // Build the server URL from the request
    let server_url = if request_uri.contains("://") {
        request_uri
            .split_once("/.well-known")
            .map(|(base, _)| base.to_string())
            .unwrap_or_else(|| "http://localhost:8080".to_string())
    } else {
        "http://localhost:8080".to_string()
    };

    let metadata = get_resource_metadata(provider_config, &server_url);
    let json = json!({
        "resource": metadata.resource_url,
        "authorization_servers": metadata.authorization_servers,
        "scopes_supported": metadata.scopes_supported,
        "bearer_methods_supported": metadata.bearer_methods_supported,
        "resource_documentation": metadata.resource_documentation,
    });

    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(json.to_string())
        .build())
}

/// Handle OAuth server metadata discovery endpoint
pub fn handle_server_metadata(provider_config: &ProviderAuthConfig) -> Result<Response> {
    let metadata = get_server_metadata(provider_config);
    let json = json!({
        "issuer": metadata.issuer,
        "authorization_endpoint": metadata.authorization_endpoint,
        "token_endpoint": metadata.token_endpoint,
        "jwks_uri": metadata.jwks_uri,
        "response_types_supported": metadata.response_types_supported,
        "grant_types_supported": metadata.grant_types_supported,
        "code_challenge_methods_supported": metadata.code_challenge_methods_supported,
        "scopes_supported": metadata.scopes_supported,
        "token_endpoint_auth_methods_supported": metadata.token_endpoint_auth_methods_supported,
        "service_documentation": metadata.service_documentation,
        "registration_endpoint": metadata.registration_endpoint,
    });

    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(json.to_string())
        .build())
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

