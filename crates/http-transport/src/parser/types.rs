//! Shadow types for JSON-RPC deserialization
//!
//! This module contains intermediate types used for deserializing JSON-RPC
//! requests before converting them to WIT types.

use crate::bindings::wasmcp::mcp_v20250618::mcp::{
    ClientCapabilities, ClientLists, Implementation, ProtocolVersion, RequestId,
};
use serde::Deserialize;
use serde_json::Value;

// =============================================================================
// REQUEST ID TYPES
// =============================================================================

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum JsonRequestId {
    Number(i64),
    String(String),
}

impl From<JsonRequestId> for RequestId {
    fn from(id: JsonRequestId) -> Self {
        match id {
            JsonRequestId::Number(n) => RequestId::Number(n),
            JsonRequestId::String(s) => RequestId::String(s),
        }
    }
}

// =============================================================================
// INITIALIZE REQUEST TYPES
// =============================================================================

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JsonInitializeRequestParams {
    pub protocol_version: String,
    pub capabilities: JsonClientCapabilities,
    pub client_info: JsonImplementation,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JsonClientCapabilities {
    #[serde(default)]
    pub elicitation: Option<Value>,
    #[serde(default)]
    pub experimental: Option<Vec<(String, Value)>>,
    #[serde(default)]
    pub roots: Option<JsonRootsCapability>,
    #[serde(default)]
    pub sampling: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JsonRootsCapability {
    #[serde(default)]
    pub list_changed: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JsonImplementation {
    pub name: String,
    #[serde(default)]
    pub title: Option<String>,
    pub version: String,
}

// =============================================================================
// CONVERSION FUNCTIONS
// =============================================================================

pub(crate) fn parse_protocol_version(s: &str) -> Result<ProtocolVersion, String> {
    match s {
        "2025-06-18" => Ok(ProtocolVersion::V20250618),
        "2025-03-26" => Ok(ProtocolVersion::V20250326),
        "2024-11-05" => Ok(ProtocolVersion::V20241105),
        _ => Err(format!("Unsupported protocol version: {}", s)),
    }
}

pub(crate) fn convert_client_capabilities(caps: JsonClientCapabilities) -> ClientCapabilities {
    ClientCapabilities {
        elicitation: caps
            .elicitation
            .and_then(|v| serde_json::to_string(&v).ok()),
        experimental: caps.experimental.map(|exp| {
            exp.into_iter()
                .filter_map(|(k, v)| serde_json::to_string(&v).ok().map(|s| (k, s)))
                .collect()
        }),
        list_changed: caps.roots.and_then(|r| r.list_changed).and_then(|lc| {
            if lc {
                Some(ClientLists::ROOTS)
            } else {
                None
            }
        }),
        sampling: caps.sampling.and_then(|v| serde_json::to_string(&v).ok()),
    }
}

pub(crate) fn convert_implementation(impl_info: JsonImplementation) -> Implementation {
    Implementation {
        name: impl_info.name,
        title: impl_info.title,
        version: impl_info.version,
    }
}

/// Parse a JSON-RPC request ID
pub fn parse_request_id(value: &Value) -> Result<RequestId, String> {
    serde_json::from_value::<JsonRequestId>(value.clone())
        .map(RequestId::from)
        .map_err(|e| format!("Invalid request ID: {}", e))
}