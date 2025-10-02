# WIT Interfaces for MCP

This package provides [WebAssembly Interface Types (WIT)](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md) definitions for the Model Context Protocol (MCP).

## Package Structure

- `world.wit` - World definitions and core MCP types
- `handler.wit` - Request handler interfaces and parameter types
- `writer.wit` - Streaming writers for MCP responses

## Usage Patterns

### Simple Tool Response

For basic text responses, use the one-shot static methods:

```rust
use wasmcp::mcp::tool_writer::content_writer;

fn handle(request: request, output: output_stream) -> result<_, stream_error> {
    // Simple text response
    content_writer::send_text(output, "Tool result", None)?;
    Ok(())
}
```

### Multiple Content Blocks

Send structured content in a single call:

```rust
let blocks = vec![
    content_block::text(text_content {
        text: "First result".into(),
        options: None,
    }),
    content_block::image(image_content {
        data: base64_data.into(),
        mime_type: "image/png",
        options: None,
    }),
];

content_writer::send_content(output, blocks, None)?;
```

### Streaming Large Responses

For incremental or large responses, use the streaming API:

```rust
// Open a streaming writer
let writer = content_writer::open(output, initial_block)?;

// Check available capacity before writing
while has_more_data {
    let capacity = writer.check_write()?;
    if capacity > 0 {
        let chunk = get_next_chunk(capacity);
        writer.write(chunk)?;
    }
}

writer.close(None)?;
```

### Non-blocking with Polling

For async operation, manage pollables from the output stream:

```rust
// Get pollable before passing stream to writer
let pollable = output.subscribe();
let writer = list_writer::open(output, initial_tools)?;

loop {
    // Wait for stream readiness
    wasi::io::poll::poll(&[&pollable]);

    let capacity = writer.check_write()?;
    if capacity > 0 {
        writer.write(next_tool)?;

        if done {
            writer.close(None)?;
            break;
        }
    }
}
```

## Error Handling

All writer methods return `result<_, stream_error>` following WASI conventions. The `error` interface provides a composable `operation_error` variant for unified error handling:

```rust
use wasmcp::mcp::error::operation_error;

match writer.write(data) {
    Ok(_) => continue,
    Err(e) => {
        return Err(operation_error::stream(e));
    }
}
```

## Backpressure

Writers implement `check_write` methods that return the number of items or bytes that can be written without blocking:

- When `check_write` returns 0, the stream needs to flush before accepting more writes
- Calling `write` with more data than permitted will trap (following WASI semantics)
- Use polling or blocking variants at the transport layer to wait for capacity

## Composition

The middleware pattern allows request interception and transformation:

```wit
world my_middleware {
    // Import the next handler in chain
    import incoming_handler;
    // Export our handler implementation
    export incoming_handler;
}
```

Handlers can be composed into chains, with each middleware able to:
- Inspect and modify requests
- Short-circuit with early responses
- Forward to the next handler
- Transform responses

## Transport Independence

Components implementing handlers are transport-agnostic. The same handler can be composed with different transports:

```bash
# Compose with HTTP transport
wac plug handler.wasm --plug wasmcp:mcp-transport-http

# Compose with stdio transport
wac plug handler.wasm --plug wasmcp:mcp-transport-stdio
```

## Version History

All interfaces are marked with `@since(version = 0.3.0-alpha.45)` annotations for version tracking. Future additions will include appropriate version annotations to maintain compatibility.