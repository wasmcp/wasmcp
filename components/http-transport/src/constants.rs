/// MCP protocol constants

pub mod methods {
    // Lifecycle methods
    pub const INITIALIZE: &str = "initialize";
    pub const INITIALIZED: &str = "notifications/initialized";
    pub const PING: &str = "ping";
    pub const SHUTDOWN: &str = "shutdown";
    
    // Tools methods
    pub const TOOLS_LIST: &str = "tools/list";
    pub const TOOLS_CALL: &str = "tools/call";
    
    // Resources methods
    #[cfg(feature = "resources")]
    pub const RESOURCES_LIST: &str = "resources/list";
    #[cfg(feature = "resources")]
    pub const RESOURCES_READ: &str = "resources/read";
    
    // Prompts methods
    #[cfg(feature = "prompts")]
    pub const PROMPTS_LIST: &str = "prompts/list";
    #[cfg(feature = "prompts")]
    pub const PROMPTS_GET: &str = "prompts/get";
    
    // Completion methods
    #[cfg(feature = "completion")]
    pub const COMPLETION_COMPLETE: &str = "completion/complete";
}

pub mod oauth {
    // OAuth 2.0 discovery endpoints
    pub const WELL_KNOWN_RESOURCE_METADATA: &str = "/.well-known/oauth-protected-resource";
    pub const WELL_KNOWN_SERVER_METADATA: &str = "/.well-known/oauth-authorization-server";
}