//! Shared utility functions

/// Convert serde_json::Value to String for custom claims
///
/// Handles all JSON value types consistently across jwt.rs and introspection.rs
pub fn json_value_to_string(v: serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        _ => serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
    }
}
