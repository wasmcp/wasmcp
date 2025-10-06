//! Error types for OTLP serialization.
//!
//! This module provides structured error types that replace generic String errors
//! throughout the serialization pipeline. These errors are designed to map cleanly
//! to the WIT serialization-error variant exposed to component consumers.
//!
//! # Error Hierarchy
//!
//! All serialization errors implement the following conversions:
//! - `serde_json::Error` → `SerializationError::JsonEncoding`
//! - `prost::EncodeError` → `SerializationError::ProtobufEncoding`
//! - `SerializationError` → WIT `SerializationError` variant
//!
//! # Examples
//!
//! ```no_run
//! use otel_trace::error::SerializationError;
//!
//! // Errors automatically convert from underlying serialization libraries
//! fn serialize_json() -> Result<Vec<u8>, SerializationError> {
//!     let value = serde_json::json!({"key": "value"});
//!     let bytes = serde_json::to_vec(&value)?; // Automatically converts
//!     Ok(bytes)
//! }
//! ```

use thiserror::Error;
use crate::bindings::exports::wasi::otel_sdk::trace::SerializationError as WitSerializationError;

/// Serialization errors for OTLP encoding.
///
/// This enum provides structured error handling for the trace serialization pipeline,
/// replacing generic String errors with specific, actionable error variants.
///
/// Each variant maps to a specific failure mode:
/// - [`JsonEncoding`](Self::JsonEncoding): serde_json failed to serialize span data
/// - [`ProtobufEncoding`](Self::ProtobufEncoding): prost failed to encode protobuf message
/// - [`InvalidData`](Self::InvalidData): span data structure is malformed
///
/// # Design Notes
///
/// This error type was introduced in Issue #10 to replace `Result<T, String>` throughout
/// the serialization pipeline. It provides:
/// - Type-safe error handling with pattern matching
/// - Automatic conversion from underlying serialization libraries
/// - Clean mapping to WIT interface error variants
/// - Proper error context via `thiserror` derive macro
#[derive(Debug, Error)]
pub enum SerializationError {
    /// JSON serialization failed.
    ///
    /// This error occurs when `serde_json` cannot serialize span data to JSON format.
    /// Common causes include:
    /// - Invalid UTF-8 in string attributes
    /// - Circular references (should not occur with OTLP data)
    /// - Out of memory during large span batches
    #[error("JSON encoding failed: {0}")]
    JsonEncoding(String),

    /// Protobuf encoding failed.
    ///
    /// This error occurs when `prost` cannot encode span data to protobuf format.
    /// Common causes include:
    /// - Invalid field values (e.g., required fields missing)
    /// - Buffer overflow during encoding
    /// - Invalid protobuf structure
    #[error("Protobuf encoding failed: {0}")]
    ProtobufEncoding(String),

    /// Invalid span data structure.
    ///
    /// This error occurs when span data is malformed before serialization.
    /// Common causes include:
    /// - Invalid trace ID length (must be exactly 16 bytes)
    /// - Invalid span ID length (must be exactly 8 bytes)
    /// - Missing required fields
    #[error("Invalid span data: {0}")]
    InvalidData(String),
}

/// Converts internal Rust error to WIT-exported error variant.
///
/// This conversion is used at the component boundary to expose errors
/// to component consumers via the WIT interface.
impl From<SerializationError> for WitSerializationError {
    fn from(err: SerializationError) -> Self {
        match err {
            SerializationError::JsonEncoding(msg) => WitSerializationError::JsonEncoding(msg),
            SerializationError::ProtobufEncoding(msg) => WitSerializationError::ProtobufEncoding(msg),
            SerializationError::InvalidData(msg) => WitSerializationError::InvalidData(msg),
        }
    }
}

/// Automatically converts `serde_json` errors to `SerializationError`.
///
/// This allows using the `?` operator directly on JSON serialization calls:
/// ```no_run
/// # use otel_trace::error::SerializationError;
/// fn serialize() -> Result<Vec<u8>, SerializationError> {
///     let data = serde_json::json!({"key": "value"});
///     serde_json::to_vec(&data)?  // Auto-converts error
/// }
/// ```
impl From<serde_json::Error> for SerializationError {
    fn from(err: serde_json::Error) -> Self {
        SerializationError::JsonEncoding(err.to_string())
    }
}

/// Automatically converts `prost` encoding errors to `SerializationError`.
///
/// This allows using the `?` operator directly on protobuf encoding calls:
/// ```no_run
/// # use otel_trace::error::SerializationError;
/// # use prost::Message;
/// fn encode_proto<T: Message>(msg: &T) -> Result<Vec<u8>, SerializationError> {
///     let mut buf = Vec::new();
///     msg.encode(&mut buf)?;  // Auto-converts error
///     Ok(buf)
/// }
/// ```
impl From<prost::EncodeError> for SerializationError {
    fn from(err: prost::EncodeError) -> Self {
        SerializationError::ProtobufEncoding(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_encoding_error() {
        let err = SerializationError::JsonEncoding("test error".to_string());
        assert_eq!(err.to_string(), "JSON encoding failed: test error");
    }

    #[test]
    fn test_protobuf_encoding_error() {
        let err = SerializationError::ProtobufEncoding("test error".to_string());
        assert_eq!(err.to_string(), "Protobuf encoding failed: test error");
    }

    #[test]
    fn test_invalid_data_error() {
        let err = SerializationError::InvalidData("test error".to_string());
        assert_eq!(err.to_string(), "Invalid span data: test error");
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("{invalid json}")
            .unwrap_err();
        let err = SerializationError::from(json_err);
        assert!(matches!(err, SerializationError::JsonEncoding(_)));
        assert!(err.to_string().contains("JSON encoding failed"));
    }

    #[test]
    fn test_wit_error_conversion() {
        let err = SerializationError::JsonEncoding("test".to_string());
        let wit_err: WitSerializationError = err.into();
        assert!(matches!(wit_err, WitSerializationError::JsonEncoding(_)));

        let err = SerializationError::ProtobufEncoding("test".to_string());
        let wit_err: WitSerializationError = err.into();
        assert!(matches!(wit_err, WitSerializationError::ProtobufEncoding(_)));

        let err = SerializationError::InvalidData("test".to_string());
        let wit_err: WitSerializationError = err.into();
        assert!(matches!(wit_err, WitSerializationError::InvalidData(_)));
    }
}
