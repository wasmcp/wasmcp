//! Key-value storage using WASI KV interfaces

use crate::wit::wasi::keyvalue::store;
use anyhow::Result;

/// A key-value store
pub struct Store {
    bucket: store::Bucket,
}

impl Store {
    /// Open a store by name
    pub fn open(name: &str) -> Result<Self> {
        let bucket = store::open(name)?;
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
    
    /// List all keys
    pub fn list_keys(&self, cursor: Option<&str>) -> Result<store::KeyResponse> {
        self.bucket.list_keys(cursor)
            .map_err(|e| anyhow::anyhow!("Failed to list keys: {:?}", e))
    }
}