//! Authentication implementation for OTLP export.
//!
//! # Security Note: WIT Credential Storage Limitation
//!
//! **The WIT interface defines credentials as plain `string` types**, which means:
//! - Credentials arrive as plain Rust `String` from wit-bindgen generated code
//! - We cannot zeroize the source `AuthConfig` fields (wit-bindgen manages that memory)
//! - The WIT Component Model has no concept of "secure strings" or memory zeroization
//!
//! **What we CAN do (and do in this implementation):**
//! - Zeroize temporary copies of credentials created during authentication
//! - Minimize credential lifetime in intermediate strings
//! - Clear sensitive data from stack/heap as soon as possible
//!
//! **What we CANNOT do:**
//! - Zeroize the original `ExportConfig.authentication` fields (WIT limitation)
//! - Prevent credentials from existing in component memory (required by WIT API)
//!
//! **Mitigation:**
//! - WASM components run in sandboxed, isolated memory spaces
//! - Memory dumps are significantly harder to exploit than native code
//! - Host systems should manage credentials securely before passing to components

use crate::bindings::wasi::http::types::Fields;
use crate::bindings::exports::wasi::otel_sdk::http_transport::{
    AuthConfig, BasicAuthConfig, BearerTokenConfig, HeaderPair,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use zeroize::Zeroize;

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
///
/// # Security: Credential Zeroization
///
/// This function creates temporary copies of credentials for HTTP header construction.
/// We zeroize these temporary strings to minimize credential lifetime in memory:
/// - `credentials`: plaintext "username:password" (zeroized immediately after encoding)
/// - `encoded`: base64-encoded credentials (zeroized after header construction)
/// - `auth_value`: "Basic {encoded}" (zeroized after header set)
///
/// Note: The source `config.username` and `config.password` remain in memory
/// as they are managed by WIT-generated code (see module documentation).
fn apply_basic_auth(config: &BasicAuthConfig, headers: &Fields) -> Result<(), String> {
    // Create credentials string "username:password"
    let mut credentials = format!("{}:{}", config.username, config.password);

    // Base64 encode the credentials
    let mut encoded = BASE64.encode(credentials.as_bytes());

    // Zeroize plaintext credentials immediately after encoding
    credentials.zeroize();

    // Create Authorization header value
    let mut auth_value = format!("Basic {}", encoded);

    // Zeroize base64-encoded credentials after constructing header
    encoded.zeroize();

    // Set the Authorization header
    let result = headers.set(
        &"authorization".to_string(),
        &[auth_value.as_bytes().to_vec()]
    ).map_err(|_| "Failed to set Authorization header".to_string());

    // Zeroize the auth header value after use
    auth_value.zeroize();

    result
}

/// Apply Bearer token authentication
///
/// # Security: Credential Zeroization
///
/// This function creates a temporary copy of the bearer token for HTTP header construction.
/// We zeroize the temporary `auth_value` string to minimize token lifetime in memory.
///
/// Note: The source `config.token` remains in memory as it is managed by
/// WIT-generated code (see module documentation).
fn apply_bearer_auth(config: &BearerTokenConfig, headers: &Fields) -> Result<(), String> {
    // Create Authorization header value
    let mut auth_value = format!("Bearer {}", config.token);

    // Set the Authorization header
    let result = headers.set(
        &"authorization".to_string(),
        &[auth_value.as_bytes().to_vec()]
    ).map_err(|_| "Failed to set Authorization header".to_string());

    // Zeroize the auth header value after use
    auth_value.zeroize();

    result
}

/// Apply custom headers for authentication
///
/// # Security: Credential Zeroization
///
/// Custom headers may contain sensitive data (API keys, tokens, etc.).
/// We zeroize temporary copies of header values to minimize their lifetime in memory.
///
/// Note: The source `header.key` and `header.value` remain in memory as they are
/// managed by WIT-generated code (see module documentation).
fn apply_custom_headers(custom_headers: &[HeaderPair], headers: &Fields) -> Result<(), String> {
    for header in custom_headers {
        // Convert header key to lowercase (HTTP headers are case-insensitive)
        let key = header.key.to_lowercase();

        // Create a mutable copy of the value for zeroization
        let mut value_bytes = header.value.as_bytes().to_vec();

        // Set the header
        let result = headers.set(
            &key,
            &[value_bytes.clone()]
        ).map_err(|_| format!("Failed to set custom header: {}", header.key));

        // Zeroize the value bytes after use
        value_bytes.zeroize();

        result?;
    }

    Ok(())
}
