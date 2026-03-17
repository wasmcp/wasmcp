//! OAuth 2.0 Authorization Server Metadata (RFC 8414)
//! with OpenID Connect Discovery 1.0 support

use crate::bindings::exports::wasmcp::auth::authorization_server_discovery::AuthorizationServerMetadata;
use serde::Deserialize;

/// JSON representation of authorization server metadata
/// Covers both RFC 8414 and OpenID Connect Discovery 1.0 fields.
#[derive(Debug, Deserialize)]
struct AsMetadataJson {
    issuer: String,
    #[serde(default)]
    authorization_endpoint: Option<String>,
    #[serde(default)]
    token_endpoint: Option<String>,
    #[serde(default)]
    jwks_uri: Option<String>,
    #[serde(default)]
    registration_endpoint: Option<String>,
    #[serde(default)]
    scopes_supported: Vec<String>,
    #[serde(default)]
    response_types_supported: Vec<String>,
    #[serde(default)]
    grant_types_supported: Vec<String>,
    #[serde(default)]
    token_endpoint_auth_methods_supported: Vec<String>,
    #[serde(default)]
    revocation_endpoint: Option<String>,
    #[serde(default)]
    introspection_endpoint: Option<String>,
    #[serde(default)]
    code_challenge_methods_supported: Vec<String>,
    #[serde(default)]
    pushed_authorization_request_endpoint: Option<String>,
    // OpenID Connect Discovery 1.0 fields
    #[serde(default)]
    userinfo_endpoint: Option<String>,
    #[serde(default)]
    subject_types_supported: Vec<String>,
    #[serde(default)]
    id_token_signing_alg_values_supported: Vec<String>,
}

impl From<AsMetadataJson> for AuthorizationServerMetadata {
    fn from(j: AsMetadataJson) -> Self {
        AuthorizationServerMetadata {
            issuer: j.issuer,
            authorization_endpoint: j.authorization_endpoint,
            token_endpoint: j.token_endpoint,
            jwks_uri: j.jwks_uri,
            registration_endpoint: j.registration_endpoint,
            scopes_supported: j.scopes_supported,
            response_types_supported: j.response_types_supported,
            grant_types_supported: j.grant_types_supported,
            token_endpoint_auth_methods_supported: j.token_endpoint_auth_methods_supported,
            revocation_endpoint: j.revocation_endpoint,
            introspection_endpoint: j.introspection_endpoint,
            code_challenge_methods_supported: j.code_challenge_methods_supported,
            pushed_authorization_request_endpoint: j.pushed_authorization_request_endpoint,
            userinfo_endpoint: j.userinfo_endpoint,
            subject_types_supported: j.subject_types_supported,
            id_token_signing_alg_values_supported: j.id_token_signing_alg_values_supported,
        }
    }
}

fn normalize_issuer(issuer: &str) -> String {
    issuer.trim_end_matches('/').to_string()
}

fn fetch_and_parse(url: &str) -> Result<AuthorizationServerMetadata, String> {
    let body = super::http::http_get(url)?;
    let json: AsMetadataJson =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse metadata: {}", e))?;
    Ok(json.into())
}

/// Fetch OAuth 2.0 Authorization Server Metadata (RFC 8414)
///
/// Fetches from `{issuer}/.well-known/oauth-authorization-server`
pub fn fetch_authorization_server_metadata(
    issuer: &str,
) -> Result<AuthorizationServerMetadata, String> {
    let base = normalize_issuer(issuer);
    let url = format!("{}/.well-known/oauth-authorization-server", base);
    fetch_and_parse(&url)
}

/// Fetch OpenID Connect Discovery document
///
/// Fetches from `{issuer}/.well-known/openid-configuration`
pub fn fetch_openid_configuration(issuer: &str) -> Result<AuthorizationServerMetadata, String> {
    let base = normalize_issuer(issuer);
    let url = format!("{}/.well-known/openid-configuration", base);
    fetch_and_parse(&url)
}

/// Validate authorization server metadata (RFC 8414 §3.3)
///
/// Verifies:
/// - `issuer` matches the expected value (prevents metadata mix-up attacks)
/// - `issuer` is an HTTPS URL with no query or fragment
pub fn validate_metadata(
    metadata: &AuthorizationServerMetadata,
    expected_issuer: &str,
) -> Result<(), String> {
    let normalized = normalize_issuer(expected_issuer);

    if metadata.issuer != normalized {
        return Err(format!(
            "Issuer mismatch: expected '{}', got '{}'",
            normalized, metadata.issuer
        ));
    }

    if !metadata.issuer.starts_with("https://") {
        return Err("Issuer must be an HTTPS URL".to_string());
    }

    if metadata.issuer.contains('?') || metadata.issuer.contains('#') {
        return Err("Issuer must not contain query or fragment components".to_string());
    }

    Ok(())
}
