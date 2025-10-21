//! Minimal notification-channel implementation for stdio transport
//!
//! Stdio transport currently doesn't support notifications as they would
//! interfere with the request/response JSON-RPC stream. This is a stub
//! implementation that satisfies the interface requirements.

use crate::bindings::exports::wasmcp::server::notifications::{
    Guest, GuestNotificationChannel, LogLevel, NotificationChannel, NotificationError,
    ProgressToken, ServerLists, ServerRequest, ServerSubscriptions,
};
use crate::bindings::wasi::io::streams::OutputStream;

pub struct StdioNotificationChannel {
    // For stdio, notifications are currently not supported
    // as they would interfere with the JSON-RPC stream
}

impl GuestNotificationChannel for StdioNotificationChannel {
    fn new(stream: OutputStream) -> Self {
        // Create a stub channel that discards the stream
        // In a real implementation, this might buffer notifications
        // or send them through a side channel
        drop(stream);
        StdioNotificationChannel {}
    }

    fn finish(channel: NotificationChannel) -> OutputStream {
        // Return a null stream since we don't actually use it
        // In a real implementation, this would return the original stream
        let _this: StdioNotificationChannel = channel.into_inner();
        // This is a hack - we create a new stderr stream as placeholder
        // The proper implementation would store and return the original stream
        crate::bindings::wasi::cli::stderr::get_stderr()
    }

    fn progress(
        &self,
        _token: ProgressToken,
        _progress: f64,
        _total: Option<f64>,
        _message: Option<String>,
    ) -> Result<(), NotificationError> {
        // Silently discard progress notifications
        Ok(())
    }

    fn log(
        &self,
        _message: String,
        _level: LogLevel,
        _logger: Option<String>,
    ) -> Result<(), NotificationError> {
        // Silently discard log notifications
        Ok(())
    }

    fn list_changed(&self, _changes: ServerLists) -> Result<(), NotificationError> {
        // Silently discard list change notifications
        Ok(())
    }

    fn updated(&self, _updates: ServerSubscriptions) -> Result<(), NotificationError> {
        // Silently discard subscription updates
        Ok(())
    }

    fn request(&self, _request: ServerRequest) -> Result<(), NotificationError> {
        // Silently discard server requests
        Ok(())
    }
}

impl Guest for StdioNotificationChannel {
    type NotificationChannel = StdioNotificationChannel;
}