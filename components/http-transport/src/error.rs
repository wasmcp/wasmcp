/// Re-export error types from wasmcp-core for consistency
pub use wasmcp_core::{ErrorCode, McpError};


/// Extension trait for converting ErrorCode to numeric codes
pub trait ErrorCodeExt {
    fn to_code(&self) -> i32;
}

impl ErrorCodeExt for ErrorCode {
    fn to_code(&self) -> i32 {
        match self {
            ErrorCode::ParseError => -32700,
            ErrorCode::InvalidRequest => -32600,
            ErrorCode::MethodNotFound => -32601,
            ErrorCode::InvalidParams => -32602,
            ErrorCode::InternalError => -32603,
            ErrorCode::ResourceNotFound => -32001,
            ErrorCode::ToolNotFound => -32002,
            ErrorCode::PromptNotFound => -32003,
            ErrorCode::Unauthorized => -32005,
            ErrorCode::RateLimited => -32006,
            ErrorCode::Timeout => -32007,
            ErrorCode::Cancelled => -32008,
            ErrorCode::CustomCode(code) => *code,
        }
    }
}

