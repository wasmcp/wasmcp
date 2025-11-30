# {{project_name}}

Routing configuration provider for filter-middleware in Rust.

This component provides path-based and tag-based routing rules that control which MCP tools are accessible at different HTTP paths. Works with `filter-middleware` to enable fine-grained access control.

## Quick Start

```bash
make setup  # Install wasm32-wasip2 target
make build  # Output: target/wasm32-wasip2/release/{{package_name}}.wasm
```

## Compose with Filter-Middleware

```bash
# Compose with filter-middleware and your tools
wasmcp compose server \
    path/to/filter_middleware.wasm \
    target/wasm32-wasip2/release/{{package_name}}.wasm \
    path/to/your_tools.wasm \
    -o server.wasm \
    --runtime spin
```

The CLI automatically:
1. Detects this is a resources component
2. Wraps it with resources-middleware
3. Places filter-middleware first to intercept tools/list and tools/call
4. Routes config discovery through to this component

## How It Works

### Resource URI Pattern

This component exposes: `config://routing-{{project_name}}`

filter-middleware discovers ALL routing configs matching `config://routing-*` pattern, enabling multi-config support.

### Multi-Config Aggregation

When multiple routing configs exist (e.g., base + team overrides):

- **Whitelists merge (union)** - Any config can allow tools
- **Blacklists merge (union)** - Any config can deny tools
- **Deny Trumps Allow** - Blacklist always wins over whitelist

Example:
```toml
# Base config: config://routing-base
[path-rules."/api"]
whitelist = ["admin-tools"]  # Includes "delete_user"

# Override config: config://routing-team
[path-rules."/api"]
blacklist = ["delete_user"]  # Deny specific tool

# Result: admin-tools allowed EXCEPT delete_user (blacklist wins)
```

### Routing Configuration Syntax

The `routing.toml` file defines filtering rules:

#### Path-Based Rules

```toml
# Component-level whitelist
[path-rules."/api/admin"]
whitelist = ["admin-component"]  # Allow entire component

# Tool-level whitelist
[path-rules."/api/public"]
whitelist = ["get_status", "health"]  # Specific tools only

# Blacklist
[path-rules."/api/restricted"]
blacklist = ["dangerous_tool"]  # Deny these tools
```

#### Tag-Based Filtering

```toml
# Single tag filter
[path-rules."/api/math"]
tag-filters = { category = "math" }

# Multiple tags (AND logic - must match ALL)
[path-rules."/api/core"]
tag-filters = { category = "utilities", tool-level = "foundational" }
```

#### Global Tag Filters

```toml
# Apply to ALL paths
[tag-filters]
tool-level = "foundational"
```

### Hierarchical Path Matching

Longest matching path wins:

```toml
[path-rules."/api"]
whitelist = ["all-tools"]

[path-rules."/api/admin"]
whitelist = ["admin-only"]  # More specific, overrides /api

[path-rules."/api/admin/readonly"]
whitelist = ["view_only"]  # Most specific, overrides /api/admin
```

Request to `/api/admin/readonly` matches the most specific rule.

## Tool Metadata Tags

Tags are defined in tool metadata (tool.options.meta):

```json
{
  "component_id": "my-component",
  "tags": {
    "category": "math",
    "tool-level": "foundational",
    "security-level": "public"
  }
}
```

Common tag patterns:
- **category**: math, utilities, admin, data, system
- **tool-level**: foundational, advanced, expert
- **security-level**: public, internal, admin
- **data-access**: read-only, read-write, admin

## Common Use Cases

### Public API with Restricted Admin Tools

```toml
# Public tools at root
[path-rules."/api"]
whitelist = ["public-component"]

# Admin path requires specific tools
[path-rules."/api/admin"]
whitelist = ["admin-component"]
blacklist = ["destructive_actions"]
```

### Environment-Based Filtering

Create multiple configs:
- `config://routing-production` - Strict rules
- `config://routing-development` - Permissive rules

Deploy the appropriate config per environment.

### Team-Based Overrides

```toml
# config://routing-base (deployed org-wide)
[path-rules."/api"]
whitelist = ["standard-tools"]

# config://routing-data-team (deployed for data team)
[path-rules."/api/data"]
whitelist = ["analytics-tools", "ml-tools"]
```

Teams get base rules + their overrides automatically.

## Diagnostic Tool

When composed with filter-middleware, an `inspect_routing` tool becomes available:

```bash
# Call from MCP client
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "inspect_routing",
    "arguments": {}
  }
}
```

Returns:
- All discovered config sources (URIs + versions)
- Effective rules per path (merged from all configs)
- Conflict reports (where tools are both whitelisted and blacklisted)

## Implementation Details

This component uses the **resources capability pattern**:

- `list_resources()` - Exposes config://routing-{{project_name}}
- `read_resource()` - Returns embedded routing.toml content
- `list_resource_templates()` - Returns empty (no templates)

The routing.toml file is embedded at compile time via `include_str!("../routing.toml")`, so changes require rebuild.

## Testing

To test your routing configuration:

1. Compose with filter-middleware and test tools
2. Use `inspect_routing` tool to verify effective rules
3. Test tool access at different paths
4. Check conflict reports for unintended blocks

Example test:
```bash
# Initialize session at path
curl -X POST http://localhost:3000/mcp \
  -H "Mcp-Session-Path: /api/admin" \
  -d '{"method":"initialize",...}'

# List available tools
curl -X POST http://localhost:3000/mcp \
  -d '{"method":"tools/list",...}'

# Try calling filtered tool (should fail)
curl -X POST http://localhost:3000/mcp \
  -d '{"method":"tools/call","params":{"name":"blocked_tool"}}'
```

## Customization

Edit `routing.toml` to define your rules:

1. Define path patterns matching your API structure
2. Specify whitelists for allowed components/tools
3. Add blacklists for denied tools
4. Configure tag filters for attribute-based filtering
5. Rebuild and redeploy

The component automatically exposes the updated configuration.

## Multi-Config Strategy

Benefits of multiple routing configs:

- **Separation of concerns** - Base rules + team/env overrides
- **Gradual rollout** - Add new configs without changing base
- **Team autonomy** - Teams manage their own routing rules
- **Environment parity** - Same base, different overrides per env

All configs are auto-discovered and merged at runtime.

## Learn More

- [filter-middleware documentation](../../crates/filter-middleware/README.md)
- [MCP specification](https://spec.modelcontextprotocol.io/)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
