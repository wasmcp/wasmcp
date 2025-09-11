use wasmcp_core::runtime::{HttpClient, TimeProvider, CacheProvider, Logger};
use wasmcp_core::error::McpError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use async_trait::async_trait;

/// Native HTTP client using reqwest
#[derive(Clone)]
pub struct NativeHttpClient {
    client: reqwest::Client,
}

impl NativeHttpClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl HttpClient for NativeHttpClient {
    async fn get(&self, url: &str) -> Result<String, McpError> {
        self.client
            .get(url)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?
            .text()
            .await
            .map_err(|e| McpError::Http(e.to_string()))
    }
    
    async fn post(&self, url: &str, body: &str) -> Result<String, McpError> {
        self.client
            .post(url)
            .body(body.to_owned())
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?
            .text()
            .await
            .map_err(|e| McpError::Http(e.to_string()))
    }
}

/// Native time provider using chrono
#[derive(Clone, Copy)]
pub struct SystemTimeProvider;

impl TimeProvider for SystemTimeProvider {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
    
    fn unix_timestamp(&self) -> i64 {
        self.now().timestamp()
    }
}

/// In-memory cache for native (with TTL support)
#[derive(Clone)]
pub struct InMemoryCache {
    store: Arc<Mutex<HashMap<String, (Vec<u8>, Instant)>>>,
}

impl InMemoryCache {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl CacheProvider for InMemoryCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let store = self.store.lock().unwrap();
        store.get(key)
            .filter(|(_, expiry)| expiry > &Instant::now())
            .map(|(data, _)| data.clone())
    }
    
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<(), McpError> {
        let mut store = self.store.lock().unwrap();
        let expiry = Instant::now() + ttl;
        store.insert(key.to_string(), (value, expiry));
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> Result<(), McpError> {
        let mut store = self.store.lock().unwrap();
        store.remove(key);
        Ok(())
    }
}

/// Tracing-based logger for native
#[derive(Clone, Copy)]
pub struct TracingLogger;

impl Logger for TracingLogger {
    fn debug(&self, msg: &str) { 
        tracing::debug!("{}", msg); 
    }
    
    fn info(&self, msg: &str) { 
        tracing::info!("{}", msg); 
    }
    
    fn warn(&self, msg: &str) { 
        tracing::warn!("{}", msg); 
    }
    
    fn error(&self, msg: &str) { 
        tracing::error!("{}", msg); 
    }
}