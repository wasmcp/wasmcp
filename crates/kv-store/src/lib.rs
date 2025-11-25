//! Key-Value Store Component
//!
//! Provides a typed abstraction over wasi:keyvalue that:
//! - Abstracts away version differences (draft vs draft2)
//! - Stores type metadata with each value for runtime type safety
//! - Provides both generic and typed convenience methods
//!
//! This component is dual-published to support both wasi:keyvalue draft and draft2.

#[cfg(feature = "draft2")]
mod bindings {
    wit_bindgen::generate!({
        path: "wit-draft2",
        world: "kv-store-draft2",
        generate_all,
    });
}

#[cfg(not(feature = "draft2"))]
mod bindings {
    wit_bindgen::generate!({
        world: "kv-store",
        generate_all,
    });
}

use bindings::exports::wasmcp::keyvalue::store::{
    Bucket, Error, Guest, GuestBucket, KeyResponse, TypedValue,
};
use bindings::wasi::keyvalue::{atomics, batch, store as wasi_kv};

// ============================================================================
// Type Tag Constants
// ============================================================================

const TAG_STRING: u8 = 0x01;
const TAG_JSON: u8 = 0x02;
const TAG_U64: u8 = 0x03;
const TAG_S64: u8 = 0x04;
const TAG_BOOL: u8 = 0x05;
const TAG_BYTES: u8 = 0x06;

// ============================================================================
// Encoding/Decoding Helpers
// ============================================================================

/// Encode a typed value into bytes with type tag prefix
fn encode_typed_value(value: &TypedValue) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();

    match value {
        TypedValue::AsString(s) => {
            bytes.push(TAG_STRING);
            bytes.extend_from_slice(s.as_bytes());
        }
        TypedValue::AsJson(json) => {
            // Validate JSON syntax
            validate_json(json)?;
            bytes.push(TAG_JSON);
            bytes.extend_from_slice(json.as_bytes());
        }
        TypedValue::AsU64(n) => {
            bytes.push(TAG_U64);
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        TypedValue::AsS64(n) => {
            bytes.push(TAG_S64);
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        TypedValue::AsBool(b) => {
            bytes.push(TAG_BOOL);
            bytes.push(if *b { 1 } else { 0 });
        }
        TypedValue::AsBytes(data) => {
            bytes.push(TAG_BYTES);
            bytes.extend_from_slice(data);
        }
    }

    Ok(bytes)
}

/// Decode bytes with type tag prefix into a typed value
fn decode_typed_value(bytes: &[u8]) -> Result<TypedValue, Error> {
    if bytes.is_empty() {
        return Err(Error::Other("Empty value, missing type tag".to_string()));
    }

    let tag = bytes[0];
    let payload = &bytes[1..];

    match tag {
        TAG_STRING => {
            let s = std::str::from_utf8(payload)
                .map_err(|e| Error::Other(format!("Invalid UTF-8 in string value: {}", e)))?;
            Ok(TypedValue::AsString(s.to_string()))
        }
        TAG_JSON => {
            let s = std::str::from_utf8(payload)
                .map_err(|e| Error::Other(format!("Invalid UTF-8 in JSON value: {}", e)))?;
            Ok(TypedValue::AsJson(s.to_string()))
        }
        TAG_U64 => {
            if payload.len() != 8 {
                return Err(Error::Other(format!(
                    "Invalid u64 value length: expected 8 bytes, got {}",
                    payload.len()
                )));
            }
            let n = u64::from_le_bytes(
                payload
                    .try_into()
                    .map_err(|_| Error::Other("Failed to parse u64 bytes".to_string()))?,
            );
            Ok(TypedValue::AsU64(n))
        }
        TAG_S64 => {
            if payload.len() != 8 {
                return Err(Error::Other(format!(
                    "Invalid s64 value length: expected 8 bytes, got {}",
                    payload.len()
                )));
            }
            let n = i64::from_le_bytes(
                payload
                    .try_into()
                    .map_err(|_| Error::Other("Failed to parse i64 bytes".to_string()))?,
            );
            Ok(TypedValue::AsS64(n))
        }
        TAG_BOOL => {
            if payload.len() != 1 {
                return Err(Error::Other(format!(
                    "Invalid bool value length: expected 1 byte, got {}",
                    payload.len()
                )));
            }
            let b = payload[0] != 0;
            Ok(TypedValue::AsBool(b))
        }
        TAG_BYTES => Ok(TypedValue::AsBytes(payload.to_vec())),
        _ => Err(Error::Other(format!("Unknown type tag: 0x{:02x}", tag))),
    }
}

/// Validate JSON syntax
fn validate_json(json: &str) -> Result<(), Error> {
    // Simple validation: try to parse as serde_json::Value
    serde_json::from_str::<serde_json::Value>(json)
        .map_err(|e| Error::Other(format!("Invalid JSON syntax: {}", e)))?;
    Ok(())
}

/// Expect a specific type tag, return error if mismatch
fn expect_type(value: &TypedValue, expected_tag: u8) -> Result<(), Error> {
    let actual_tag = match value {
        TypedValue::AsString(_) => TAG_STRING,
        TypedValue::AsJson(_) => TAG_JSON,
        TypedValue::AsU64(_) => TAG_U64,
        TypedValue::AsS64(_) => TAG_S64,
        TypedValue::AsBool(_) => TAG_BOOL,
        TypedValue::AsBytes(_) => TAG_BYTES,
    };

    if actual_tag != expected_tag {
        let expected_name = type_tag_name(expected_tag);
        let actual_name = type_tag_name(actual_tag);
        return Err(Error::Other(format!(
            "Type mismatch: expected {}, got {}",
            expected_name, actual_name
        )));
    }

    Ok(())
}

fn type_tag_name(tag: u8) -> &'static str {
    match tag {
        TAG_STRING => "string",
        TAG_JSON => "json",
        TAG_U64 => "u64",
        TAG_S64 => "s64",
        TAG_BOOL => "bool",
        TAG_BYTES => "bytes",
        _ => "unknown",
    }
}

// ============================================================================
// Component Implementation
// ============================================================================

struct Component;

impl Guest for Component {
    type Bucket = BucketImpl;

    fn open(identifier: String) -> Result<Bucket, Error> {
        // Use WASMCP_SESSION_BUCKET if identifier is empty
        let bucket_name = if identifier.is_empty() {
            use bindings::wasi::cli::environment::get_environment;
            let env_vars = get_environment();
            env_vars
                .iter()
                .find(|(k, _)| k == "WASMCP_SESSION_BUCKET")
                .map(|(_, v)| v.clone())
                .unwrap_or_else(|| "default".to_string())
        } else {
            identifier
        };

        let bucket = wasi_kv::open(&bucket_name).map_err(convert_error)?;

        Ok(Bucket::new(BucketImpl { inner: bucket }))
    }
}

/// Bucket implementation wrapping wasi:keyvalue bucket
struct BucketImpl {
    inner: wasi_kv::Bucket,
}

impl GuestBucket for BucketImpl {
    // ========== Generic API ==========

    fn get(&self, key: String) -> Result<Option<TypedValue>, Error> {
        match self.inner.get(&key).map_err(convert_error)? {
            Some(bytes) => {
                let value = decode_typed_value(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    fn set(&self, key: String, value: TypedValue) -> Result<(), Error> {
        let bytes = encode_typed_value(&value)?;
        self.inner.set(&key, &bytes).map_err(convert_error)
    }

    // ========== Typed Convenience API ==========

    fn get_json(&self, key: String) -> Result<Option<String>, Error> {
        match self.get(key)? {
            Some(value) => {
                expect_type(&value, TAG_JSON)?;
                match value {
                    TypedValue::AsJson(json) => Ok(Some(json)),
                    _ => Err(Error::Other(
                        "Type mismatch after validation (should never happen)".to_string(),
                    )),
                }
            }
            None => Ok(None),
        }
    }

    fn set_json(&self, key: String, json: String) -> Result<(), Error> {
        self.set(key, TypedValue::AsJson(json))
    }

    fn get_string(&self, key: String) -> Result<Option<String>, Error> {
        match self.get(key)? {
            Some(value) => {
                expect_type(&value, TAG_STRING)?;
                match value {
                    TypedValue::AsString(s) => Ok(Some(s)),
                    _ => Err(Error::Other(
                        "Type mismatch after validation (should never happen)".to_string(),
                    )),
                }
            }
            None => Ok(None),
        }
    }

    fn set_string(&self, key: String, value: String) -> Result<(), Error> {
        self.set(key, TypedValue::AsString(value))
    }

    fn get_u64(&self, key: String) -> Result<Option<u64>, Error> {
        match self.get(key)? {
            Some(value) => {
                expect_type(&value, TAG_U64)?;
                match value {
                    TypedValue::AsU64(n) => Ok(Some(n)),
                    _ => Err(Error::Other(
                        "Type mismatch after validation (should never happen)".to_string(),
                    )),
                }
            }
            None => Ok(None),
        }
    }

    fn set_u64(&self, key: String, value: u64) -> Result<(), Error> {
        self.set(key, TypedValue::AsU64(value))
    }

    fn get_s64(&self, key: String) -> Result<Option<i64>, Error> {
        match self.get(key)? {
            Some(value) => {
                expect_type(&value, TAG_S64)?;
                match value {
                    TypedValue::AsS64(n) => Ok(Some(n)),
                    _ => Err(Error::Other(
                        "Type mismatch after validation (should never happen)".to_string(),
                    )),
                }
            }
            None => Ok(None),
        }
    }

    fn set_s64(&self, key: String, value: i64) -> Result<(), Error> {
        self.set(key, TypedValue::AsS64(value))
    }

    fn get_bool(&self, key: String) -> Result<Option<bool>, Error> {
        match self.get(key)? {
            Some(value) => {
                expect_type(&value, TAG_BOOL)?;
                match value {
                    TypedValue::AsBool(b) => Ok(Some(b)),
                    _ => Err(Error::Other(
                        "Type mismatch after validation (should never happen)".to_string(),
                    )),
                }
            }
            None => Ok(None),
        }
    }

    fn set_bool(&self, key: String, value: bool) -> Result<(), Error> {
        self.set(key, TypedValue::AsBool(value))
    }

    fn get_bytes(&self, key: String) -> Result<Option<Vec<u8>>, Error> {
        match self.get(key)? {
            Some(value) => {
                expect_type(&value, TAG_BYTES)?;
                match value {
                    TypedValue::AsBytes(bytes) => Ok(Some(bytes)),
                    _ => Err(Error::Other(
                        "Type mismatch after validation (should never happen)".to_string(),
                    )),
                }
            }
            None => Ok(None),
        }
    }

    fn set_bytes(&self, key: String, value: Vec<u8>) -> Result<(), Error> {
        self.set(key, TypedValue::AsBytes(value))
    }

    // ========== Batch Operations ==========

    fn get_many(&self, keys: Vec<String>) -> Result<Vec<Option<(String, TypedValue)>>, Error> {
        let results = batch::get_many(&self.inner, &keys).map_err(convert_error)?;

        #[cfg(feature = "draft2")]
        {
            // Spin's batch returns: list<tuple<string, option<list<u8>>>>
            results
                .into_iter()
                .map(|(key, opt_bytes)| {
                    if let Some(ref bytes) = opt_bytes {
                        let typed_value = decode_typed_value(bytes)?;
                        Ok(Some((key, typed_value)))
                    } else {
                        Ok(None)
                    }
                })
                .collect()
        }

        #[cfg(not(feature = "draft2"))]
        {
            // Official draft returns: list<option<tuple<string, list<u8>>>>
            results
                .into_iter()
                .map(|opt| match opt {
                    Some((key, bytes)) => {
                        let typed_value = decode_typed_value(&bytes)?;
                        Ok(Some((key, typed_value)))
                    }
                    None => Ok(None),
                })
                .collect()
        }
    }

    fn set_many(&self, pairs: Vec<(String, TypedValue)>) -> Result<(), Error> {
        let encoded_pairs: Result<Vec<(String, Vec<u8>)>, Error> = pairs
            .iter()
            .map(|(key, value)| {
                let bytes = encode_typed_value(value)?;
                Ok((key.clone(), bytes))
            })
            .collect();

        let encoded_pairs = encoded_pairs?;
        batch::set_many(&self.inner, &encoded_pairs).map_err(convert_error)
    }

    fn delete_many(&self, keys: Vec<String>) -> Result<(), Error> {
        batch::delete_many(&self.inner, &keys).map_err(convert_error)
    }

    // ========== Common Operations ==========

    fn delete(&self, key: String) -> Result<(), Error> {
        self.inner.delete(&key).map_err(convert_error)
    }

    fn exists(&self, key: String) -> Result<bool, Error> {
        self.inner.exists(&key).map_err(convert_error)
    }

    fn list_keys(&self, cursor: Option<String>) -> Result<KeyResponse, Error> {
        // Convert string cursor to u64 for draft version (draft2 uses string)
        #[cfg(not(feature = "draft2"))]
        let cursor_param = cursor.as_ref().and_then(|s| s.parse::<u64>().ok());

        #[cfg(feature = "draft2")]
        let cursor_param = cursor.as_deref();

        let response = self.inner.list_keys(cursor_param).map_err(convert_error)?;

        // Convert response cursor back to string
        #[cfg(not(feature = "draft2"))]
        let cursor_result = response.cursor.map(|n| n.to_string());

        #[cfg(feature = "draft2")]
        let cursor_result = response.cursor.map(|s| s.to_string());

        Ok(KeyResponse {
            keys: response.keys,
            cursor: cursor_result,
        })
    }

    // ========== Atomic Operations ==========

    fn increment(&self, key: String, delta: i64) -> Result<i64, Error> {
        // Draft version uses u64, draft2 uses s64
        #[cfg(not(feature = "draft2"))]
        let result = {
            let delta_u64 = if delta < 0 {
                return Err(Error::Other(format!(
                    "Draft version does not support negative deltas: {}",
                    delta
                )));
            } else {
                delta as u64
            };
            atomics::increment(&self.inner, &key, delta_u64)
                .map(|v| v as i64)
                .map_err(convert_error)?
        };

        #[cfg(feature = "draft2")]
        let result = atomics::increment(&self.inner, &key, delta).map_err(convert_error)?;

        Ok(result)
    }
}

/// Convert wasi:keyvalue error to our Error type
fn convert_error(e: wasi_kv::Error) -> Error {
    match e {
        wasi_kv::Error::NoSuchStore => Error::NoSuchStore,
        wasi_kv::Error::AccessDenied => Error::AccessDenied,
        wasi_kv::Error::Other(msg) => Error::Other(msg),
    }
}

bindings::export!(Component with_types_in bindings);
