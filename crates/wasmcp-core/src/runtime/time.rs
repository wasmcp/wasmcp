/// Time provider abstraction for getting current time
/// Allows for deterministic testing and platform-specific implementations
pub trait TimeProvider: Send + Sync {
    /// Get the current UTC time
    fn now(&self) -> chrono::DateTime<chrono::Utc>;
    
    /// Get the current Unix timestamp in seconds
    fn unix_timestamp(&self) -> i64;
}