//! Authentication implementation for OTLP export.

use crate::bindings::wasi::http::types::Fields;
use crate::bindings::exports::wasi::otel_sdk::otel_export::{
    AuthConfig, BasicAuthConfig, BearerTokenConfig, HeaderPair, Oauth2Config, OidcConfig,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

/// Apply authentication to HTTP headers
pub fn apply_authentication(auth_config: &AuthConfig, headers: &Fields) -> Result<(), String> {
    match auth_config {
        AuthConfig::None => Ok(()),

        AuthConfig::Basic(config) => apply_basic_auth(config, headers),

        AuthConfig::Bearer(config) => apply_bearer_auth(config, headers),

        AuthConfig::Oauth2(config) => apply_oauth2(config, headers),

        AuthConfig::Oidc(config) => apply_oidc(config, headers),

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

/// Apply OAuth2 client credentials authentication
fn apply_oauth2(config: &Oauth2Config, headers: &Fields) -> Result<(), String> {
    use crate::bindings::wasi::http::outgoing_handler;
    use crate::bindings::wasi::http::types::{Method, OutgoingBody, OutgoingRequest};

    // Parse token URL
    let (scheme, authority, path) = parse_token_url(&config.token_url)?;

    // Create form data for client credentials grant
    let mut form_data = format!(
        "grant_type=client_credentials&client_id={}&client_secret={}",
        urlencoding::encode(&config.client_id),
        urlencoding::encode(&config.client_secret)
    );

    // Add scopes if provided
    if !config.scopes.is_empty() {
        form_data.push_str("&scope=");
        form_data.push_str(&urlencoding::encode(&config.scopes.join(" ")));
    }

    // Create request headers for token request
    let token_headers = Fields::new();
    token_headers.set(
        &"content-type".to_string(),
        &[b"application/x-www-form-urlencoded".to_vec()]
    ).map_err(|_| "Failed to set content-type for OAuth2 token request".to_string())?;

    let content_length = form_data.len().to_string();
    token_headers.set(
        &"content-length".to_string(),
        &[content_length.as_bytes().to_vec()]
    ).map_err(|_| "Failed to set content-length for OAuth2 token request".to_string())?;

    // Create token request
    let request = OutgoingRequest::new(token_headers);
    request.set_method(&Method::Post)
        .map_err(|_| "Failed to set method for OAuth2 token request".to_string())?;
    request.set_scheme(Some(&scheme))
        .map_err(|_| "Failed to set scheme for OAuth2 token request".to_string())?;
    request.set_authority(Some(&authority))
        .map_err(|_| "Failed to set authority for OAuth2 token request".to_string())?;
    request.set_path_with_query(Some(&path))
        .map_err(|_| "Failed to set path for OAuth2 token request".to_string())?;

    // Write request body
    let body = request.body()
        .map_err(|_| "Failed to get OAuth2 token request body".to_string())?;
    let stream = body.write()
        .map_err(|_| "Failed to get OAuth2 token request stream".to_string())?;
    stream.blocking_write_and_flush(form_data.as_bytes())
        .map_err(|_| "Failed to write OAuth2 token request body".to_string())?;
    drop(stream);
    OutgoingBody::finish(body, None)
        .map_err(|_| "Failed to finish OAuth2 token request body".to_string())?;

    // Send token request
    let incoming_response = outgoing_handler::handle(request, None)
        .map_err(|_| "Failed to send OAuth2 token request".to_string())?;

    // Wait for response
    incoming_response.subscribe().block();

    let response = match incoming_response.get() {
        Some(Ok(resp)) => resp,
        Some(Err(_)) => return Err("Failed to get OAuth2 token response".to_string()),
        None => return Err("No OAuth2 token response received".to_string()),
    };

    // Check response status
    let response = response.expect("OAuth2 response unwrap");
    let status = response.status();
    if status < 200 || status >= 300 {
        return Err(format!("OAuth2 token request failed with status: {}", status));
    }

    // Read response body
    let response_body = response.consume()
        .map_err(|_| "Failed to consume OAuth2 token response body".to_string())?;
    let stream = response_body.stream()
        .map_err(|_| "Failed to get OAuth2 token response stream".to_string())?;

    let mut token_data = Vec::new();
    loop {
        match stream.blocking_read(4096) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                token_data.extend_from_slice(&chunk);
            }
            Err(_) => break,
        }
    }

    // Parse JSON response to extract access token
    let token_response = String::from_utf8(token_data)
        .map_err(|_| "Invalid UTF-8 in OAuth2 token response".to_string())?;

    // Simple JSON parsing for access_token field
    let access_token = extract_json_field(&token_response, "access_token")
        .ok_or_else(|| "No access_token in OAuth2 response".to_string())?;

    // Apply the access token as a Bearer token
    let auth_value = format!("Bearer {}", access_token);
    headers.set(
        &"authorization".to_string(),
        &[auth_value.as_bytes().to_vec()]
    ).map_err(|_| "Failed to set Authorization header with OAuth2 token".to_string())?;

    Ok(())
}

/// Apply OpenID Connect authentication
fn apply_oidc(config: &OidcConfig, headers: &Fields) -> Result<(), String> {
    use crate::bindings::wasi::http::outgoing_handler;
    use crate::bindings::wasi::http::types::{Method, OutgoingBody, OutgoingRequest};

    // Step 1: Discover OIDC configuration
    let discovery_url = format!("{}/.well-known/openid-configuration", config.issuer_url.trim_end_matches('/'));
    let (scheme, authority, path) = parse_token_url(&discovery_url)?;

    // Create discovery request
    let discovery_headers = Fields::new();
    let discovery_request = OutgoingRequest::new(discovery_headers);
    discovery_request.set_method(&Method::Get)
        .map_err(|_| "Failed to set method for OIDC discovery".to_string())?;
    discovery_request.set_scheme(Some(&scheme))
        .map_err(|_| "Failed to set scheme for OIDC discovery".to_string())?;
    discovery_request.set_authority(Some(&authority))
        .map_err(|_| "Failed to set authority for OIDC discovery".to_string())?;
    discovery_request.set_path_with_query(Some(&path))
        .map_err(|_| "Failed to set path for OIDC discovery".to_string())?;

    // Send discovery request
    let incoming_response = outgoing_handler::handle(discovery_request, None)
        .map_err(|_| "Failed to send OIDC discovery request".to_string())?;

    incoming_response.subscribe().block();

    let response = match incoming_response.get() {
        Some(Ok(resp)) => resp,
        Some(Err(_)) => return Err("Failed to get OIDC discovery response".to_string()),
        None => return Err("No OIDC discovery response received".to_string()),
    };

    let response = response.expect("OIDC discovery response unwrap");
    if response.status() != 200 {
        return Err(format!("OIDC discovery failed with status: {}", response.status()));
    }

    // Read discovery response
    let response_body = response.consume()
        .map_err(|_| "Failed to consume OIDC discovery response".to_string())?;
    let stream = response_body.stream()
        .map_err(|_| "Failed to get OIDC discovery stream".to_string())?;

    let mut discovery_data = Vec::new();
    loop {
        match stream.blocking_read(4096) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                discovery_data.extend_from_slice(&chunk);
            }
            Err(_) => break,
        }
    }

    let discovery_json = String::from_utf8(discovery_data)
        .map_err(|_| "Invalid UTF-8 in OIDC discovery response".to_string())?;

    // Extract token endpoint from discovery
    let token_endpoint = extract_json_field(&discovery_json, "token_endpoint")
        .ok_or_else(|| "No token_endpoint in OIDC discovery".to_string())?;

    // Step 2: Get token using client credentials (if client_id/secret provided)
    if let (Some(client_id), Some(client_secret)) = (&config.client_id, &config.client_secret) {
        // Use client credentials flow
        let (token_scheme, token_authority, token_path) = parse_token_url(&token_endpoint)?;

        let form_data = format!(
            "grant_type=client_credentials&client_id={}&client_secret={}&audience={}",
            urlencoding::encode(client_id),
            urlencoding::encode(client_secret),
            urlencoding::encode(&config.audience)
        );

        // Create token request
        let token_headers = Fields::new();
        token_headers.set(
            &"content-type".to_string(),
            &[b"application/x-www-form-urlencoded".to_vec()]
        ).map_err(|_| "Failed to set content-type for OIDC token request".to_string())?;

        let content_length = form_data.len().to_string();
        token_headers.set(
            &"content-length".to_string(),
            &[content_length.as_bytes().to_vec()]
        ).map_err(|_| "Failed to set content-length for OIDC token request".to_string())?;

        let token_request = OutgoingRequest::new(token_headers);
        token_request.set_method(&Method::Post)
            .map_err(|_| "Failed to set method for OIDC token request".to_string())?;
        token_request.set_scheme(Some(&token_scheme))
            .map_err(|_| "Failed to set scheme for OIDC token request".to_string())?;
        token_request.set_authority(Some(&token_authority))
            .map_err(|_| "Failed to set authority for OIDC token request".to_string())?;
        token_request.set_path_with_query(Some(&token_path))
            .map_err(|_| "Failed to set path for OIDC token request".to_string())?;

        // Write request body
        let body = token_request.body()
            .map_err(|_| "Failed to get OIDC token request body".to_string())?;
        let stream = body.write()
            .map_err(|_| "Failed to get OIDC token request stream".to_string())?;
        stream.blocking_write_and_flush(form_data.as_bytes())
            .map_err(|_| "Failed to write OIDC token request body".to_string())?;
        drop(stream);
        OutgoingBody::finish(body, None)
            .map_err(|_| "Failed to finish OIDC token request body".to_string())?;

        // Send token request
        let token_response = outgoing_handler::handle(token_request, None)
            .map_err(|_| "Failed to send OIDC token request".to_string())?;

        token_response.subscribe().block();

        let response = match token_response.get() {
            Some(Ok(resp)) => resp,
            Some(Err(_)) => return Err("Failed to get OIDC token response".to_string()),
            None => return Err("No OIDC token response received".to_string()),
        };

        let response = response.expect("OIDC token response unwrap");
        if response.status() < 200 || response.status() >= 300 {
            return Err(format!("OIDC token request failed with status: {}", response.status()));
        }

        // Read token response
        let response_body = response.consume()
            .map_err(|_| "Failed to consume OIDC token response".to_string())?;
        let stream = response_body.stream()
            .map_err(|_| "Failed to get OIDC token response stream".to_string())?;

        let mut token_data = Vec::new();
        loop {
            match stream.blocking_read(4096) {
                Ok(chunk) => {
                    if chunk.is_empty() {
                        break;
                    }
                    token_data.extend_from_slice(&chunk);
                }
                Err(_) => break,
            }
        }

        let token_json = String::from_utf8(token_data)
            .map_err(|_| "Invalid UTF-8 in OIDC token response".to_string())?;

        // Try to extract access_token or id_token
        let token = extract_json_field(&token_json, "access_token")
            .or_else(|| extract_json_field(&token_json, "id_token"))
            .ok_or_else(|| "No access_token or id_token in OIDC response".to_string())?;

        // Apply the token as a Bearer token
        let auth_value = format!("Bearer {}", token);
        headers.set(
            &"authorization".to_string(),
            &[auth_value.as_bytes().to_vec()]
        ).map_err(|_| "Failed to set Authorization header with OIDC token".to_string())?;

        Ok(())
    } else {
        // Without client credentials, we can't get a token automatically
        Err("OIDC authentication requires client_id and client_secret for automatic token acquisition".to_string())
    }
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

/// Parse URL into scheme, authority, and path components
fn parse_token_url(url: &str) -> Result<(crate::bindings::wasi::http::types::Scheme, String, String), String> {
    use crate::bindings::wasi::http::types::Scheme;

    let url = url.trim();

    let (scheme, rest) = if url.starts_with("https://") {
        (Scheme::Https, &url[8..])
    } else if url.starts_with("http://") {
        (Scheme::Http, &url[7..])
    } else {
        return Err("URL must start with http:// or https://".to_string());
    };

    // Find the path separator
    let (authority, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };

    Ok((scheme, authority.to_string(), path.to_string()))
}

/// Simple JSON field extraction (for basic JSON parsing without full deserialization)
fn extract_json_field(json: &str, field: &str) -> Option<String> {
    // Look for "field": "value" or "field":"value"
    let search_pattern = format!("\"{}\"", field);
    let field_pos = json.find(&search_pattern)?;

    // Find the colon after the field name
    let after_field = &json[field_pos + search_pattern.len()..];
    let colon_pos = after_field.find(':')?;

    // Find the opening quote for the value
    let after_colon = &after_field[colon_pos + 1..].trim_start();
    if !after_colon.starts_with('"') {
        // Handle non-string values (numbers, booleans, null)
        let end_pos = after_colon
            .find(|c: char| c == ',' || c == '}' || c == ']' || c.is_whitespace())
            .unwrap_or(after_colon.len());
        return Some(after_colon[..end_pos].trim().to_string());
    }

    // Extract string value
    let value_start = 1; // Skip opening quote
    let mut chars = after_colon[value_start..].chars();
    let mut value = String::new();
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if escaped {
            // Handle escaped characters
            match c {
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                '\\' => value.push('\\'),
                '"' => value.push('"'),
                _ => {
                    value.push('\\');
                    value.push(c);
                }
            }
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            // End of string value
            return Some(value);
        } else {
            value.push(c);
        }
    }

    None
}