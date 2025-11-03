//! Key-Value Store Component
//!
//! Provides a thin abstraction over wasi:keyvalue that allows components
//! to use KV storage without coupling to specific wasi:keyvalue versions.
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

use bindings::exports::wasmcp::storage::kv::{Guest, GuestBucket, Bucket};
use bindings::wasi::keyvalue::store as wasi_kv;

struct Component;

impl Guest for Component {
    type Bucket = BucketImpl;

    fn open(identifier: String) -> Result<Bucket, String> {
        let bucket = wasi_kv::open(&identifier)
            .map_err(|e| format!("Failed to open bucket '{}': {:?}", identifier, e))?;

        Ok(Bucket::new(BucketImpl { inner: bucket }))
    }
}

/// Bucket implementation wrapping wasi:keyvalue bucket
struct BucketImpl {
    inner: wasi_kv::Bucket,
}

impl GuestBucket for BucketImpl {
    fn get(&self, key: String) -> Result<Option<Vec<u8>>, String> {
        self.inner
            .get(&key)
            .map_err(|e| format!("Get failed for key '{}': {:?}", key, e))
    }

    fn set(&self, key: String, value: Vec<u8>) -> Result<(), String> {
        self.inner
            .set(&key, &value)
            .map_err(|e| format!("Set failed for key '{}': {:?}", key, e))
    }

    fn delete(&self, key: String) -> Result<(), String> {
        self.inner
            .delete(&key)
            .map_err(|e| format!("Delete failed for key '{}': {:?}", key, e))
    }

    fn exists(&self, key: String) -> Result<bool, String> {
        self.inner
            .exists(&key)
            .map_err(|e| format!("Exists check failed for key '{}': {:?}", key, e))
    }
}

bindings::export!(Component with_types_in bindings);
