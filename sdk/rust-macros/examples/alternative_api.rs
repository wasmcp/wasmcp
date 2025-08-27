// Alternative API designs for consideration:

// Option 1: Explicit registration (no magic, very clear)
use wasmcp_macros::mcp_component;

#[mcp_component]
struct MyComponent;

impl MyComponent {
    #[tool]
    fn echo(message: String) -> Result<String, String> {
        Ok(format!("Echo: {}", message))
    }
    
    #[tool]
    async fn get_weather(location: String) -> Result<String, String> {
        // implementation
    }
}

// Option 2: Manual registration (most control, least magic)
wasmcp_macros::register_tools! {
    echo(message: String) -> Result<String, String>,
    get_weather(location: String) -> Result<String, String>,
}

fn echo(message: String) -> Result<String, String> {
    Ok(format!("Echo: {}", message))
}

async fn get_weather(location: String) -> Result<String, String> {
    // implementation
}

// Option 3: File-level attribute (requires nightly)
#![mcp_component]

#[tool]
fn echo(message: String) -> Result<String, String> {
    Ok(format!("Echo: {}", message))
}

#[tool] 
async fn get_weather(location: String) -> Result<String, String> {
    // implementation
}

// Option 4: Keep the module approach but make it cleaner
#[mcp_component]
mod tools {
    pub use super::*;  // Access other imports
    
    #[tool]
    pub fn echo(message: String) -> Result<String, String> {
        Ok(format!("Echo: {}", message))
    }
}