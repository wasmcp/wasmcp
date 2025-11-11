//! OAuth 2.0 Token Introspection (RFC 7662)

use crate::bindings::exports::wasmcp::oauth::errors::OauthError;
use crate::bindings::exports::wasmcp::oauth::introspection::{
    IntrospectionRequest, IntrospectionResponse,
};
use crate::bindings::wasi::http::outgoing_handler;
use crate::bindings::wasi::http::types::{Fields, Method, OutgoingBody, OutgoingRequest, Scheme};
use crate::bindings::wasi::io::poll;
use crate::bindings::wasi::io::streams::StreamError;
use crate::bindings::wasmcp::oauth::types::JwtClaims;
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Internal structure for parsing introspection JSON response
#[derive(Debug, Serialize, Deserialize)]
struct IntrospectionResponseJson {
    active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iat: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nbf: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aud: Option<serde_json::Value>, // Can be string or array
    #[serde(skip_serializing_if = "Option::is_none")]
    iss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jti: Option<String>,
    #[serde(flatten)]
    additional: serde_json::Map<String, serde_json::Value>,
}

/// Introspect an opaque token with authorization server (RFC 7662)
///
/// Makes HTTP POST request to introspection endpoint with Basic auth
pub fn introspect_token(
    introspection_endpoint: &str,
    request: &IntrospectionRequest,
    client_credentials: &(String, String),
) -> Result<IntrospectionResponse, OauthError> {
    use crate::bindings::exports::wasmcp::oauth::errors::ErrorCode;

    eprintln!(
        "[oauth-auth:introspection] Introspecting token at: {}",
        introspection_endpoint
    );

    // Parse URL
    let url = introspection_endpoint
        .parse::<url::Url>()
        .map_err(|e| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some(format!("Invalid introspection URL: {}", e)),
            error_uri: None,
        })?;

    let scheme = match url.scheme() {
        "https" => Scheme::Https,
        "http" => Scheme::Http,
        _ => {
            return Err(OauthError {
                error: ErrorCode::ServerError,
                error_description: Some("Invalid URL scheme".to_string()),
                error_uri: None,
            });
        }
    };

    let authority = url.host_str().ok_or_else(|| OauthError {
        error: ErrorCode::ServerError,
        error_description: Some("No host in URL".to_string()),
        error_uri: None,
    })?;

    let path = url.path().to_string();

    // Build form-encoded body
    let mut body = format!("token={}", urlencoding::encode(&request.token));
    if let Some(ref hint) = request.token_type_hint {
        body.push_str(&format!("&token_type_hint={}", urlencoding::encode(hint)));
    }
    let body_bytes = body.as_bytes().to_vec();

    // Create Basic auth header
    let (client_id, client_secret) = client_credentials;
    let credentials = format!("{}:{}", client_id, client_secret);
    let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
    let auth_value = format!("Basic {}", encoded);

    // Create headers
    let headers = Fields::new();
    headers
        .append(
            "Content-Type",
            b"application/x-www-form-urlencoded".as_ref(),
        )
        .map_err(|_| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some("Failed to set Content-Type header".to_string()),
            error_uri: None,
        })?;

    headers
        .append("Authorization", auth_value.as_bytes())
        .map_err(|_| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some("Failed to set Authorization header".to_string()),
            error_uri: None,
        })?;

    headers
        .append("Content-Length", body_bytes.len().to_string().as_bytes())
        .map_err(|_| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some("Failed to set Content-Length header".to_string()),
            error_uri: None,
        })?;

    headers
        .append("Accept", b"application/json".as_ref())
        .map_err(|_| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some("Failed to set Accept header".to_string()),
            error_uri: None,
        })?;

    // Create request
    let outgoing_request = OutgoingRequest::new(headers);
    outgoing_request
        .set_scheme(Some(&scheme))
        .map_err(|_| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some("Failed to set scheme".to_string()),
            error_uri: None,
        })?;
    outgoing_request
        .set_authority(Some(authority))
        .map_err(|_| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some("Failed to set authority".to_string()),
            error_uri: None,
        })?;
    outgoing_request
        .set_path_with_query(Some(&path))
        .map_err(|_| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some("Failed to set path".to_string()),
            error_uri: None,
        })?;
    outgoing_request
        .set_method(&Method::Post)
        .map_err(|_| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some("Failed to set method".to_string()),
            error_uri: None,
        })?;

    // Write body
    let outgoing_body = outgoing_request.body().map_err(|_| OauthError {
        error: ErrorCode::ServerError,
        error_description: Some("Failed to get request body".to_string()),
        error_uri: None,
    })?;

    let output_stream = outgoing_body.write().map_err(|_| OauthError {
        error: ErrorCode::ServerError,
        error_description: Some("Failed to get output stream".to_string()),
        error_uri: None,
    })?;

    output_stream
        .blocking_write_and_flush(&body_bytes)
        .map_err(|e| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some(format!("Failed to write body: {:?}", e)),
            error_uri: None,
        })?;

    drop(output_stream);
    OutgoingBody::finish(outgoing_body, None).map_err(|_| OauthError {
        error: ErrorCode::ServerError,
        error_description: Some("Failed to finish request body".to_string()),
        error_uri: None,
    })?;

    // Send request
    let future_response =
        outgoing_handler::handle(outgoing_request, None).map_err(|e| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some(format!("Failed to send request: {:?}", e)),
            error_uri: None,
        })?;

    // Poll for response
    let pollable = future_response.subscribe();
    poll::poll(&[&pollable]);
    drop(pollable);

    // Get response
    let incoming_response = future_response
        .get()
        .ok_or_else(|| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some("Response not ready".to_string()),
            error_uri: None,
        })?
        .map_err(|e| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some(format!("Request failed: {:?}", e)),
            error_uri: None,
        })?
        .map_err(|e| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some(format!("HTTP error: {:?}", e)),
            error_uri: None,
        })?;

    let status = incoming_response.status();

    // Read response body
    let incoming_body = incoming_response.consume().map_err(|_| OauthError {
        error: ErrorCode::ServerError,
        error_description: Some("Failed to get response body".to_string()),
        error_uri: None,
    })?;

    let input_stream = incoming_body.stream().map_err(|_| OauthError {
        error: ErrorCode::ServerError,
        error_description: Some("Failed to get response stream".to_string()),
        error_uri: None,
    })?;

    let mut body_bytes = Vec::new();
    loop {
        match input_stream.blocking_read(4096) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                body_bytes.extend_from_slice(&chunk);
            }
            Err(StreamError::Closed) => break,
            Err(e) => {
                return Err(OauthError {
                    error: ErrorCode::ServerError,
                    error_description: Some(format!("Failed to read response: {:?}", e)),
                    error_uri: None,
                });
            }
        }
    }

    // Parse response
    let body_str = String::from_utf8(body_bytes).map_err(|e| OauthError {
        error: ErrorCode::ServerError,
        error_description: Some(format!("Invalid UTF-8 in response: {}", e)),
        error_uri: None,
    })?;

    // Check for non-200 status
    if status != 200 {
        eprintln!(
            "[oauth-auth:introspection] Request failed with status {}: {}",
            status, body_str
        );

        return Err(OauthError {
            error: ErrorCode::ServerError,
            error_description: Some(format!(
                "Introspection failed with status {}: {}",
                status, body_str
            )),
            error_uri: None,
        });
    }

    // Parse JSON response
    let json_response: IntrospectionResponseJson =
        serde_json::from_str(&body_str).map_err(|e| OauthError {
            error: ErrorCode::ServerError,
            error_description: Some(format!("Failed to parse introspection response: {}", e)),
            error_uri: None,
        })?;

    eprintln!(
        "[oauth-auth:introspection] Token active: {}",
        json_response.active
    );

    // Convert to WIT response structure
    let scope = json_response.scope.map(|s| {
        s.split_whitespace()
            .map(String::from)
            .collect::<Vec<String>>()
    });

    let aud = json_response.aud.and_then(|a| match a {
        serde_json::Value::String(s) => Some(vec![s]),
        serde_json::Value::Array(arr) => Some(
            arr.into_iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
        ),
        _ => None,
    });

    // Convert additional claims to tuple list
    let additional_claims: Vec<(String, String)> = json_response
        .additional
        .into_iter()
        .map(|(k, v)| {
            let value_str = match v {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => "null".to_string(),
                _ => serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
            };
            (k, value_str)
        })
        .collect();

    Ok(IntrospectionResponse {
        active: json_response.active,
        scope,
        client_id: json_response.client_id,
        username: json_response.username,
        token_type: json_response.token_type,
        exp: json_response.exp,
        iat: json_response.iat,
        nbf: json_response.nbf,
        sub: json_response.sub,
        aud,
        iss: json_response.iss,
        jti: json_response.jti,
        additional_claims,
    })
}

/// Convert introspection response to jwt-claims
pub fn to_jwt_claims(response: &IntrospectionResponse) -> Option<JwtClaims> {
    // If token is not active, return None
    if !response.active {
        return None;
    }

    // Build jwt-claims from introspection response
    Some(JwtClaims {
        subject: response.sub.clone().unwrap_or_default(),
        issuer: response.iss.clone(),
        audience: response.aud.clone().unwrap_or_default(),
        expiration: response.exp,
        issued_at: response.iat,
        not_before: response.nbf,
        jwt_id: response.jti.clone(),
        scopes: response.scope.clone().unwrap_or_default(),
        confirmation: None, // Introspection doesn't include confirmation
        custom_claims: response.additional_claims.clone(),
    })
}
