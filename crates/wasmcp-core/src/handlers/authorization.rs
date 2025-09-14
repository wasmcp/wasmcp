use crate::{traits::McpAuthorizationHandler, ErrorCode, McpError};
use serde_json::Value;

/// Generic handler for auth config retrieval.
pub fn get_auth_config(
    provider: &dyn McpAuthorizationHandler,
) -> Result<Value, McpError> {
    let result = provider.get_auth_config()?;
    Ok(serde_json::to_value(result).unwrap())
}

/// Generic handler for JWKS cache get operation.
pub fn jwks_cache_get(
    provider: &dyn McpAuthorizationHandler,
    params: Option<Value>,
) -> Result<Value, McpError> {
    let params_obj = params.ok_or_else(|| McpError {
        code: ErrorCode::InvalidParams,
        message: "Missing params".to_string(),
        data: None,
    })?;

    let jwks_uri = params_obj
        .get("jwks_uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: ErrorCode::InvalidParams,
            message: "Missing jwks_uri parameter".to_string(),
            data: None,
        })?;

    let result = provider.jwks_cache_get(jwks_uri.to_string())?;
    Ok(serde_json::to_value(result).unwrap())
}

/// Generic handler for JWKS cache set operation.
pub fn jwks_cache_set(
    provider: &dyn McpAuthorizationHandler,
    params: Option<Value>,
) -> Result<Value, McpError> {
    let params_obj = params.ok_or_else(|| McpError {
        code: ErrorCode::InvalidParams,
        message: "Missing params".to_string(),
        data: None,
    })?;

    let jwks_uri = params_obj
        .get("jwks_uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: ErrorCode::InvalidParams,
            message: "Missing jwks_uri parameter".to_string(),
            data: None,
        })?;

    let jwks = params_obj
        .get("jwks")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError {
            code: ErrorCode::InvalidParams,
            message: "Missing jwks parameter".to_string(),
            data: None,
        })?;

    provider.jwks_cache_set(jwks_uri.to_string(), jwks.to_string())?;
    Ok(Value::Null)
}