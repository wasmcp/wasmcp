# filter-middleware

MCP server middleware that filters tools based on HTTP path and metadata tags.

## Purpose

Routes different tool sets to different URL paths and filters tools by metadata tags. Supports multiple routing configurations that aggregate with "Deny Trumps Allow" semantics.

## Features

**Path-based filtering**: Whitelist or blacklist tools for specific URL paths
**Tag-based filtering**: Filter tools by metadata tags with AND logic
**Multi-config discovery**: Automatically discovers and aggregates `config://routing-*` resources
**Conflict detection**: Diagnostic tool shows effective rules and identifies conflicts

## Usage

### Routing Config Format

TOML files exposed as `config://routing-*` resources with `application/toml` MIME type:

```toml
version = "1.0"

# Global tag filters (apply to all paths)
[global-tag-filters]
category = "math"  # Only tools with tags.category = "math"

# Path-specific rules
[path-rules."/mcp/calculator"]
whitelist = ["calculator-rs", "add"]  # Allow by component ID or tool name
blacklist = ["dangerous"]              # Deny specific tools
tag-filters = { tool-level = "basic" } # Additional tag requirements
```

### Filtering Rules

1. **Path matching**: Longest matching path wins (`/mcp/calculator/advanced` matches `/mcp/calculator` over `/mcp`)
2. **Whitelist**: If present, only listed tools/components pass
3. **Blacklist**: Always denies listed tools, even if whitelisted ("Deny Trumps Allow")
4. **Tag filters**: Tool must match ALL specified tag values (AND logic)

### Multi-Config Aggregation

When multiple configs define rules for the same path:
- **Whitelists**: Union (tool allowed if in ANY whitelist)
- **Blacklists**: Union (tool denied if in ANY blacklist)
- **Tag filters**: Union of values per tag name
- **Deny Trumps Allow**: Blacklists always win

### Diagnostic Tool

The `inspect_routing` tool shows:
- Active configuration sources
- Effective rules per path
- Conflicts (tools both whitelisted and blacklisted)

## Architecture

**Optimized pipeline**:
1. Load and aggregate all routing configs on each `tools/list` request
2. Find most specific path rule for current HTTP path
3. Parse tool metadata once, cache for reuse
4. Apply path filters (whitelist → blacklist)
5. Apply tag filters (global + path-specific)
6. Cache filtered tool list in session storage

**Modules**:
- `lib.rs` - Handler and request routing
- `config.rs` - Config discovery and aggregation
- `filtering.rs` - Filtering pipeline with metadata caching
- `metadata.rs` - Tool metadata parsing and predicates
- `session.rs` - Session storage integration
- `diagnostic.rs` - `inspect_routing` tool implementation
- `types.rs` - Data structures
- `helpers.rs` - Utility functions

## Implementation Notes

- Config loading happens per request (no caching - WASM stateless)
- Metadata parsing is cached during single request pipeline
- Session storage used to persist filtered tool list across requests
- Missing session gracefully allows unfiltered tool calls

## Building

```bash
make build
# or
cargo build --target wasm32-wasip2 --release
```

Output: `target/wasm32-wasip2/release/filter_middleware.wasm`

## Testing

Tests run on native target (WASM test runner has limitations):

```bash
cargo test --lib --target aarch64-apple-darwin
# or your platform target (x86_64-apple-darwin, x86_64-unknown-linux-gnu, etc.)
```

**Test Coverage**: 23 unit tests covering:
- Path matching (longest prefix algorithm)
- Tag filter matching (AND logic)
- Config aggregation (whitelist/blacklist union, deny trumps allow)
- Tool metadata parsing
- Whitelist/blacklist checking
