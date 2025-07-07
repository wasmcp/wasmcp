# FTL Components

WebAssembly components for building and deploying MCP (Model Context Protocol) servers.

## Overview

This repository provides:

1. **mcp-http-component** - A WebAssembly gateway component that exposes MCP handlers over HTTP
2. **ftl-sdk-rust** - SDK for building MCP handler components in Rust
3. **ftl-sdk-typescript** - SDK for building MCP handler components in TypeScript/JavaScript
4. **wit** - WebAssembly Interface Types defining the MCP component interfaces

## Architecture

```
┌─────────────────┐         ┌──────────────────┐
│   HTTP Client   │ ──────> │ mcp-http-gateway │
└─────────────────┘         └──────────────────┘
                                     │
                                     │ imports
                                     ▼
                            ┌──────────────────┐
                            │   MCP Handler    │
                            │   (Your Code)    │
                            └──────────────────┘
```

The mcp-http-component acts as a gateway that:
- Receives HTTP requests following the MCP protocol
- Calls into your MCP handler component
- Returns responses over HTTP

## Quick Start

### Building a Rust MCP Handler

1. Create a new component:
   ```bash
   cargo component new my-handler --lib
   cd my-handler
   ```

2. Add the SDK and configure WIT dependencies in `Cargo.toml`:
   ```toml
   [dependencies]
   ftl-sdk = "0.2.1"
   
   [package.metadata.component.target.dependencies]
   "component:mcp" = { path = "../ftl-components/wit" }
   ```

3. Implement your handler using the SDK types
4. Build: `cargo component build --release`

### Building a TypeScript MCP Handler

1. Set up your project:
   ```bash
   npm init -y
   npm install @fastertools/ftl-sdk
   npm install -D @bytecodealliance/jco
   ```

2. Copy the WIT files to your project
3. Implement your handler using the SDK
4. Build with jco: `jco componentize ...`

## Repository Structure

```
ftl-components/
├── wit/                    # Shared WIT interface definitions
│   ├── mcp.wit            # MCP handler interface
│   └── world.wit          # Component world definitions
├── src/
│   ├── mcp-http-component/ # HTTP gateway component
│   ├── ftl-sdk-rust/      # Rust SDK
│   └── ftl-sdk-typescript/ # TypeScript SDK
└── examples/              # Example implementations
```

## Version Management

This repository uses a centralized version management system. All component versions are managed through `versions.toml`.

### Quick Commands

```bash
# Show current versions
make show-versions

# Bump all packages by patch version (e.g., 0.1.2 → 0.1.3)
make bump-all-patch

# Bump all packages by minor version (e.g., 0.1.2 → 0.2.0)
make bump-all-minor

# Bump individual packages
make bump-rust-patch      # Bump Rust SDK patch version
make bump-gateway-minor   # Bump HTTP gateway minor version
make bump-ts-patch       # Bump TypeScript SDK patch version

# Ensure versions are in sync
make sync-versions
```

### How It Works

1. **Single Source of Truth**: `versions.toml` contains all version information
2. **Automatic Propagation**: Version changes are automatically synced to:
   - Package files (Cargo.toml, package.json)
   - Template dependencies
   - Gateway component references
   - Documentation

3. **CI Validation**: Pull requests are checked to ensure version consistency

### Release Process

1. Bump version: `make bump-all-patch`
2. Review changes: `git diff`
3. Commit: `git commit -am "chore: bump versions"`
4. Create tags:
   ```bash
   git tag mcp-http-component-v0.1.3
   git tag ftl-sdk-rust-v0.2.3
   git tag ftl-sdk-typescript-v0.1.3
   ```
5. Push: `git push origin main --tags`

For more details, see [Version Management Scripts](./scripts/README.md).

## Documentation

- [WIT Interface Documentation](./wit/README.md)
- [Rust SDK Documentation](./src/ftl-sdk-rust/README.md)
- [TypeScript SDK Documentation](./src/ftl-sdk-typescript/README.md)
- [HTTP Gateway Documentation](./src/mcp-http-component/README.md)