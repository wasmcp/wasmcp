//! JWKS (JSON Web Key Set) fetching and caching

use crate::bindings::wasi::http::outgoing_handler;
use crate::bindings::wasi::http::types::{Fields, Method, OutgoingRequest, Scheme};
use crate::bindings::wasmcp::keyvalue::store as kv;
use crate::error::{AuthError, Result};
use jsonwebtoken::DecodingKey;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default JWKS cache TTL in seconds (5 minutes - matches ftl behavior)
/// Can be overridden via JWT_JWKS_TTL environment variable
const DEFAULT_JWKS_CACHE_TTL: u64 = 300;

/// Get JWKS cache TTL from environment or use default
fn get_jwks_ttl() -> u64 {
    std::env::var("JWT_JWKS_TTL")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_JWKS_CACHE_TTL)
}

/// JWKS response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

/// JSON Web Key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    /// Key type (RSA, EC, etc.)
    pub kty: String,

    /// Key use (sig, enc)
    #[serde(rename = "use")]
    pub use_: Option<String>,

    /// Algorithm
    pub alg: Option<String>,

    /// Key ID
    pub kid: Option<String>,

    /// RSA modulus (base64url)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,

    /// RSA exponent (base64url)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub e: Option<String>,
}

/// Cached JWKS with expiration
#[derive(Debug, Serialize, Deserialize)]
struct CachedJwks {
    jwks: Jwks,
    expires_at: u64,
}

/// Fetch JWKS from URI with caching
pub fn fetch_jwks(jwks_uri: &str) -> Result<Jwks> {
    eprintln!("[oauth-auth:jwks] Fetching JWKS from: {}", jwks_uri);

    let cache_key = "oauth-jwks";

    // Get bucket name from environment (must match MCP_SESSION_BUCKET)
    let bucket_name = std::env::var("MCP_SESSION_BUCKET")
        .or_else(|_| std::env::var("MCP_KV_BUCKET"))
        .unwrap_or_else(|_| "default".to_string());

    eprintln!("[oauth-auth:jwks] Using KV bucket: {}", bucket_name);

    // Open KV bucket for caching
    let bucket = kv::open(&bucket_name).map_err(|e| {
        AuthError::Internal(format!("Failed to open KV store '{}': {}", bucket_name, e))
    })?;

    // Check cache first
    if let Ok(Some(cached_value)) = bucket.get(&cache_key) {
        eprintln!("[oauth-auth:jwks] Found cached JWKS, checking expiry...");

        // Extract string from typed-value
        let cached_str = match cached_value {
            kv::TypedValue::AsJson(s) | kv::TypedValue::AsString(s) => s,
            _ => return Err(AuthError::Internal("Invalid cached JWKS type".to_string())),
        };

        if let Ok(cached) = serde_json::from_str::<CachedJwks>(&cached_str) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            eprintln!(
                "[oauth-auth:jwks] Cache expires at: {}, now: {}",
                cached.expires_at, now
            );

            if now < cached.expires_at {
                eprintln!(
                    "[oauth-auth:jwks] ✓ Using cached JWKS ({} keys)",
                    cached.jwks.keys.len()
                );
                return Ok(cached.jwks);
            } else {
                eprintln!("[oauth-auth:jwks] Cache expired, fetching fresh JWKS");
            }
        }
    } else {
        eprintln!("[oauth-auth:jwks] No cached JWKS found, fetching from URI");
    }

    // Fetch JWKS from URI
    let jwks = fetch_jwks_http(jwks_uri)?;

    eprintln!(
        "[oauth-auth:jwks] ✓ Fetched JWKS with {} keys",
        jwks.keys.len()
    );

    // Cache the JWKS with configured TTL
    let ttl = get_jwks_ttl();
    eprintln!("[oauth-auth:jwks] Caching JWKS with TTL: {}s", ttl);

    let expires_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + ttl;

    let cached = CachedJwks {
        jwks: jwks.clone(),
        expires_at,
    };

    if let Ok(cached_json) = serde_json::to_string(&cached) {
        match bucket.set(&cache_key, &kv::TypedValue::AsJson(cached_json)) {
            Ok(_) => eprintln!("[oauth-auth:jwks] ✓ JWKS cached successfully"),
            Err(e) => eprintln!("[oauth-auth:jwks] ✗ Failed to cache JWKS: {}", e),
        }
    }

    Ok(jwks)
}

/// Fetch JWKS via HTTP
fn fetch_jwks_http(uri: &str) -> Result<Jwks> {
    eprintln!("[oauth-auth:jwks] Fetching JWKS via HTTP from: {}", uri);

    // Parse URI
    let url = uri
        .parse::<url::Url>()
        .map_err(|e| AuthError::Configuration(format!("Invalid JWKS URI: {}", e)))?;

    let scheme = match url.scheme() {
        "https" => Scheme::Https,
        "http" => Scheme::Http,
        _ => {
            return Err(AuthError::Configuration(
                "JWKS URI must use HTTP(S)".to_string(),
            ));
        }
    };

    let authority = url
        .host_str()
        .ok_or_else(|| AuthError::Configuration("Missing host in JWKS URI".to_string()))?
        .to_string();

    let path_and_query = if let Some(query) = url.query() {
        format!("{}?{}", url.path(), query)
    } else {
        url.path().to_string()
    };

    // Create outgoing request
    let headers = Fields::new();
    headers
        .append(
            &"Accept".to_string(),
            &"application/json".as_bytes().to_vec(),
        )
        .map_err(|_| AuthError::Internal("Failed to set Accept header".to_string()))?;

    let request = OutgoingRequest::new(headers);
    request
        .set_scheme(Some(&scheme))
        .map_err(|_| AuthError::Internal("Failed to set scheme".to_string()))?;
    request
        .set_authority(Some(&authority))
        .map_err(|_| AuthError::Internal("Failed to set authority".to_string()))?;
    request
        .set_path_with_query(Some(&path_and_query))
        .map_err(|_| AuthError::Internal("Failed to set path".to_string()))?;
    request
        .set_method(&Method::Get)
        .map_err(|_| AuthError::Internal("Failed to set method".to_string()))?;

    // Send request
    let future_response = outgoing_handler::handle(request, None)
        .map_err(|e| AuthError::Internal(format!("Failed to send JWKS request: {:?}", e)))?;

    // Wait for the response using pollable
    use crate::bindings::wasi::io::poll;
    let pollable = future_response.subscribe();
    poll::poll(&[&pollable]);
    drop(pollable);

    let incoming_response = future_response
        .get()
        .ok_or_else(|| AuthError::Internal("JWKS request not ready after poll".to_string()))?
        .map_err(|e| AuthError::Internal(format!("JWKS request failed: {:?}", e)))?
        .map_err(|e| AuthError::Internal(format!("JWKS HTTP error: {:?}", e)))?;

    let status = incoming_response.status();
    eprintln!("[oauth-auth:jwks] HTTP response status: {}", status);

    if status != 200 {
        eprintln!(
            "[oauth-auth:jwks] ✗ JWKS fetch failed with status: {}",
            status
        );
        return Err(AuthError::Internal(format!(
            "JWKS fetch failed with status: {}",
            status
        )));
    }

    // Read response body
    let body = incoming_response
        .consume()
        .map_err(|_| AuthError::Internal("Failed to consume response body".to_string()))?;

    let stream = body
        .stream()
        .map_err(|_| AuthError::Internal("Failed to get body stream".to_string()))?;

    let mut body_bytes = Vec::new();
    loop {
        use crate::bindings::wasi::io::streams::StreamError;

        match stream.blocking_read(8192) {
            Ok(chunk) => {
                if chunk.is_empty() {
                    break;
                }
                body_bytes.extend_from_slice(&chunk);
            }
            Err(StreamError::Closed) => break,
            Err(e) => {
                drop(stream);
                return Err(AuthError::Internal(format!("Stream error: {:?}", e)));
            }
        }
    }

    drop(stream);

    // Parse JWKS
    let jwks: Jwks = serde_json::from_slice(&body_bytes)
        .map_err(|e| AuthError::Internal(format!("Failed to parse JWKS: {}", e)))?;

    Ok(jwks)
}

/// Find a key in JWKS that matches the given KID
pub fn find_key(jwks: &Jwks, kid: Option<&str>) -> Result<DecodingKey> {
    eprintln!("[oauth-auth:jwks] Finding key in JWKS");
    eprintln!("[oauth-auth:jwks]   Looking for kid: {:?}", kid);
    eprintln!(
        "[oauth-auth:jwks]   Total keys in JWKS: {}",
        jwks.keys.len()
    );

    // Log all available keys
    for (i, key) in jwks.keys.iter().enumerate() {
        eprintln!(
            "[oauth-auth:jwks]   Key {}: kty={}, kid={:?}, alg={:?}, use={:?}",
            i, key.kty, key.kid, key.alg, key.use_
        );
    }

    // Filter keys by type and use
    let matching_keys: Vec<&Jwk> = jwks
        .keys
        .iter()
        .filter(|key| {
            // Check key type
            if key.kty != "RSA" {
                return false;
            }

            // Check use if specified
            if let Some(use_) = &key.use_ {
                if use_ != "sig" {
                    return false;
                }
            }

            true
        })
        .collect();

    eprintln!(
        "[oauth-auth:jwks]   Matching RSA sig keys: {}",
        matching_keys.len()
    );

    if matching_keys.is_empty() {
        return Err(AuthError::InvalidToken(
            "No matching keys found in JWKS".to_string(),
        ));
    }

    // Find key by KID if specified
    let key = if let Some(kid) = kid {
        eprintln!("[oauth-auth:jwks]   Token has kid, looking for exact match...");
        // Token has KID - find exact match
        matching_keys
            .iter()
            .find(|k| k.kid.as_deref() == Some(kid))
            .ok_or_else(|| {
                eprintln!(
                    "[oauth-auth:jwks] ✗ Key with kid '{}' not found in JWKS",
                    kid
                );
                AuthError::InvalidToken(format!("Key with kid '{}' not found", kid))
            })?
    } else {
        eprintln!("[oauth-auth:jwks]   No kid in token, using single key logic...");
        // No KID in token - only allow if there's exactly one key
        if matching_keys.len() == 1 {
            eprintln!("[oauth-auth:jwks]   Using single available key");
            matching_keys
                .first()
                .copied()
                .ok_or_else(|| AuthError::InvalidToken("No keys found".to_string()))?
        } else if matching_keys.is_empty() {
            eprintln!("[oauth-auth:jwks] ✗ No keys found in JWKS");
            return Err(AuthError::InvalidToken("No keys found in JWKS".to_string()));
        } else {
            eprintln!("[oauth-auth:jwks] ✗ Multiple keys in JWKS but no kid in token");
            return Err(AuthError::InvalidToken(
                "Multiple keys in JWKS but no key ID (kid) in token".to_string(),
            ));
        }
    };

    eprintln!("[oauth-auth:jwks] ✓ Found matching key: kid={:?}", key.kid);

    // Extract RSA components
    let n = key
        .n
        .as_ref()
        .ok_or_else(|| AuthError::InvalidToken("Missing RSA modulus".to_string()))?;
    let e = key
        .e
        .as_ref()
        .ok_or_else(|| AuthError::InvalidToken("Missing RSA exponent".to_string()))?;

    // Build RSA public key
    build_rsa_key(n, e)
}

/// Build RSA decoding key from modulus and exponent
fn build_rsa_key(n: &str, e: &str) -> Result<DecodingKey> {
    DecodingKey::from_rsa_components(n, e)
        .map_err(|e| AuthError::InvalidToken(format!("Invalid RSA key components: {}", e)))
}
