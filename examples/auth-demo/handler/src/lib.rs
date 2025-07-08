use wasmcp::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Serialize, Deserialize)]
struct EchoArgs {
    message: String,
}

/// Simple echo tool for testing
struct EchoTool;

impl ToolHandler for EchoTool {
    const NAME: &'static str = "echo";
    const DESCRIPTION: &'static str = "Echoes back the provided message";

    fn input_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Message to echo back"
                }
            },
            "required": ["message"]
        })
    }

    fn execute(args: Value) -> Result<String, String> {
        let args: EchoArgs = serde_json::from_value(args)
            .map_err(|e| format!("Invalid arguments: {}", e))?;
        
        Ok(format!("Echo: {}", args.message))
    }
}

/// User info tool that shows auth details
struct UserInfoTool;

impl ToolHandler for UserInfoTool {
    const NAME: &'static str = "user_info";
    const DESCRIPTION: &'static str = "Returns information about the authenticated user";

    fn input_schema() -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    fn execute(_args: Value) -> Result<String, String> {
        // In a real implementation, this would access user context from auth
        Ok("Authenticated user information would appear here".to_string())
    }
}

/// Simple resource for testing
struct ReadmeResource;

impl ResourceHandler for ReadmeResource {
    const URI: &'static str = "file:///readme";
    const NAME: &'static str = "README";
    const DESCRIPTION: Option<&'static str> = Some("Example readme resource");
    const MIME_TYPE: Option<&'static str> = Some("text/plain");

    fn read() -> Result<String, String> {
        Ok("This is a demo MCP handler with authentication enabled!".to_string())
    }
}

/// Arguments for the welcome prompt
struct WelcomeArgs;

impl PromptArguments for WelcomeArgs {
    fn schema() -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "name",
                description: Some("User's name"),
                required: true,
            }
        ]
    }
}

/// Welcome prompt
struct WelcomePrompt;

impl PromptHandler for WelcomePrompt {
    const NAME: &'static str = "welcome";
    const DESCRIPTION: Option<&'static str> = Some("Generate a welcome message");
    
    type Arguments = WelcomeArgs;

    fn resolve(args: Value) -> Result<Vec<PromptMessage>, String> {
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .ok_or("Name is required")?;
        
        Ok(vec![
            PromptMessage {
                role: PromptRole::Assistant,
                content: format!("Welcome to the authenticated MCP demo, {}!", name),
            }
        ])
    }
}

// Generate the handler implementation
create_handler! {
    tools: [EchoTool, UserInfoTool],
    resources: [ReadmeResource],
    prompts: [WelcomePrompt]
}