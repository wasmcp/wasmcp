use ftl_sdk::{create_tool, create_resource, create_prompt, json, Tool, Resource, Prompt, PromptMessage, PromptRole};

// Define your tools
pub fn tools() -> Vec<Tool> {
    vec![
        create_tool(
            "echo",
            "Echo a message back to the user",
            json!({
                "type": "object",
                "properties": {
                    "message": { 
                        "type": "string", 
                        "description": "Message to echo back" 
                    }
                },
                "required": ["message"]
            }),
            |args| {
                let message = args["message"].as_str().unwrap_or("Hello, world!");
                Ok(format!("Echo: {}", message))
            }
        ),
        // Add more tools here
    ]
}

// Define your resources
pub fn resources() -> Vec<Resource> {
    vec![
        // Example:
        // create_resource(
        //     "file:///example.txt",
        //     "Example File",
        //     || {
        //         Ok("File contents here".to_string())
        //     }
        // )
        // .with_description("An example text file")
        // .with_mime_type("text/plain"),
    ]
}

// Define your prompts
pub fn prompts() -> Vec<Prompt> {
    vec![
        // Example:
        // create_prompt(
        //     "greeting",
        //     |args| {
        //         let name = args["name"].as_str().unwrap_or("User");
        //         Ok(vec![
        //             PromptMessage {
        //                 role: PromptRole::User,
        //                 content: format!("Please greet {}", name),
        //             },
        //             PromptMessage {
        //                 role: PromptRole::Assistant,
        //                 content: format!("Hello, {}! How can I help you today?", name),
        //             },
        //         ])
        //     }
        // )
        // .with_description("Generate a greeting message")
        // .with_argument("name", "Name to greet", true),
    ]
}