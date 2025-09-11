use crate::error::{McpError, AuthError};
use crate::runtime::{HttpClient, TimeProvider, CacheProvider};
use jsonwebtoken::{Validation, DecodingKey, decode, decode_header};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::collections::HashMap;
use base64::{Engine, engine::general_purpose};

/// JWT validator that uses runtime abstractions for HTTP and caching
pub struct JwtValidator<H: HttpClient, T: TimeProvider, C: CacheProvider> {
    http_client: H,
    #[allow(dead_code)]
    time_provider: T,
    cache_provider: C,
    jwks_url: Option<String>,
    expected_issuer: Option<String>,
    expected_audiences: Vec<String>,
}

impl<H: HttpClient, T: TimeProvider, C: CacheProvider> JwtValidator<H, T, C> {
    pub fn new(http_client: H, time_provider: T, cache_provider: C) -> Self {
        Self {
            http_client,
            time_provider,
            cache_provider,
            jwks_url: None,
            expected_issuer: None,
            expected_audiences: Vec::new(),
        }
    }

    pub fn with_jwks_url(mut self, url: String) -> Self {
        self.jwks_url = Some(url);
        self
    }

    pub fn with_expected_issuer(mut self, issuer: String) -> Self {
        self.expected_issuer = Some(issuer);
        self
    }

    pub fn with_expected_audiences(mut self, audiences: Vec<String>) -> Self {
        self.expected_audiences = audiences;
        self
    }

    /// Validate a JWT token
    pub async fn validate_token(&self, token: &str) -> Result<TokenClaims, McpError> {
        // Parse the token header to get algorithm and key ID
        let header = decode_header(token)
            .map_err(|_| McpError::Auth(AuthError::InvalidToken))?;

        // Set up validation parameters
        let mut validation = Validation::new(header.alg);
        
        // Configure expected issuer and audiences if provided
        if let Some(ref issuer) = self.expected_issuer {
            validation.set_issuer(&[issuer.as_str()]);
        }
        
        if !self.expected_audiences.is_empty() {
            let audiences: Vec<&str> = self.expected_audiences.iter().map(|s| s.as_str()).collect();
            validation.set_audience(&audiences);
        }

        // Get the decoding key from JWKS
        let decoding_key = self.get_decoding_key(&header).await?;

        // Decode and validate the token
        let token_data = decode::<StandardClaims>(token, &decoding_key, &validation)
            .map_err(|e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => McpError::Auth(AuthError::TokenExpired),
                jsonwebtoken::errors::ErrorKind::InvalidSignature => McpError::Auth(AuthError::InvalidSignature),
                _ => McpError::Auth(AuthError::InvalidToken),
            })?;

        let claims = token_data.claims;

        Ok(TokenClaims {
            sub: claims.sub,
            exp: claims.exp,
            iat: claims.iat,
            aud: claims.aud.map(|a| match a {
                AudienceClaim::Single(s) => vec![s],
                AudienceClaim::Multiple(v) => v,
            }),
            iss: Some(claims.iss),
            additional: claims.additional,
        })
    }

    /// Get the decoding key from JWKS
    async fn get_decoding_key(&self, header: &jsonwebtoken::Header) -> Result<DecodingKey, McpError> {
        let jwks = self.fetch_jwks().await?;
        let jwks: Jwks = serde_json::from_str(&jwks)
            .map_err(|_| McpError::Auth(AuthError::JwksDiscovery("Invalid JWKS format".to_string())))?;

        // Find the appropriate key
        let jwk = if let Some(ref kid) = header.kid {
            jwks.keys.iter().find(|k| k.kid.as_deref() == Some(kid.as_str()))
        } else {
            // No kid specified, use first signing key
            jwks.keys.iter().find(|k| k.use_.as_deref() == Some("sig"))
        };

        let jwk = jwk.ok_or_else(|| McpError::Auth(AuthError::JwksDiscovery("Key not found in JWKS".to_string())))?;

        // Convert JWK to DecodingKey based on key type
        match jwk.kty.as_str() {
            "RSA" => {
                let n = jwk.n.as_ref()
                    .ok_or_else(|| McpError::Auth(AuthError::JwksDiscovery("Missing RSA modulus".to_string())))?;
                let e = jwk.e.as_ref()
                    .ok_or_else(|| McpError::Auth(AuthError::JwksDiscovery("Missing RSA exponent".to_string())))?;
                DecodingKey::from_rsa_components(n, e)
                    .map_err(|_| McpError::Auth(AuthError::JwksDiscovery("Invalid RSA components".to_string())))
            }
            "oct" => {
                let k = jwk.k.as_ref()
                    .ok_or_else(|| McpError::Auth(AuthError::JwksDiscovery("Missing symmetric key".to_string())))?;
                let key_bytes = general_purpose::URL_SAFE_NO_PAD
                    .decode(k)
                    .map_err(|_| McpError::Auth(AuthError::JwksDiscovery("Invalid base64 key".to_string())))?;
                Ok(DecodingKey::from_secret(&key_bytes))
            }
            _ => Err(McpError::Auth(AuthError::JwksDiscovery(format!("Unsupported key type: {}", jwk.kty)))),
        }
    }

    /// Fetch JWKS from the configured URL with caching
    async fn fetch_jwks(&self) -> Result<String, McpError> {
        let url = self.jwks_url.as_ref()
            .ok_or_else(|| McpError::Config("JWKS URL not configured".to_string()))?;
        
        // Check cache first
        let cache_key = format!("jwks:{}", url);
        if let Some(cached) = self.cache_provider.get(&cache_key).await {
            if let Ok(jwks) = String::from_utf8(cached) {
                return Ok(jwks);
            }
        }

        // Fetch from HTTP
        let jwks = self.http_client.get(url).await?;
        
        // Cache for 1 hour
        let _ = self.cache_provider
            .set(&cache_key, jwks.as_bytes().to_vec(), Duration::from_secs(3600))
            .await;

        Ok(jwks)
    }
}

/// Standard JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StandardClaims {
    sub: String,
    iss: String,
    
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub exp: Option<i64>,
    pub iat: Option<i64>,
    pub aud: Option<Vec<String>>,
    pub iss: Option<String>,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}