//! {{project_name}} Prompts Capability Provider
//!
//! A prompts capability that provides example prompt templates.

mod bindings {
    wit_bindgen::generate!({
        world: "{{project_name}}",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::prompts::Guest;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_handler::MessageContext;

struct ExamplePrompts;

impl Guest for ExamplePrompts {
    fn list_prompts(
        _ctx: MessageContext,
        _request: ListPromptsRequest,
    ) -> Result<ListPromptsResult, ErrorCode> {
        Ok(ListPromptsResult {
            prompts: vec![
                Prompt {
                    name: "code-review".to_string(),
                    options: Some(PromptOptions {
                        meta: None,
                        arguments: Some(vec![
                            PromptArgument {
                                name: "language".to_string(),
                                description: Some("Programming language (e.g., python, rust, typescript)".to_string()),
                                required: Some(true),
                                title: Some("Language".to_string()),
                            },
                            PromptArgument {
                                name: "code".to_string(),
                                description: Some("Code to review".to_string()),
                                required: Some(true),
                                title: Some("Code".to_string()),
                            },
                        ]),
                        description: Some("Review code for best practices and potential issues".to_string()),
                        title: Some("Code Review".to_string()),
                    }),
                },
                Prompt {
                    name: "greeting".to_string(),
                    options: Some(PromptOptions {
                        meta: None,
                        arguments: Some(vec![
                            PromptArgument {
                                name: "name".to_string(),
                                description: Some("Name to greet".to_string()),
                                required: Some(false),
                                title: Some("Name".to_string()),
                            },
                        ]),
                        description: Some("Generate a friendly greeting".to_string()),
                        title: Some("Greeting".to_string()),
                    }),
                },
            ],
            next_cursor: None,
            meta: None,
        })
    }

    fn get_prompt(
        _ctx: MessageContext,
        request: GetPromptRequest,
    ) -> Result<Option<GetPromptResult>, ErrorCode> {
        match request.name.as_str() {
            "code-review" => {
                // Parse arguments (simplified - would use serde_json in real implementation)
                let args: serde_json::Value = request
                    .arguments
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();

                let language = args.get("language")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let code = args.get("code")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                Ok(Some(GetPromptResult {
                    meta: None,
                    description: Some(format!("Code review for {}", language)),
                    messages: vec![
                        PromptMessage {
                            role: Role::User,
                            content: ContentBlock::Text(TextContent {
                                text: TextData::Text(format!(
                                    "Please review this {} code for best practices, potential bugs, and suggest improvements:\n\n{}",
                                    language, code
                                )),
                                options: None,
                            }),
                        },
                    ],
                }))
            }
            "greeting" => {
                let args: serde_json::Value = request
                    .arguments
                    .as_ref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();

                let name = args.get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("there");

                Ok(Some(GetPromptResult {
                    meta: None,
                    description: Some("A friendly greeting".to_string()),
                    messages: vec![
                        PromptMessage {
                            role: Role::User,
                            content: ContentBlock::Text(TextContent {
                                text: TextData::Text(format!(
                                    "Greet {} in a friendly and welcoming way.",
                                    name
                                )),
                                options: None,
                            }),
                        },
                    ],
                }))
            }
            _ => Ok(None), // We don't handle this prompt
        }
    }
}

bindings::export!(ExamplePrompts with_types_in bindings);
