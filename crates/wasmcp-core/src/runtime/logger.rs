/// Logger abstraction for platform-agnostic logging
/// Allows for different implementations (console, file, tracing, etc.)
pub trait Logger: Send + Sync {
    /// Log a debug message
    fn debug(&self, msg: &str);
    
    /// Log an info message
    fn info(&self, msg: &str);
    
    /// Log a warning message
    fn warn(&self, msg: &str);
    
    /// Log an error message
    fn error(&self, msg: &str);
}