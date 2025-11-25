//! OAuth 2.0 Protected Resource Metadata (RFC 9728)

use crate::bindings::exports::wasmcp::auth::resource_metadata::ProtectedResourceMetadata;

/// Fetch resource metadata from well-known location
/// TODO: Implement HTTP GET to /.well-known/oauth-protected-resource
pub fn fetch_metadata(_resource_url: &str) -> Result<ProtectedResourceMetadata, String> {
    // This would make an HTTP GET to:
    // {resource_url}/.well-known/oauth-protected-resource
    Err("Resource metadata fetching not yet implemented".to_string())
}

/// Validate resource metadata
pub fn validate_metadata(
    metadata: &ProtectedResourceMetadata,
    expected_resource: &str,
) -> Result<(), String> {
    // Check resource identifier matches
    if metadata.resource != expected_resource {
        return Err(format!(
            "Resource identifier mismatch: expected '{}', got '{}'",
            expected_resource, metadata.resource
        ));
    }

    // Check resource is HTTPS
    if !metadata.resource.starts_with("https://") {
        return Err("Resource identifier must be HTTPS URL".to_string());
    }

    // Check no fragment
    if metadata.resource.contains('#') {
        return Err("Resource identifier must not contain fragment".to_string());
    }

    Ok(())
}

/// Parse WWW-Authenticate header for resource metadata URL
pub fn parse_www_authenticate_metadata(www_authenticate_header: &str) -> Option<String> {
    // Parse Bearer challenge parameters
    // Format: Bearer realm="...", resource_metadata="https://...", error="..."

    // Simple parser for resource_metadata parameter
    for param in www_authenticate_header.split(',') {
        let param = param.trim();
        if param.starts_with("resource_metadata") {
            // Extract value from: resource_metadata="value"
            if let Some(start) = param.find('"')
                && let Some(end) = param[start + 1..].find('"')
            {
                return Some(param[start + 1..start + 1 + end].to_string());
            }
        }
    }

    None
}
