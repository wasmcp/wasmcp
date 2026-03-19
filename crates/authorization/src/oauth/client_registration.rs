//! OAuth Client ID Metadata Document support (RFC 7591)
//!
//! In MCP, the client_id is a URL. Fetching that URL returns the client's
//! metadata document, enabling servers to discover client configuration
//! without requiring out-of-band registration (SEP-991 / PR #1296).

use crate::bindings::exports::wasmcp::auth::client_registration::OauthClientMetadata;
use serde::Deserialize;

/// JSON representation of OAuth client metadata (RFC 7591 §2)
#[derive(Debug, Deserialize)]
struct ClientMetadataJson {
    #[serde(default)]
    redirect_uris: Vec<String>,
    #[serde(default)]
    client_name: Option<String>,
    #[serde(default)]
    client_uri: Option<String>,
    #[serde(default)]
    logo_uri: Option<String>,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    contacts: Vec<String>,
    #[serde(default)]
    tos_uri: Option<String>,
    #[serde(default)]
    policy_uri: Option<String>,
    #[serde(default)]
    jwks_uri: Option<String>,
    #[serde(default)]
    token_endpoint_auth_method: Option<String>,
    #[serde(default)]
    grant_types: Vec<String>,
    #[serde(default)]
    response_types: Vec<String>,
    #[serde(default)]
    software_id: Option<String>,
    #[serde(default)]
    software_version: Option<String>,
}

impl From<ClientMetadataJson> for OauthClientMetadata {
    fn from(j: ClientMetadataJson) -> Self {
        OauthClientMetadata {
            redirect_uris: j.redirect_uris,
            client_name: j.client_name,
            client_uri: j.client_uri,
            logo_uri: j.logo_uri,
            scope: j.scope,
            contacts: j.contacts,
            tos_uri: j.tos_uri,
            policy_uri: j.policy_uri,
            jwks_uri: j.jwks_uri,
            token_endpoint_auth_method: j.token_endpoint_auth_method,
            grant_types: j.grant_types,
            response_types: j.response_types,
            software_id: j.software_id,
            software_version: j.software_version,
        }
    }
}

/// Fetch client metadata from the client ID URL
///
/// Performs HTTP GET to `client_id_url` and parses the JSON response.
pub fn fetch_client_metadata(client_id_url: &str) -> Result<OauthClientMetadata, String> {
    let body = super::http::http_get(client_id_url)?;
    let json: ClientMetadataJson = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse client metadata: {}", e))?;
    Ok(json.into())
}

/// Validate client metadata against the expected client ID
///
/// Verifies:
/// - The client_id URL is HTTPS (required for security)
/// - The client_id has no fragment component
pub fn validate_client_metadata(
    _metadata: &OauthClientMetadata,
    expected_client_id: &str,
) -> Result<(), String> {
    if !expected_client_id.starts_with("https://") {
        return Err("Client ID must be an HTTPS URL".to_string());
    }

    if expected_client_id.contains('#') {
        return Err("Client ID must not contain a fragment".to_string());
    }

    Ok(())
}
