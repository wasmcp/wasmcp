mod features;

use features::{tools, resources, prompts};

// Create the MCP handler using the SDK macro
ftl_sdk::create_handler! {
    tools: tools,
    resources: resources,
    prompts: prompts
}