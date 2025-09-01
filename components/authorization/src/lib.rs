use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod jwt;
mod policy;
mod discovery;
mod mcp;
mod error;

use error::AuthError;

// Component bindings will be generated here
#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        world: "authorization",
        exports: {
            "fastertools:mcp/authorization": Component,
            "fastertools:mcp/jwt-validator": Component,
            "fastertools:mcp/policy-engine": Component,
            "fastertools:mcp/oauth-discovery": Component,
            "fastertools:mcp/mcp-authorization": Component,
        }
    });
}

use bindings::exports::fastertools::mcp::{
    authorization::{
        AuthContext, AuthError as WitAuthError, AuthRequest, AuthResponse, Guest as AuthGuest,
    },
    jwt_validator::{
        Guest as JwtGuest, JwtClaims, JwtError, JwtRequest, JwtResult,
    },
    mcp_authorization::{
        Guest as McpAuthGuest, McpAuthRequest, ResourceAuthRequest, ResourceOperation,
        ToolAuthRequest,
    },
    oauth_discovery::{Guest as OAuthGuest, ResourceMetadata, ServerMetadata},
    policy_engine::{Guest as PolicyGuest, PolicyRequest, PolicyResult},
};

use bindings::fastertools::mcp::types::{JsonValue, MetaFields};

/// Main component struct
struct Component;

/// Authorization implementation
impl AuthGuest for Component {
    fn authorize(request: AuthRequest) -> AuthResponse {
        // First validate the JWT token
        let jwt_request = JwtRequest {
            token: request.token.clone(),
            expected_issuer: request.expected_issuer.clone(),
            expected_audience: request.expected_audience.clone(),
            jwks_uri: request.jwks_uri.clone(),
            jwks_json: None,
            validate_exp: Some(true),
            validate_nbf: Some(true),
            clock_skew: Some(60),
        };

        let jwt_result = jwt::validate(jwt_request);
        
        let claims = match jwt_result {
            JwtResult::Valid(claims) => claims,
            JwtResult::Invalid(error) => {
                return AuthResponse::Unauthorized(WitAuthError {
                    status: 401,
                    error_code: "invalid_token".to_string(),
                    description: format!("JWT validation failed: {:?}", error),
                    www_authenticate: Some(build_www_authenticate(&error)),
                });
            }
        };

        // Extract auth context from validated claims
        let auth_context = AuthContext {
            client_id: claims.client_id.clone(),
            user_id: Some(claims.sub.clone()),
            scopes: claims.scopes.clone(),
            issuer: Some(claims.iss.clone()),
            audience: claims.aud.and_then(|a| a.first().cloned()),
            claims: convert_claims_to_meta(&claims.additional_claims),
            exp: claims.exp,
            iat: claims.iat,
        };

        // Apply policy-based authorization if we have a body
        if let Some(body) = request.body {
            // Parse the body as JSON-RPC for MCP context
            if let Ok(json_str) = std::str::from_utf8(&body) {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                    // Build policy input
                    let policy_input = serde_json::json!({
                        "token": {
                            "sub": claims.sub,
                            "iss": claims.iss,
                            "aud": claims.aud,
                            "scopes": claims.scopes,
                            "client_id": claims.client_id,
                        },
                        "request": {
                            "method": request.method,
                            "path": request.path,
                            "headers": headers_to_object(&request.headers),
                        },
                        "mcp": parse_mcp_context(&json_value),
                    });

                    // Check if we have a policy configured (via environment or default)
                    if let Some(policy) = get_configured_policy() {
                        let policy_request = PolicyRequest {
                            policy,
                            data: get_configured_policy_data(),
                            input: serde_json::to_string(&policy_input).unwrap(),
                            query: Some("data.mcp.authorization.allow".to_string()),
                        };

                        match policy::evaluate(policy_request) {
                            PolicyResult::Allow => {
                                // Policy allowed, continue
                            }
                            PolicyResult::Deny(reason) => {
                                return AuthResponse::Unauthorized(WitAuthError {
                                    status: 403,
                                    error_code: "insufficient_scope".to_string(),
                                    description: format!("Authorization denied: {}", reason),
                                    www_authenticate: None,
                                });
                            }
                            PolicyResult::Error(err) => {
                                return AuthResponse::Unauthorized(WitAuthError {
                                    status: 500,
                                    error_code: "server_error".to_string(),
                                    description: format!("Policy evaluation failed: {}", err),
                                    www_authenticate: None,
                                });
                            }
                        }
                    }
                }
            }
        }

        AuthResponse::Authorized(auth_context)
    }
}

/// JWT Validator implementation
impl JwtGuest for Component {
    fn validate(request: JwtRequest) -> JwtResult {
        jwt::validate(request)
    }

    fn fetch_jwks(uri: String) -> Result<String, String> {
        jwt::fetch_jwks(&uri)
    }
}

/// Policy Engine implementation
impl PolicyGuest for Component {
    fn evaluate(request: PolicyRequest) -> PolicyResult {
        policy::evaluate(request)
    }
}

/// OAuth Discovery implementation
impl OAuthGuest for Component {
    fn get_resource_metadata() -> ResourceMetadata {
        discovery::get_resource_metadata()
    }

    fn get_server_metadata() -> ServerMetadata {
        discovery::get_server_metadata()
    }
}

/// MCP Authorization implementation
impl McpAuthGuest for Component {
    fn authorize_method(
        request: McpAuthRequest,
    ) -> Result<(), WitAuthError> {
        mcp::authorize_method(request)
    }

    fn authorize_tool(
        request: ToolAuthRequest,
    ) -> Result<(), WitAuthError> {
        mcp::authorize_tool(request)
    }

    fn authorize_resource(
        request: ResourceAuthRequest,
    ) -> Result<(), WitAuthError> {
        mcp::authorize_resource(request)
    }
}

// Helper functions

fn build_www_authenticate(error: &JwtError) -> String {
    let error_code = match error {
        JwtError::Expired => "invalid_token",
        JwtError::InvalidSignature => "invalid_token",
        JwtError::InvalidIssuer => "invalid_token",
        JwtError::InvalidAudience => "invalid_token",
        _ => "invalid_token",
    };
    
    let description = match error {
        JwtError::Expired => "Token has expired",
        JwtError::InvalidSignature => "Invalid token signature",
        JwtError::InvalidIssuer => "Invalid token issuer",
        JwtError::InvalidAudience => "Invalid token audience",
        JwtError::Malformed => "Malformed token",
        JwtError::NotYetValid => "Token not yet valid",
        JwtError::MissingClaim => "Required claim missing",
        JwtError::JwksError => "JWKS validation error",
        JwtError::UnknownKid => "Unknown key ID",
        JwtError::Other => "Token validation failed",
    };
    
    format!(r#"Bearer error="{}", error_description="{}""#, error_code, description)
}

fn convert_claims_to_meta(claims: &[(String, String)]) -> Vec<(String, String)> {
    claims.to_vec()
}

fn headers_to_object(headers: &[(String, String)]) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (key, value) in headers {
        map.insert(key.clone(), serde_json::Value::String(value.clone()));
    }
    serde_json::Value::Object(map)
}

fn parse_mcp_context(json: &serde_json::Value) -> serde_json::Value {
    if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
        let mut context = serde_json::json!({
            "method": method,
        });
        
        if method == "tools/call" {
            if let Some(params) = json.get("params") {
                if let Some(name) = params.get("name") {
                    context["tool"] = name.clone();
                }
                if let Some(args) = params.get("arguments") {
                    context["arguments"] = args.clone();
                }
            }
        }
        
        context
    } else {
        serde_json::json!({})
    }
}

fn get_configured_policy() -> Option<String> {
    // In production, this would read from environment or configuration
    // For now, return a default permissive policy
    Some(r#"
        package mcp.authorization
        
        default allow = false
        
        # Allow all authenticated requests by default
        # Override this with your own policy
        allow {
            input.token.sub != ""
        }
    "#.to_string())
}

fn get_configured_policy_data() -> Option<String> {
    // Optional external data for policy evaluation
    None
}

// Export the component
bindings::export!(Component with_types_in bindings);