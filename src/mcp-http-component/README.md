# MCP Gateway Component

This directory contains the source code for the MCP gateway component.

## Using the Published Gateway

Most users should use the pre-published gateway component:

```toml
[component.mcp-gateway]
source = { registry = "ghcr.io", package = "bowlofarugula:mcp-gateway", version = "0.1.0" }
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
   wkg oci push ghcr.io/yourusername/custom-mcp-gateway:0.1.0 \
     target/wasm32-wasip1/release/mcp_http_gateway.wasm
   ```
4. Use your custom gateway in `spin.toml`:
   ```toml
   [component.mcp-gateway]
   source = { registry = "ghcr.io", package = "yourusername:custom-mcp-gateway", version = "0.1.0" }
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