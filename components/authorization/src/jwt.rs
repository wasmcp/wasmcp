use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use base64::{Engine as _, engine::general_purpose};
use spin_sdk::http::{send, Method, Request};
use once_cell::sync::Lazy;

use crate::bindings::exports::fastertools::mcp::jwt_validator::{
    JwtClaims, JwtError, JwtRequest, JwtResult,
};

/// Standard JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StandardClaims {
    // Required claims
    sub: String,
    iss: String,
    
    // Optional standard claims
    #[serde(skip_serializing_if = "Option::is_none")]
    aud: Option<AudienceClaim>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nbf: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    iat: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    jti: Option<String>,
    
    // OAuth-specific claims
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    azp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    
    // Capture all other claims
    #[serde(flatten)]
    additional: HashMap<String, serde_json::Value>,
}

/// Audience claim can be string or array of strings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum AudienceClaim {
    Single(String),
    Multiple(Vec<String>),
}

impl AudienceClaim {
    fn to_vec(&self) -> Vec<String> {
        match self {
            AudienceClaim::Single(s) => vec![s.clone()],
            AudienceClaim::Multiple(v) => v.clone(),
        }
    }
}

/// JWKS structure for key discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Jwk {
    kty: String,
    kid: Option<String>,
    alg: Option<String>,
    #[serde(rename = "use")]
    use_: Option<String>,
    
    // RSA specific
    n: Option<String>,
    e: Option<String>,
    
    // HMAC specific
    k: Option<String>,
}

/// Cache for JWKS to avoid repeated fetches
static JWKS_CACHE: Lazy<Mutex<HashMap<String, (Jwks, u64)>>> = Lazy::new(|| {
    Mutex::new(HashMap::new())
});
const JWKS_CACHE_DURATION: u64 = 3600; // 1 hour

pub fn validate(request: JwtRequest) -> JwtResult {
    // Parse the token header first to get the algorithm and key ID
    let header = match decode_header(&request.token) {
        Ok(h) => h,
        Err(_) => return JwtResult::Invalid(JwtError::Malformed),
    };
    
    // Set up validation parameters
    let mut validation = Validation::new(header.alg);
    
    // Configure validation based on request parameters
    if let Some(false) = request.validate_exp {
        validation.validate_exp = false;
    }
    if let Some(false) = request.validate_nbf {
        validation.validate_nbf = false;
    }
    
    // Set expected issuer (now required)
    validation.set_issuer(&[request.expected_issuer.as_str()]);
    
    // Set expected audiences (now required)
    validation.set_audience(&request.expected_audiences.iter().map(|s| s.as_str()).collect::<Vec<_>>());
    
    // Get the decoding key
    let decoding_key = match get_decoding_key(&header, &request) {
        Ok(key) => key,
        Err(err) => return JwtResult::Invalid(err),
    };
    
    // Decode and validate the token
    let token_data = match decode::<StandardClaims>(&request.token, &decoding_key, &validation) {
        Ok(data) => data,
        Err(err) => {
            return JwtResult::Invalid(map_jwt_error(err));
        }
    };
    
    let claims = token_data.claims;
    
    // Extract scopes from the scope claim
    let scopes = claims.scope
        .as_ref()
        .map(|s| s.split_whitespace().map(String::from).collect())
        .unwrap_or_default();
    
    // Extract client ID (prefer azp over client_id)
    let client_id = claims.azp.clone().or(claims.client_id.clone());
    
    // Convert additional claims to string pairs
    let additional_claims: Vec<(String, String)> = claims.additional
        .into_iter()
        .map(|(k, v)| (k, serde_json::to_string(&v).unwrap_or_else(|_| v.to_string())))
        .collect();
    
    // Build the validated claims response
    JwtResult::Valid(JwtClaims {
        sub: claims.sub,
        iss: claims.iss,
        aud: claims.aud.map(|a| a.to_vec()),
        exp: claims.exp.map(|e| e as u64),
        iat: claims.iat.map(|i| i as u64),
        nbf: claims.nbf.map(|n| n as u64),
        jti: claims.jti,
        scopes,
        client_id,
        additional_claims,
    })
}

fn get_decoding_key(
    header: &jsonwebtoken::Header,
    request: &JwtRequest,
) -> Result<DecodingKey, JwtError> {
    // If JWKS JSON is provided directly, use it
    if let Some(ref jwks_json) = request.jwks_json {
        let jwks: Jwks = serde_json::from_str(jwks_json)
            .map_err(|_| JwtError::JwksError)?;
        return find_key_in_jwks(&jwks, header.kid.as_deref());
    }
    
    // Fetch and cache JWKS (now required)
    let jwks = fetch_and_cache_jwks(&request.jwks_uri)?;
    find_key_in_jwks(&jwks, header.kid.as_deref())
}

fn find_key_in_jwks(jwks: &Jwks, kid: Option<&str>) -> Result<DecodingKey, JwtError> {
    // Find the appropriate key
    let jwk = if let Some(kid) = kid {
        jwks.keys.iter().find(|k| k.kid.as_deref() == Some(kid))
    } else {
        // No kid specified, use first signing key
        jwks.keys.iter().find(|k| k.use_.as_deref() == Some("sig"))
    };
    
    let jwk = jwk.ok_or(JwtError::UnknownKid)?;
    
    // Convert JWK to DecodingKey based on key type
    match jwk.kty.as_str() {
        "RSA" => {
            let n = jwk.n.as_ref().ok_or(JwtError::JwksError)?;
            let e = jwk.e.as_ref().ok_or(JwtError::JwksError)?;
            DecodingKey::from_rsa_components(n, e)
                .map_err(|_| JwtError::JwksError)
        }
        "oct" => {
            let k = jwk.k.as_ref().ok_or(JwtError::JwksError)?;
            let key_bytes = general_purpose::URL_SAFE_NO_PAD.decode(k)
                .map_err(|_| JwtError::JwksError)?;
            Ok(DecodingKey::from_secret(&key_bytes))
        }
        _ => Err(JwtError::JwksError),
    }
}

fn fetch_and_cache_jwks(uri: &str) -> Result<Jwks, JwtError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Check cache
    {
        let cache = JWKS_CACHE.lock().unwrap();
        if let Some((jwks, cached_at)) = cache.get(uri) {
            if now - cached_at < JWKS_CACHE_DURATION {
                return Ok(jwks.clone());
            }
        }
    }
    
    // Fetch JWKS using spin-sdk HTTP client
    let jwks_json = fetch_jwks(uri).map_err(|_| JwtError::JwksError)?;
    let jwks: Jwks = serde_json::from_str(&jwks_json)
        .map_err(|_| JwtError::JwksError)?;
    
    // Update cache
    {
        let mut cache = JWKS_CACHE.lock().unwrap();
        cache.insert(uri.to_string(), (jwks.clone(), now));
    }
    
    Ok(jwks)
}

pub fn fetch_jwks(uri: &str) -> Result<String, String> {
    // Use spin-sdk for WASI-compatible HTTP requests
    eprintln!("Fetching JWKS from: {}", uri);
    
    // Use spin's executor to run async HTTP request
    spin_sdk::http::run(async move {
        let request = Request::builder()
            .method(Method::Get)
            .uri(uri)
            .build();
        
        let response: spin_sdk::http::Response = send(request)
            .await
            .map_err(|e| format!("Failed to fetch JWKS: {:?}", e))?;
        
        if response.status() != &200 {
            return Err(format!("JWKS fetch failed with status: {:?}", response.status()));
        }
        
        let body = response.into_body();
        let text = String::from_utf8(body)
            .map_err(|e| format!("Failed to parse JWKS response: {}", e))?;
        
        eprintln!("Successfully fetched JWKS");
        Ok(text)
    })
}

fn map_jwt_error(err: jsonwebtoken::errors::Error) -> JwtError {
    use jsonwebtoken::errors::ErrorKind;
    
    match err.kind() {
        ErrorKind::InvalidToken => JwtError::Malformed,
        ErrorKind::InvalidSignature => JwtError::InvalidSignature,
        ErrorKind::ExpiredSignature => JwtError::Expired,
        ErrorKind::ImmatureSignature => JwtError::NotYetValid,
        ErrorKind::InvalidIssuer => JwtError::InvalidIssuer,
        ErrorKind::InvalidAudience => JwtError::InvalidAudience,
        _ => JwtError::Other,
    }
}

