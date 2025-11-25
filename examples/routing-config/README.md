# Routing Configuration with Filter Middleware

This example demonstrates advanced tool filtering patterns using the `filter-middleware` component. It shows how to implement path-based routing, tag-based filtering, hierarchical rules, and multi-config aggregation for fine-grained access control in MCP servers.

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

## Key Concepts

### Filter Middleware Architecture

Filter-middleware is a **server middleware component** that:

1. **Discovers** routing configurations via MCP resources capability
2. **Parses** TOML configuration files from resources
3. **Applies** filtering rules based on request path
4. **Delegates** filtered tool lists to downstream handlers
5. **Enforces** access control during `tools/call`

```
┌─────────────────────────────────────────────┐
│  Client Request: GET /mcp/math/addition    │
└──────────────────┬──────────────────────────┘
                   │
         ┌─────────▼─────────┐
         │ Filter Middleware │
         │                   │
         │  1. Match path    │
         │  2. Apply rules   │
         │  3. Filter tools  │
         └─────────┬─────────┘
                   │
         ┌─────────▼─────────┐
         │ Downstream Tools  │
         │ (calculator, todo)│
         └───────────────────┘
```

### Configuration Discovery

Filter-middleware automatically discovers routing configs using a **convention-based pattern**:

1. At startup, calls `resources/list` to discover all resources
2. Identifies routing configs with URIs matching `config://routing-*` or `routing://*`
3. Reads each config via `resources/read`
4. Merges all configs with conflict resolution rules

This allows multiple components to provide routing configs that get automatically aggregated.

## Routing Configuration Format

Routing rules are defined in TOML files exposed as MCP resources.

### Basic Structure

```toml
version = "1.0"

# Optional: Global tag filters (apply to ALL paths)
[tag-filters]
tool-level = "foundational"

# Path-specific rules
[path-rules."/mcp/math"]
whitelist = ["calculator-rs"]      # Allow entire component
blacklist = ["factorial"]          # Except this tool

[path-rules."/mcp/calc"]
whitelist = ["add", "subtract"]    # Allow specific tools

[path-rules."/mcp/math-only"]
tag-filters = { category = "math" }  # Filter by tag
```

### Path Rules

Each path can define:

| Field | Type | Description |
|-------|------|-------------|
| `whitelist` | Array | Component IDs or tool names to allow |
| `blacklist` | Array | Tool names to deny (overrides whitelist) |
| `tag-filters` | Object | Key-value pairs that tools must match |

**Important**: `whitelist` can contain:
- **Component IDs** (e.g., `"calculator-rs"`) - Allows all tools from that component
- **Tool names** (e.g., `"add"`, `"subtract"`) - Allows specific tools regardless of component

### Tag Filters

Tools can include metadata tags in their annotations. Filter-middleware supports filtering by these tags:

```rust
// Tool definition with tags
McpTool {
    name: "add".to_string(),
    // ...
    annotations: Some(ToolAnnotations {
        meta: Some(serde_json::json!({
            "component_id": "calculator-rs",
            "tags": {
                "category": "math",
                "tool-level": "foundational"
            }
        }))
    })
}
```

Tag filters use **AND logic** - tools must match ALL specified tags:

```toml
# Only tools with BOTH category=math AND tool-level=foundational
[path-rules."/mcp/foundational-math"]
tag-filters = { category = "math", tool-level = "foundational" }
```

## Filtering Patterns

### Pattern 1: Component-Level Filtering

**Use case**: Allow all tools from specific components

```toml
[path-rules."/mcp/math"]
whitelist = ["calculator-rs"]

[path-rules."/mcp/todo"]
whitelist = ["todo-list-auth"]
```

**Result**:
- `/mcp/math` → Only calculator tools (add, subtract, factorial)
- `/mcp/todo` → Only todo tools (add_item, list_items, remove_item, clear_all)

### Pattern 2: Tool-Level Filtering

**Use case**: Allow specific tools regardless of component

```toml
[path-rules."/mcp/calc"]
whitelist = ["add", "subtract", "factorial"]
```

**Result**:
- `/mcp/calc` → Only the three specified tools

### Pattern 3: Blacklisting

**Use case**: Allow component but exclude specific tools

```toml
[path-rules."/mcp/math"]
whitelist = ["calculator-rs"]
blacklist = ["factorial"]  # Deny trumps allow
```

**Result**:
- `/mcp/math` → add, subtract (factorial is blocked)

### Pattern 4: Hierarchical Paths

**Use case**: Override parent rules for specific paths

```toml
# Parent rule - allow calculator component
[path-rules."/mcp/math"]
whitelist = ["calculator-rs"]

# Child rule - only allow add (overrides parent)
[path-rules."/mcp/math/addition"]
whitelist = ["add"]
```

**Result**:
- `/mcp/math` → add, subtract, factorial
- `/mcp/math/addition` → add only

**Matching logic**: Longest matching path wins

### Pattern 5: Tag-Based Filtering

**Use case**: Filter by tool metadata categories

```toml
# Only math tools
[path-rules."/mcp/math-only"]
tag-filters = { category = "math" }

# Only foundational math tools (AND logic)
[path-rules."/mcp/foundational-math"]
tag-filters = { category = "math", tool-level = "foundational" }
```

**Result**:
- `/mcp/math-only` → All tools tagged with category=math
- `/mcp/foundational-math` → Tools with BOTH tags

### Pattern 6: Global Tag Filters

**Use case**: Apply baseline filtering to ALL paths

```toml
# Global filters affect every path
[tag-filters]
tool-level = "foundational"

# Path-specific rules still apply
[path-rules."/mcp/math"]
whitelist = ["calculator-rs"]
```

**Result**: All paths only show foundational-level tools, then path rules further refine

## Multi-Config Aggregation

Multiple routing configs can be provided by different components and merged automatically.

### Configuration Sources

This example exposes two configs from a single component:

```rust
// In routing-config/src/lib.rs
fn list_resources(...) -> Result<ListResourcesResult, ErrorCode> {
    Ok(ListResourcesResult {
        resources: vec![
            McpResource {
                uri: "routing://config".to_string(),
                // Base configuration
            },
            McpResource {
                uri: "config://routing-team-override".to_string(),
                // Override configuration
            },
        ],
    })
}
```

Filter-middleware discovers both and merges them.

### Merge Rules

When multiple configs define rules for the same path:

1. **Whitelists**: Union (combine all allowed tools/components)
2. **Blacklists**: Union (combine all denied tools)
3. **Tag filters**: Union (combine all required tags)
4. **Conflicts**: "Deny Trumps Allow" - blacklists override whitelists

### Example: Deny Trumps Allow

**Base config** (routing://config):
```toml
[path-rules."/mcp/math"]
whitelist = ["calculator-rs"]  # Includes add, subtract, factorial
```

**Override config** (config://routing-team-override):
```toml
[path-rules."/mcp/math"]
blacklist = ["add"]  # Deny add tool
```

**Effective rules**:
- Whitelist: calculator-rs component (includes add, subtract, factorial)
- Blacklist: add
- **Result**: Only subtract and factorial available (add is DENIED)

### Example: Whitelist Union

**Base config**:
```toml
[path-rules."/mcp/calc"]
whitelist = ["add", "subtract", "factorial"]
```

**Override config**:
```toml
[path-rules."/mcp/calc"]
whitelist = ["add_item", "list_items"]
```

**Effective rules**:
- **Result**: All 5 tools available (union of both whitelists)

## Diagnostic Tool

Filter-middleware adds an `inspect_routing` tool for debugging:

```bash
# Initialize session
curl -X POST http://localhost:3000/mcp \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize",...}'

# Call inspect_routing
curl -X POST http://localhost:3000/mcp \
  -H "Authorization: Bearer $TOKEN" \
  -H "Mcp-Session-Id: $SESSION_ID" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"inspect_routing"}}'
```

**Response includes**:

1. **Config sources**: List of discovered routing configs
2. **Effective rules**: Merged rules for each path
3. **Conflict reports**: Details of any Deny Trumps Allow scenarios

Example output:

```json
{
  "config_sources": [
    {"uri": "routing://config", "version": "1.0"},
    {"uri": "config://routing-team-override", "version": "1.0-override"}
  ],
  "effective_rules": {
    "/mcp/math": {
      "path": "/mcp/math",
      "whitelist": ["calculator-rs"],
      "blacklist": ["add", "factorial"],
      "sources": {
        "whitelist_from": ["routing://config"],
        "blacklist_from": ["routing://config", "config://routing-team-override"]
      }
    }
  },
  "conflict_reports": [
    {
      "path": "/mcp/math",
      "tool_or_component": "add",
      "conflict": "Tool 'add' is whitelisted via component 'calculator-rs' but blacklisted by [\"routing://config\", \"config://routing-team-override\"]",
      "resolution": "DENIED (blacklist wins per Deny Trumps Allow rule)"
    }
  ]
}
```

## Using These Patterns in Your Components

### Option 1: Expose Routing Config as Resource

Create a component that exposes routing configuration:

```rust
use bindings::exports::wasmcp::mcp_v20250618::resources::Guest;
use bindings::wasmcp::mcp_v20250618::mcp::*;

struct MyRoutingConfig;

impl Guest for MyRoutingConfig {
    fn list_resources(...) -> Result<ListResourcesResult, ErrorCode> {
        Ok(ListResourcesResult {
            resources: vec![McpResource {
                uri: "config://routing-myteam".to_string(),
                name: "My Team Routing Rules".to_string(),
                options: Some(ResourceOptions {
                    mime_type: Some("application/toml".to_string()),
                    // ...
                }),
            }],
        })
    }

    fn read_resource(...) -> Result<Option<ReadResourceResult>, ErrorCode> {
        if request.uri == "config://routing-myteam" {
            let config = include_str!("../routing.toml");
            Ok(Some(ReadResourceResult {
                contents: vec![ResourceContents::Text(TextResourceContents {
                    uri: request.uri,
                    text: TextData::Text(config.to_string()),
                    // ...
                })],
            }))
        } else {
            Ok(None)
        }
    }

    fn list_resource_templates(...) -> Result<ListResourceTemplatesResult, ErrorCode> {
        Ok(ListResourceTemplatesResult::default())
    }
}
```

Then compose with filter-middleware:

```bash
wasmcp compose server \
  filter_middleware.wasm \
  my_routing_config.wasm \
  my_tools.wasm \
  -o mcp-server.wasm
```

### Option 2: Add Tags to Your Tools

Add metadata tags to enable tag-based filtering:

```rust
fn list_tools(...) -> Result<ListToolsResult, ErrorCode> {
    Ok(ListToolsResult {
        tools: vec![McpTool {
            name: "my_tool".to_string(),
            description: Some("My tool description".to_string()),
            input_schema: /* ... */,
            annotations: Some(ToolAnnotations {
                meta: Some(serde_json::json!({
                    "component_id": "my-component",
                    "tags": {
                        "category": "analytics",
                        "tool-level": "advanced",
                        "data-source": "database"
                    }
                })),
            }),
        }],
    })
}
```

Then use tag filters in routing config:

```toml
[path-rules."/api/analytics"]
tag-filters = { category = "analytics" }

[path-rules."/api/database"]
tag-filters = { data-source = "database" }
```

### Option 3: Use filter-middleware CLI Template

Generate a routing-config component from template:

```bash
wasmcp new my-routing-rules --language rust --template-type routingconfig
cd my-routing-rules

# Edit routing.toml with your rules
vim routing.toml

# Build
make build

# Compose with filter-middleware
wasmcp compose server \
  /path/to/filter_middleware.wasm \
  target/wasm32-wasip2/release/my_routing_rules.wasm \
  /path/to/my_tools.wasm \
  -o mcp-server.wasm
```

## Building

```bash
# Install dependencies
make setup

# Update WIT dependencies
make wit

# Build routing-config component
make build

# Build all dependencies (filter-middleware, calculator-rs, todo-list-auth)
make build-deps

# Compose complete MCP server
make compose
# Creates: mcp-server.wasm (ready to run with Spin)
```

### What Gets Composed

The `make compose` target creates a complete MCP server by composing:

1. **filter-middleware.wasm** - Filtering logic
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
TOKEN=$(../../cli/target/aarch64-apple-darwin/release/wasmcp jwt load-token admin)
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

Configure Claude Code to use different paths for different contexts:

```json
{
  "mcpServers": {
    "math-tools": {
      "command": "spin",
      "args": ["up", "--listen", "127.0.0.1:3001"],
      "env": {
        "SPIN_HTTP_LISTEN_ADDR": "127.0.0.1:3001",
        "MCP_BASE_PATH": "/mcp/math"
      }
    },
    "todo-tools": {
      "command": "spin",
      "args": ["up", "--listen", "127.0.0.1:3002"],
      "env": {
        "SPIN_HTTP_LISTEN_ADDR": "127.0.0.1:3002",
        "MCP_BASE_PATH": "/mcp/todo"
      }
    }
  }
}
```

### With Multiple Environments

Use different routing configs for dev/staging/prod:

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

## Files

```
routing-config/
├── Cargo.toml                  # Rust package configuration
├── Makefile                    # Build targets
├── README.md                   # This file
├── routing.toml                # Base routing configuration
├── routing-override.toml       # Override config (demonstrates conflicts)
├── spin.toml                   # Spin runtime configuration
├── wit/
│   ├── deps/                   # WIT dependencies
│   ├── deps.lock
│   ├── deps.toml
│   └── world.wit              # Component world (exposes resources)
├── src/
│   └── lib.rs                 # Routing config provider implementation
└── scripts/
    └── test-filtering.sh      # Comprehensive test suite (14 scenarios)
```

## Performance Considerations

- **Config caching**: Filter-middleware caches parsed configs per session
- **Path matching**: Uses longest-prefix matching (O(log n) with path trie)
- **Tag filtering**: Pre-computed during `tools/list` (no per-call overhead)
- **Composition order**: Filter-middleware should be FIRST in composition chain

## Security Considerations

Filter-middleware provides **defense in depth** but is not a substitute for proper authorization:

- **Use with authorization**: Combine with JWT-based auth (see todo-list-auth example)
- **Path validation**: Ensure clients can't manipulate path headers
- **Config sources**: Validate routing config URIs match expected patterns
- **Tool metadata**: Verify tool tags are set by trusted component authors

For production:
- Use HTTPS for all communication
- Implement audit logging
- Monitor for unauthorized access attempts
- Regularly review routing configurations

## Related Examples

- **counter-middleware** - Basic middleware pattern and session storage
- **todo-list-auth** - JWT authorization patterns (SBAC, RBAC, ABAC)
- **calculator-rs** - Basic tools capability provider
- **filter-middleware** (crates/) - The middleware component source code

## Related Documentation

- [Filter Middleware Implementation](../../crates/filter-middleware/)
- [MCP Resources Capability](https://spec.modelcontextprotocol.io/capabilities/resources/)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
