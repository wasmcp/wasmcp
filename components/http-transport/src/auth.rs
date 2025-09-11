use crate::auth_types::*;
use crate::{jwt, policy};

/// Main authorization function - checks JWT token and applies policy
pub fn authorize(request: AuthRequest) -> AuthResponse {
    // All configuration comes from the request (required fields)
    let expected_issuer = request.expected_issuer.clone();
    let expected_audiences = request.expected_audiences.clone();
    let expected_subject = request.expected_subject.clone();
    let jwks_uri = request.jwks_uri.clone();

    // Default clock skew to 60 seconds
    let clock_skew = 60;

    // First validate the JWT token
    let jwt_request = JwtRequest {
        token: request.token.clone(),
        expected_issuer,
        expected_audiences,
        expected_subject,
        jwks_uri,
        jwks_json: None,
        clock_skew: Some(clock_skew),
    };

    let jwt_result = jwt::validate(jwt_request);

    let claims = match jwt_result {
        JwtResult::Valid(claims) => claims,
        JwtResult::Invalid(error) => {
            return AuthResponse::Unauthorized(AuthError {
                status: 401,
                error_code: "invalid_token".to_string(),
                description: format!("JWT validation failed: {error:?}"),
                www_authenticate: Some(build_www_authenticate(&error)),
            });
        }
    };

    // Extract auth context from validated claims
    // exp is guaranteed to exist because validation.require_exp = true
    let auth_context = AuthContext {
        client_id: claims.client_id.clone(),
        sub: claims.sub.clone(),  // Standard JWT claim name
        scopes: claims.scopes.clone(),
        iss: claims.iss.clone(),  // Standard JWT claim name
        aud: claims.aud.clone().unwrap_or_else(Vec::new),  // Required by validation, but defensive
        claims: convert_claims_to_meta(&claims.additional_claims),
        exp: claims.exp.unwrap(),  // Safe: validation.require_exp ensures this exists
        iat: claims.iat,
        nbf: claims.nbf,
        jwt: if request.pass_jwt {
            Some(request.token.clone())
        } else {
            None
        },
    };

    // Apply policy-based authorization if we have a body
    if let Some(body) = request.body {
        // Parse the body as JSON-RPC for MCP context
        if let Ok(json_str) = std::str::from_utf8(&body)
            && let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str)
        {
            // Build policy input
            let policy_input = serde_json::json!({
                "token": {
                    "sub": claims.sub,
                    "iss": claims.iss,
                    "aud": claims.aud.clone(),
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

            // If policy is provided, evaluate it
            if let Some(policy) = request.policy.clone() {
                let policy_request = PolicyRequest {
                    policy,
                    data: request.policy_data.clone(),
                    input: serde_json::to_string(&policy_input).unwrap(),
                    query: Some("data.mcp.authorization.allow".to_string()),
                };

                match policy::evaluate(policy_request) {
                    PolicyResult::Allow => {
                        // Policy allowed, continue
                    }
                    PolicyResult::Deny(reason) => {
                        return AuthResponse::Unauthorized(AuthError {
                            status: 403,
                            error_code: "insufficient_scope".to_string(),
                            description: format!("Authorization denied: {reason}"),
                            www_authenticate: None,
                        });
                    }
                    PolicyResult::Error(err) => {
                        return AuthResponse::Unauthorized(AuthError {
                            status: 500,
                            error_code: "server_error".to_string(),
                            description: format!("Policy evaluation failed: {err}"),
                            www_authenticate: None,
                        });
                    }
                }
            }
            // If no policy provided, allow all authenticated requests
        }
    }

    AuthResponse::Authorized(auth_context)
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

    format!(r#"Bearer error="{error_code}", error_description="{description}""#)
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

        if method == "tools/call"
            && let Some(params) = json.get("params")
        {
            if let Some(name) = params.get("name") {
                context["tool"] = name.clone();
            }
            if let Some(args) = params.get("arguments") {
                context["arguments"] = args.clone();
            }
        }

        context
    } else {
        serde_json::json!({})
    }
}
