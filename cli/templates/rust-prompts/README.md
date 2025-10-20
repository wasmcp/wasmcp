# {{project_name}}

MCP prompts capability component in Rust.

## Build

```bash
make setup  # Install wasm32-wasip2 target
make build  # Output: target/wasm32-wasip2/release/{{package_name}}.wasm
```

## Compose

```bash
wasmcp compose target/wasm32-wasip2/release/{{package_name}}.wasm -o server.wasm
```

The CLI automatically detects this is a prompts-capability component and wraps it with prompts-middleware.

## Run

```bash
# HTTP
wasmtime serve -Scli server.wasm

# Stdio
wasmcp compose target/wasm32-wasip2/release/{{package_name}}.wasm -t stdio -o server.wasm
wasmtime run server.wasm
```

## Implementation

This component uses the **capability pattern**, implementing two methods from the `prompts-capability` interface:

- `list_prompts()` - Returns all prompts this component provides
- `get_prompt()` - Returns prompt content by name, or `None` if not handled

See `src/lib.rs` for example prompts demonstrating:
- Prompt definitions with names and arguments
- Dynamic prompt generation based on arguments
- No protocol handling or delegation code

The prompts-middleware automatically handles:
- MCP protocol translation
- Merging prompts from multiple components
- Request delegation to downstream components
- Error handling and response formatting

## Adding Prompts

To add new prompts:

1. Add a `Prompt` entry to the vec in `list_prompts()`:

```rust
Prompt {
    name: "my-prompt".to_string(),
    options: Some(PromptOptions {
        meta: None,
        arguments: Some(vec![
            PromptArgument {
                name: "arg1".to_string(),
                description: Some("First argument".to_string()),
                required: Some(true),
                title: Some("Argument 1".to_string()),
            },
        ]),
        description: Some("Description of my prompt".to_string()),
        title: Some("My Prompt".to_string()),
    }),
}
```

2. Add a match arm in `get_prompt()`:

```rust
"my-prompt" => {
    let args: serde_json::Value = request
        .arguments
        .as_ref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();

    let arg1 = args.get("arg1")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    Some(GetPromptResult {
        meta: None,
        description: Some("My prompt description".to_string()),
        messages: vec![
            PromptMessage {
                role: Role::User,
                content: ContentBlock::Text(TextContent {
                    text: TextData::Text(format!(
                        "Your prompt text using {}",
                        arg1
                    )),
                    options: None,
                }),
            },
        ],
    })
}
```

3. That's it! No need to handle merging, delegation, or protocol details - the middleware does that for you!
