//! Minimal WASI SDK for MCP handlers
//! 
//! This provides just the essentials MCP handlers need:
//! - Async HTTP client for outbound requests
//! - Key-value storage (Spin only)
//! - Configuration access

pub mod http;
pub mod config;

#[cfg(feature = "keyvalue-draft2")]
pub mod keyvalue {
    //! Key-value storage using WASI KV interfaces (Spin only)
    
    use crate::wit::wasi::keyvalue0_2_0_draft2 as keyvalue;
    use anyhow::Result;
    
    /// Response from listing keys
    pub struct ListKeysResponse {
        pub keys: Vec<String>,
        pub cursor: Option<String>,
    }
    
    /// A key-value store
    pub struct Store {
        bucket: keyvalue::store::Bucket,
    }
    
    impl Store {
        /// Open a store by name
        pub fn open(name: &str) -> Result<Self> {
            let bucket = keyvalue::store::open(name)?;
            Ok(Self { bucket })
        }
        
        /// Get a value by key
        pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
            match self.bucket.get(key) {
                Ok(Some(value)) => Ok(Some(value)),
                Ok(None) => Ok(None),
                Err(e) => Err(anyhow::anyhow!("Failed to get key: {:?}", e)),
            }
        }
        
        /// Set a key-value pair
        pub fn set(&self, key: &str, value: &[u8]) -> Result<()> {
            self.bucket.set(key, value)?;
            Ok(())
        }
        
        /// Delete a key
        pub fn delete(&self, key: &str) -> Result<()> {
            self.bucket.delete(key)?;
            Ok(())
        }
        
        /// Check if a key exists
        pub fn exists(&self, key: &str) -> Result<bool> {
            self.bucket.exists(key)
                .map_err(|e| anyhow::anyhow!("Failed to check key existence: {:?}", e))
        }
        
        /// List all keys (Spin-compatible version)
        pub fn list_keys(&self, cursor: Option<&str>) -> Result<ListKeysResponse> {
            let response = self.bucket.list_keys(cursor)
                .map_err(|e| anyhow::anyhow!("Failed to list keys: {:?}", e))?;
            Ok(ListKeysResponse {
                keys: response.keys,
                cursor: response.cursor,
            })
        }
    }
}

// Re-export spin_executor for blocking on async operations
pub use spin_executor;

// Generate WASI bindings
#[doc(hidden)]
pub mod wit {
    #![allow(missing_docs)]
    #![allow(warnings)]
    
    #[cfg(feature = "keyvalue-draft2")]
    wit_bindgen::generate!({
        world: "mcp-wasi-spin",
        path: "./wit",
        with: {
            "wasi:io/error@0.2.0": ::wasi::io::error,
            "wasi:io/streams@0.2.0": ::wasi::io::streams,
            "wasi:io/poll@0.2.0": ::wasi::io::poll,
        },
        generate_all,
    });
    
    #[cfg(not(feature = "keyvalue-draft2"))]
    wit_bindgen::generate!({
        world: "mcp-wasi",
        path: "./wit",
        with: {
            "wasi:io/error@0.2.0": ::wasi::io::error,
            "wasi:io/streams@0.2.0": ::wasi::io::streams,
            "wasi:io/poll@0.2.0": ::wasi::io::poll,
        },
        generate_all,
    });
}
