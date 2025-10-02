# {{ project_name }}

A middleware component for the Model Context Protocol (MCP).

This middleware intercepts all requests and logs them before forwarding to the next handler in the chain. Middleware components can be used for:
- Logging and monitoring
- Authentication and authorization
- Request enrichment
- Rate limiting
- Caching

## Building

```bash
make build
```

## Usage

Compose with other handlers:

```bash
wasmcp compose \
  --middleware target/{{ project_name }}.wasm \
  --tools path/to/tools.wasm \
  --resources path/to/resources.wasm
```

The middleware will process all requests in the order specified on the command line.
