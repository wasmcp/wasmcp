# FTL Components

WebAssembly components for building and deploying MCP (Model Context Protocol) servers.

## Overview

This repository provides:

1. **wit** - Wasm Interface Types defining the MCP component interfaces
2. **mcp-http-component** - A WebAssembly component that exposes an MCP server over Streamable HTTP, delegating business logic to a MCP handler component.
3. **ftl-sdk-rust** - SDK for building MCP handler components in Rust
4. **ftl-sdk-typescript** - SDK for building MCP handler components in TypeScript/JavaScript

## Architecture

```
┌─────────────────┐         ┌────────────────────┐
│   HTTP Client   │ ──────> │ mcp-http-component │
└─────────────────┘         └────────────────────┘
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

## Development

### Prerequisites

- Rust toolchain with `wasm32-wasip1` target
- Node.js 20+
- cargo-binstall (for faster tool installation)

### Common Commands

```bash
# Show all available commands
make help

# Install dependencies and tools
make install-deps

# Build everything
make build-all

# Run all tests
make test-all

# Full CI pipeline (build + test)
make ci
```

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

#### Automated Release (Recommended)

```bash
# For patch release (bug fixes)
make release-patch

# For minor release (new features)
make release-minor

# Then follow the instructions printed by make
```

#### Manual Publishing

If you need to publish packages manually:

```bash
# Dry run first to verify
make publish-dry-run

# Publish individual packages
make publish-gateway      # Publish to ghcr.io
make publish-rust-sdk    # Publish to crates.io  
make publish-ts-sdk      # Publish to npm

# Or publish everything at once (use with caution!)
make publish-all
```

**Note**: The GitHub Actions workflow handles publishing automatically when you push version tags. Manual publishing is only needed for special cases.

For more details, see [Version Management Scripts](./scripts/README.md).

## Documentation

- [WIT Interface Documentation](./wit/README.md)
- [Rust SDK Documentation](./src/ftl-sdk-rust/README.md)
- [TypeScript SDK Documentation](./src/ftl-sdk-typescript/README.md)
- [HTTP Gateway Documentation](./src/mcp-http-component/README.md)