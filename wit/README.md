# MCP Component Interface

This directory contains the WebAssembly Interface Types (WIT) definitions for the Model Context Protocol (MCP) component model.

## Package Structure

- `mcp.wit` - Defines the MCP handler interface and the `mcp-handler` world that components must implement

## WIT File Synchronization

The `mcp-http-component` needs a copy of the handler interface without the package declaration. This is handled automatically by:

```bash
make sync-wit
```

This command is run automatically during CI builds to ensure the files stay in sync.

## Using the Interface

### In Rust Components

When creating a Rust component that implements the MCP handler interface:

1. Reference this WIT package in your `Cargo.toml`:

```toml
[package.metadata.component.target.dependencies]
"component:mcp" = { path = "../path/to/wit" }
```

2. Use `cargo-component` to generate bindings and implement the handler.

### In JavaScript/TypeScript Components

When creating a JavaScript component:

1. Copy or reference the WIT files in your project
2. Use `jco` to generate TypeScript types:
   ```bash
   jco types ./wit/mcp.wit -o generated
   ```
3. Use `jco componentize` to build your component

## Interface Overview

The MCP handler interface provides:
- **Tools**: Functions that can be called with arguments
- **Resources**: URIs that can be read to provide content
- **Prompts**: Templates that can be resolved with arguments

Components implementing this interface can be composed with the `mcp-http-component` gateway to expose MCP functionality over HTTP.