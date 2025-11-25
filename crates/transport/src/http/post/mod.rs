//! POST request handler
//!
//! POST is the primary MCP transport method. This module handles:
//! - Header validation (Accept, session, auth)
//! - Session validation and management
//! - JWT authentication (with graceful degradation)
//! - Request body stream acquisition
//! - MCP message parsing
//! - Message type routing (Request, Notification, Result, Error)
//! - Delegation to mode-specific handlers (JSON vs SSE)

pub mod initialize;
pub mod json_mode;
pub mod message_handlers;
pub mod sse_mode;

use crate::bindings::wasi::http::types::{IncomingRequest, ResponseOutparam};
use crate::bindings::wasmcp::mcp_v20250618::mcp::ClientRequest;
use crate::common;
use crate::config::{AuthMode, TransportConfig};
use crate::error::TransportError;
use crate::http::{session, validation};
use crate::send_error;

pub async fn handle_post(
    request: IncomingRequest,
    protocol_version: String,
    response_out: ResponseOutparam,
    session_config: &TransportConfig,
) {
    // Validate Accept header per spec
    if let Err(e) = validation::validate_accept_header(&request) {
        send_error!(response_out, e);
    }

    // Validate session from request headers
    let session_id = match session::validate_session_from_request(&request, session_config) {
        Ok(id) => id,
        Err(e) => send_error!(response_out, e),
    };

    // Validate JWT based on auth mode from config
    let identity = match session_config.auth_mode {
        AuthMode::Public => {
            // Public mode - no authentication required
            None
        }
        AuthMode::OAuth => {
            // OAuth mode - JWT required
            // Validate that JWT is configured properly
            if !session_config.jwt_configured {
                let error = TransportError::internal(
                    "WASMCP_AUTH_MODE=oauth requires JWT_PUBLIC_KEY or JWT_JWKS_URI to be configured",
                );
                send_error!(response_out, error);
            }

            match validation::extract_authorization_header(&request) {
                Ok(Some(jwt)) => {
                    // Import server-auth for JWT validation
                    use crate::bindings::wasmcp::mcp_v20250618::server_auth;

                    match server_auth::decode(&jwt) {
                        Ok(claims) => Some(crate::bindings::wasmcp::mcp_v20250618::mcp::Identity {
                            jwt,
                            claims,
                        }),
                        Err(e) => {
                            // Strict validation: return 401 with WWW-Authenticate header
                            let www_authenticate = create_www_authenticate_challenge(
                                &request,
                                "invalid_token",
                                "The access token provided is invalid, expired, or malformed",
                            );

                            let error = TransportError::unauthorized_with_challenge(
                                format!("JWT validation failed: {:?}", e),
                                www_authenticate,
                            );

                            send_error!(response_out, error);
                        }
                    }
                }
                Ok(None) => {
                    // OAuth mode requires token
                    let www_authenticate = create_www_authenticate_challenge(
                        &request,
                        "invalid_request",
                        "Authorization header with Bearer token is required",
                    );

                    let error = TransportError::unauthorized_with_challenge(
                        "Missing required Authorization header",
                        www_authenticate,
                    );

                    send_error!(response_out, error);
                }
                Err(e) => {
                    // Malformed header - always error
                    let www_authenticate = create_www_authenticate_challenge(
                        &request,
                        "invalid_request",
                        "Malformed Authorization header",
                    );

                    let error = TransportError::unauthorized_with_challenge(
                        format!("Invalid Authorization header: {}", e),
                        www_authenticate,
                    );

                    send_error!(response_out, error);
                }
            }
        }
    };

    // Get request body stream
    let body_stream = match request.consume() {
        Ok(s) => s,
        Err(_) => {
            let error = TransportError::internal("Failed to consume request");
            send_error!(response_out, error);
        }
    };
    let input_stream = match body_stream.stream() {
        Ok(s) => s,
        Err(_) => {
            let error = TransportError::internal("Failed to get input stream");
            send_error!(response_out, error);
        }
    };

    // Parse MCP message
    let message = match common::parse_mcp_message(
        &input_stream,
        common::http_read_limit(),
        &common::plain_json_frame(),
    ) {
        Ok(m) => m,
        Err(e) => {
            let error = TransportError::protocol(e);
            send_error!(response_out, error);
        }
    };

    // Build HTTP context to pass to downstream components
    let http_context = build_http_context(&request);

    match message {
        common::McpMessage::Request(request_id, client_request) => {
            // Check if it's initialize - handle specially with plain JSON
            if matches!(client_request, ClientRequest::Initialize(_)) {
                drop(input_stream);
                drop(body_stream);
                initialize::handle_initialize_request(
                    request_id,
                    client_request,
                    protocol_version,
                    identity.as_ref(),
                    response_out,
                    session_config,
                );
                return;
            }

            // Not initialize - check if session is required
            if !session::check_session_required(session_config, session_id.as_deref()) {
                drop(input_stream);
                drop(body_stream);
                let error = TransportError::session_required();
                send_error!(response_out, error);
            }

            // Not initialize - delegate to mode-specific handler
            if session_config.disable_sse {
                json_mode::handle_json_mode(
                    request_id,
                    client_request,
                    protocol_version,
                    session_id.as_deref(),
                    identity.as_ref(),
                    input_stream,
                    body_stream,
                    response_out,
                    session_config,
                    Some(http_context.clone()),
                )
            } else {
                sse_mode::handle_sse_streaming_mode(
                    request_id,
                    client_request,
                    protocol_version,
                    session_id.as_deref(),
                    identity.as_ref(),
                    input_stream,
                    body_stream,
                    response_out,
                    session_config,
                    Some(http_context.clone()),
                )
                .await
            }
        }
        common::McpMessage::Notification(client_notification) => {
            let result = message_handlers::handle_mcp_notification(
                client_notification,
                protocol_version,
                session_id.as_deref(),
                session_config,
                Some(http_context.clone()),
            );
            drop(input_stream);
            drop(body_stream);
            message_handlers::respond_with_result(result, response_out);
        }
        common::McpMessage::Result(result_id, client_result) => {
            let result = message_handlers::handle_mcp_result(
                result_id,
                client_result,
                protocol_version,
                session_id.as_deref(),
                session_config,
            );
            drop(input_stream);
            drop(body_stream);
            message_handlers::respond_with_result(result, response_out);
        }
        common::McpMessage::Error(error_id, error_code) => {
            let result = message_handlers::handle_mcp_error(
                error_id,
                error_code,
                protocol_version,
                session_id.as_deref(),
                session_config,
            );
            drop(input_stream);
            drop(body_stream);
            message_handlers::respond_with_result(result, response_out);
        }
    }
}

/// Build HTTP context for authorization
fn build_http_context(
    request: &IncomingRequest,
) -> crate::bindings::wasmcp::mcp_v20250618::server_auth::HttpContext {
    // Extract HTTP method
    let method = match request.method() {
        crate::bindings::wasi::http::types::Method::Get => "GET",
        crate::bindings::wasi::http::types::Method::Post => "POST",
        crate::bindings::wasi::http::types::Method::Put => "PUT",
        crate::bindings::wasi::http::types::Method::Delete => "DELETE",
        crate::bindings::wasi::http::types::Method::Head => "HEAD",
        crate::bindings::wasi::http::types::Method::Options => "OPTIONS",
        crate::bindings::wasi::http::types::Method::Patch => "PATCH",
        crate::bindings::wasi::http::types::Method::Connect => "CONNECT",
        crate::bindings::wasi::http::types::Method::Trace => "TRACE",
        _ => "UNKNOWN",
    }
    .to_string();

    // Extract path from request
    let path = request.path_with_query().unwrap_or("/".to_string());

    // Extract headers
    let headers_obj = request.headers();
    let mut headers = Vec::new();

    // Common headers to include for policy decisions
    let header_names = [
        "host",
        "user-agent",
        "origin",
        "referer",
        "x-forwarded-for",
        "x-real-ip",
    ];

    for name in &header_names {
        let values = headers_obj.get(name);
        if !values.is_empty()
            && let Ok(value) = String::from_utf8(values[0].clone())
        {
            headers.push((name.to_string(), value));
        }
    }

    crate::bindings::wasmcp::mcp_v20250618::server_auth::HttpContext {
        method,
        path,
        headers,
    }
}

/// Create WWW-Authenticate challenge header per RFC 6750
/// Includes error code, description, and resource metadata URL
fn create_www_authenticate_challenge(
    request: &IncomingRequest,
    error: &str,
    error_description: &str,
) -> String {
    use crate::bindings::wasi::cli::environment::get_environment;

    // Get server URI for resource metadata
    let env_vars = get_environment();

    // First check env var
    let server_uri: Option<String> = env_vars
        .iter()
        .find(|(k, _)| k == "WASMCP_SERVER_URI")
        .map(|(_, v)| {
            eprintln!(
                "[transport:www-authenticate] Using WASMCP_SERVER_URI from env: {}",
                v
            );
            v.clone()
        })
        .or_else(|| {
            // Try to construct from request Host header
            let headers = request.headers();
            let host_values = headers.get("host");
            if !host_values.is_empty() {
                if let Ok(host) = String::from_utf8(host_values[0].clone()) {
                    // Use scheme from request, default to https if not available
                    let scheme = request
                        .scheme()
                        .and_then(|s| match s {
                            crate::bindings::wasi::http::types::Scheme::Http => Some("http"),
                            crate::bindings::wasi::http::types::Scheme::Https => Some("https"),
                            _ => None,
                        })
                        .unwrap_or("https");
                    Some(format!("{}://{}", scheme, host))
                } else {
                    None
                }
            } else {
                None
            }
        });

    // Build WWW-Authenticate header
    let mut parts = vec!["Bearer".to_string()];

    if let Some(ref uri) = server_uri {
        parts.push(format!("realm=\"{}\"", uri));

        // Add resource_metadata URL per RFC 9728
        parts.push(format!(
            "resource_metadata=\"{}/.well-known/oauth-protected-resource\"",
            uri
        ));
    }

    parts.push(format!("error=\"{}\"", error));
    parts.push(format!("error_description=\"{}\"", error_description));

    parts.join(", ")
}
