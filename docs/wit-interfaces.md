# WIT Interface Reference

This document provides an overview of wasmcp's WebAssembly Interface Type (WIT) definitions. For full interface details, see the actual WIT files in the repository.

## Interface Organization

wasmcp's WIT interfaces are organized into two main packages:

### Protocol Package (`wit/protocol/`)

Core MCP protocol types and capabilities that components export.

**Files:**
- **`mcp.wit`** - Complete MCP protocol type definitions
  - JSON-RPC 2.0 message types
  - Request/response structures
  - Error handling types
  - Full MCP specification mapping

- **`features.wit`** - MCP feature capability interfaces
  - `tools-capability` - Tool listing and execution
  - `resources-capability` - Resource listing and reading
  - `prompts-capability` - Prompt listing and retrieval
  - Component capability exports

**Location:** `wit/protocol/`

These interfaces define what your components should export to provide MCP features.

### Server Package (`wit/server/`)

Server-side interfaces for building wasmcp framework components (transports, middleware).

**Files:**
- **`handler.wit`** - Core request handling interface
  - `handle(request) -> response` pattern
  - Used by all middleware and transport components

- **`sessions.wit`** - Session management
  - Session lifecycle (initialize, terminate)
  - Context propagation
  - State management across requests

- **`notifications.wit`** - Server-to-client notifications
  - Progress reporting
  - Log messages
  - Resource updates
  - Notification dispatch system

**Location:** `wit/server/`

These interfaces are for building wasmcp framework components, not user components.

## For Component Developers

If you're building MCP components with `wasmcp new`, you'll primarily work with:

### Protocol Interfaces (Building Capabilities)

**Export these interfaces** to provide MCP features:

```wit
// From wit/protocol/features.wit

// For tool components:
export tools-capability

// For resource components:
export resources-capability

// For prompt components:
export prompts-capability
```

Your component exports one or more of these interfaces, and the CLI automatically wraps it with the appropriate middleware during composition.

**Key WIT files to reference:**
- `wit/protocol/features.wit` - Capability interfaces you'll export
- `wit/protocol/mcp.wit` - Type definitions for MCP protocol structures

### Generated Template Code

When you run `wasmcp new my-component --language rust`, the generated code includes:

1. WIT bindings for the capability interface
2. Stub implementation of the capability methods
3. Example tool/resource/prompt implementations

The templates handle all the WIT binding generation automatically.

## For Framework Developers

If you're building custom middleware or transport components:

### Server Interfaces (Building Framework Components)

**Import and export** the handler interface to participate in the composition chain:

```wit
// From wit/server/handler.wit

// Your middleware imports from upstream:
import handle: func(request: string) -> result<string, error>

// And exports for downstream:
export handle: func(request: string) -> result<string, error>
```

This creates the chain of responsibility pattern.

**Key WIT files to reference:**
- `wit/server/handler.wit` - Core handler interface for middleware
- `wit/server/sessions.wit` - Session management (if your middleware needs state)
- `wit/server/notifications.wit` - Sending notifications to clients

### Composition Pattern

```
Transport (exports handler)
    ↓ imports handler from
Middleware 1 (imports + exports handler)
    ↓ imports handler from
Middleware 2 (imports + exports handler)
    ↓ imports handler from
...
    ↓ imports handler from
Method-Not-Found (imports handler)
```

Each component in the chain can:
1. Handle requests it understands
2. Pass others to the next handler via import
3. Merge results when appropriate

## Viewing WIT Files

All WIT interfaces are in the repository:

```bash
# Protocol package (for component developers)
wit/protocol/mcp.wit          # Full MCP type definitions
wit/protocol/features.wit     # Capability interfaces
wit/protocol/deps.toml        # Protocol dependencies
wit/protocol/deps.lock        # Locked dependency versions

# Server package (for framework developers)
wit/server/handler.wit        # Request handler interface
wit/server/sessions.wit       # Session management
wit/server/notifications.wit  # Notification dispatch
wit/server/deps.toml          # Server dependencies
wit/server/deps.lock          # Locked dependency versions
```

### MCP Resources

Access WIT files via MCP resources:

- `wasmcp://wit/protocol/mcp` - Full MCP protocol types
- `wasmcp://wit/protocol/features` - Capability interfaces
- `wasmcp://wit/server/handler` - Handler interface
- `wasmcp://wit/server/sessions` - Session management
- `wasmcp://wit/server/notifications` - Notifications

## Component Model Basics

If you're new to WebAssembly components and WIT:

### What is WIT?

WIT (WebAssembly Interface Types) is an IDL (Interface Definition Language) for defining component interfaces. It's like protocol buffers or OpenAPI, but for WebAssembly components.

### Key Concepts

**Interfaces:** Groups of related functions
```wit
interface tools-capability {
  list-tools: func() -> result<tools-list, error>
  call-tool: func(name: string, arguments: string) -> result<call-result, error>
}
```

**Exports:** What your component provides
```wit
export tools-capability
```

**Imports:** What your component needs
```wit
import handle: func(request: string) -> result<string, error>
```

**Worlds:** Complete component contracts (what's imported + exported)
```wit
world my-component {
  export tools-capability
}
```

### Language Bindings

WIT definitions generate idiomatic bindings for each language:

- **Rust:** Uses `wit-bindgen` to generate Rust traits and types
- **Python:** Uses `componentize-py` to generate Python classes
- **TypeScript:** Uses `jco` to generate TypeScript interfaces

The `wasmcp new` templates handle all binding generation automatically.

## Additional Resources

- [Component Model Specification](https://component-model.bytecodealliance.org/)
- [WIT Documentation](https://component-model.bytecodealliance.org/design/wit.html)
- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [wasmcp Architecture](./architecture.md) - How WIT interfaces compose into servers

## Quick Reference

| You want to... | Reference these files |
|----------------|----------------------|
| Build a tool component | `wit/protocol/features.wit` (tools-capability) |
| Build a resource component | `wit/protocol/features.wit` (resources-capability) |
| Build a prompt component | `wit/protocol/features.wit` (prompts-capability) |
| Understand MCP types | `wit/protocol/mcp.wit` |
| Build custom middleware | `wit/server/handler.wit` |
| Add session support | `wit/server/sessions.wit` |
| Send notifications | `wit/server/notifications.wit` |
| See complete examples | `examples/` directory |

---

**Remember:** For most component development, you don't need to write WIT directly. Use `wasmcp new` to generate templates with working bindings, then implement your business logic in your preferred language.
