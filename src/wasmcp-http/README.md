# wasmcp HTTP Component

This directory contains the source code for the wasmcp HTTP gateway component that bridges HTTP requests to MCP handlers.

## Using the Published Gateway

Most users should use the pre-published gateway component:

```toml
[component.wasmcp-http]
source = { registry = "ghcr.io", package = "fastertools:wasmcp-http", version = "0.0.1" }
```

## Building a Custom Gateway

If you need to customize the gateway behavior:

1. Make your modifications to the source code
2. Build the component:
   ```bash
   cargo component build --release
   ```
3. Publish to your registry:
   ```bash
   wkg oci push ghcr.io/fastertools/wasmcp-http:0.0.1 \
     target/wasm32-wasip1/release/wasmcp_http.wasm
   ```
4. Use your custom gateway in `spin.toml`:
   ```toml
   [component.wasmcp-http]
   source = { registry = "ghcr.io", package = "fastertools:wasmcp-http", version = "0.0.1" }
   ```

## Gateway Features

The gateway handles:
- JSON-RPC 2.0 protocol
- MCP protocol compliance (version 2025-03-26)
- HTTP request/response handling
- Error handling and logging
- Tool calls, resource operations, and prompts

## Development

To work on the gateway:

```bash
# Build
cargo component build

# Test with a local handler
# Create a test spin.toml that uses the local gateway build
```