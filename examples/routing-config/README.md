# Routing Configuration with Filter Middleware

This example demonstrates advanced tool filtering patterns using the [`filter-middleware`](../../crates/filter-middleware/README.md) component. It shows how to implement path-based routing, tag-based filtering, hierarchical rules, and multi-config aggregation for fine-grained visibility control in MCP servers.

## Overview

The routing-config example provides a complete reference implementation for:

- **Path-based filtering**: Control which tools are available at specific URL paths
- **Component-level filtering**: Whitelist/blacklist entire components
- **Tool-level filtering**: Whitelist/blacklist individual tools
- **Tag-based filtering**: Filter tools by metadata tags (category, level, etc.)
- **Hierarchical path matching**: Child paths override parent path rules
- **Multi-config aggregation**: Multiple routing configs merge with conflict resolution
- **Diagnostic inspection**: `inspect_routing` tool for debugging filter rules

This example composes four components:
1. **filter-middleware** - Intercepts requests and applies routing rules
2. **routing-config** - Exposes routing configurations as MCP resources
3. **calculator-rs** - Provides math tools (add, subtract, factorial)
4. **todo-list-auth** - Provides todo list tools (add_item, list_items, etc.)

## Quick Start

```bash
# Build all components and compose into MCP server
make work

# Generate JWT tokens for testing
../todo-list-auth/scripts/setup-test-env.sh

# Run comprehensive test suite (14 scenarios)
./scripts/test-filtering.sh
```

The test script demonstrates all filtering patterns with color-coded output showing which tools are available at each path.

## Building

```bash
# Ensures all dependencies are installed and composes the server
make work
```

### What Gets Composed

A wasmcp server is composed using the following components:

1. **filter-middleware.wasm** - Handles filtering logic
2. **routing_config.wasm** - Routing configuration provider
3. **calculator.wasm** - Math tools
4. **todo_list_auth.wasm** - Todo tools

Final output: `mcp-server.wasm` (4 components composed together)

## Testing

### Run All Test Scenarios

```bash
# Ensure JWT tokens exist
../todo-list-auth/scripts/setup-test-env.sh

# Run 14 test scenarios
./scripts/test-filtering.sh
```

### Test Scenarios

The test script validates all filtering patterns:

**Basic Patterns** (Scenarios 1-5):
1. No path rule - all tools available
2. Component whitelist with tool blacklist
3. Hierarchical path override
4. Component-level filtering
5. Tool-level whitelist

**Advanced Patterns** (Scenarios 6-9):
6. `tools/call` enforcement (blocked tools rejected)
7. Single tag filter (category=math)
8. Multiple tag filters (category + level)
9. Multi-config discovery via `inspect_routing`

**Multi-Config Scenarios** (Scenarios 10-14):
10. Deny Trumps Allow - blacklist overrides whitelist
11. Whitelist Union - merged tool lists
12. Global tag filters from override
13. New path from override config
14. Conflict detection and resolution

### Manual Testing

```bash
# Start server
make compose
spin up

# In another terminal, initialize session
TOKEN=$(wasmcp jwt load-token admin)
SESSION_ID=$(curl -s -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -D - \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0.0"}}}' \
  http://localhost:3000/mcp/math | grep -i mcp-session-id | cut -d' ' -f2 | tr -d '\r')

# List tools at /mcp/math path
curl -X POST \
  -H "Authorization: Bearer $TOKEN" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  http://localhost:3000/mcp/math

# Try different paths to see different tool sets
# /mcp           - All tools
# /mcp/math      - Calculator tools except factorial
# /mcp/calc      - Specific calculator tools + todo tools
# /mcp/todo      - Todo tools only
```

## Integration Examples

### With Claude Code

Ensure you have the server running: `spin up -f /path/to/spin.toml`

Example Claude Code config to use different paths for different contexts:

```json
{
  "mcpServers": {
    "all": {
      "type": "http",
      "url": "http://127.0.0.1:3000/mcp"
    },
    "math": {
      "type": "http",
      "url": "http://127.0.0.1:3000/mcp/math"
    },
    "todo": {
      "type": "http",
      "url": "http://127.0.0.1:3000/mcp/todo" 
    }
  }
}
```

### With Multiple Environments

You could have different configs that provide different behaviors based on target environment:

```bash
# Development - all tools available
wasmcp compose server \
  filter_middleware.wasm \
  routing_dev.wasm \
  all_tools.wasm \
  -o server-dev.wasm

# Production - restricted tools only
wasmcp compose server \
  filter_middleware.wasm \
  routing_prod.wasm \
  all_tools.wasm \
  -o server-prod.wasm
```

