mod features;

use features::{tools, resources, prompts};

// Create the MCP handler using the SDK macro
wasmcp::create_handler! {
    tools: tools,
    resources: resources,
    prompts: prompts
}