//! Authentication implementation for OTLP export.

use crate::bindings::wasi::http::types::Fields;
use crate::bindings::exports::wasi::otel_sdk::otel_export::{
    AuthConfig, BasicAuthConfig, BearerTokenConfig, HeaderPair,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Apply authentication to HTTP headers
pub fn apply_authentication(auth_config: &AuthConfig, headers: &Fields) -> Result<(), String> {
    match auth_config {
        AuthConfig::None => Ok(()),

        AuthConfig::Basic(config) => apply_basic_auth(config, headers),

        AuthConfig::Bearer(config) => apply_bearer_auth(config, headers),


        AuthConfig::Headers(custom_headers) => apply_custom_headers(custom_headers, headers),
    }
}

/// Apply HTTP Basic authentication
fn apply_basic_auth(config: &BasicAuthConfig, headers: &Fields) -> Result<(), String> {
    // Create credentials string "username:password"
    let credentials = format!("{}:{}", config.username, config.password);

    // Base64 encode the credentials
    let encoded = BASE64.encode(credentials.as_bytes());

    // Create Authorization header value
    let auth_value = format!("Basic {}", encoded);

    // Set the Authorization header
    headers.set(
        &"authorization".to_string(),
        &[auth_value.as_bytes().to_vec()]
    ).map_err(|_| "Failed to set Authorization header".to_string())?;

    Ok(())
}

/// Apply Bearer token authentication
fn apply_bearer_auth(config: &BearerTokenConfig, headers: &Fields) -> Result<(), String> {
    // Create Authorization header value
    let auth_value = format!("Bearer {}", config.token);

    // Set the Authorization header
    headers.set(
        &"authorization".to_string(),
        &[auth_value.as_bytes().to_vec()]
    ).map_err(|_| "Failed to set Authorization header".to_string())?;

    Ok(())
}

/// Apply custom headers for authentication
fn apply_custom_headers(custom_headers: &[HeaderPair], headers: &Fields) -> Result<(), String> {
    for header in custom_headers {
        // Convert header key to lowercase (HTTP headers are case-insensitive)
        let key = header.key.to_lowercase();

        // Set the header
        headers.set(
            &key,
            &[header.value.as_bytes().to_vec()]
        ).map_err(|_| format!("Failed to set custom header: {}", header.key))?;
    }

    Ok(())
}
