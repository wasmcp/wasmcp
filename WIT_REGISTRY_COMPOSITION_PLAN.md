# WIT Registry and Composition Plan

## Current Issues

### 1. Missing Null Components for New Handlers

With our WIT updates adding `sampling-handler`, `elicitation-handler`, `roots-handler`, and `completion-handler`, the `mcp-server` world now imports these. However, we only have null components for `resources` and `prompts`. 

When composing with WAC, if a handler doesn't implement these new capabilities, we need null components to satisfy the server's imports. Otherwise composition will fail with unresolved imports.

### 2. WIT Package Registry Publication

The `Cargo-component.lock` shows it's fetching `fastertools:mcp@0.1.1` from a registry with digest `sha256:f98bdb33e3f192faf4f7435eb9e72f25104d5c59e02f7eca1ab45966e810eebd`. This is likely an older version before our updates.

For components to generate proper bindings with `cargo component`, they need access to our updated WIT definitions. This means we need to:
- Build and publish our WIT package using `wkg wit build` and `wkg publish`
- Configure wkg with a registry (local or remote)

### 3. Composition Pattern Evolution

The current compose.wac only wires up:
- core
- tool-handler  
- resource-handler (via null)
- prompt-handler (via null)

But misses the new handlers. The server will fail to compose without all its imports satisfied.

## Action Items

### Phase 1: Create Missing Null Components

Create 4 new null components under `/components/`:
- `null-sampling/` - Null implementation of sampling-handler
- `null-elicitation/` - Null implementation of elicitation-handler  
- `null-roots/` - Null implementation of roots-handler
- `null-completion/` - Null implementation of completion-handler

Each should follow the pattern of existing null components:
```rust
// Implement core interface (required)
impl core::Guest for Component { ... }

// Implement specific handler with empty/error responses
impl sampling_handler::Guest for Component { ... }
```

### Phase 2: Registry Setup

1. **Configure wkg registry**
   ```bash
   # Set up local registry for development
   wkg config --default-registry localhost:5000
   # Or use a public registry
   wkg config --default-registry ghcr.io/wasmcp
   ```

2. **Build and publish WIT package**
   ```bash
   cd /home/ian/Dev/wasmcp
   wkg wit build -o wit.wasm
   wkg publish wit.wasm --version 0.1.1
   ```

3. **Update component dependencies** to reference published WIT

### Phase 3: Update Composition Patterns

1. **Update example compose.wac files** to include all null components:
   ```wac
   let handler = new rust:handler { ... };
   let nullresources = new fastertools:null-resources { ... };
   let nullprompts = new fastertools:null-prompts { ... };
   let nullsampling = new fastertools:null-sampling { ... };
   let nullelicitation = new fastertools:null-elicitation { ... };
   let nullroots = new fastertools:null-roots { ... };
   let nullcompletion = new fastertools:null-completion { ... };

   let server = new fastertools:wasmcp-server {
       "fastertools:mcp/core@0.1.1": handler["fastertools:mcp/core@0.1.1"],
       "fastertools:mcp/tool-handler@0.1.1": handler["fastertools:mcp/tool-handler@0.1.1"],
       "fastertools:mcp/resource-handler@0.1.1": nullresources["fastertools:mcp/resource-handler@0.1.1"],
       "fastertools:mcp/prompt-handler@0.1.1": nullprompts["fastertools:mcp/prompt-handler@0.1.1"],
       "fastertools:mcp/sampling-handler@0.1.1": nullsampling["fastertools:mcp/sampling-handler@0.1.1"],
       "fastertools:mcp/elicitation-handler@0.1.1": nullelicitation["fastertools:mcp/elicitation-handler@0.1.1"],
       "fastertools:mcp/roots-handler@0.1.1": nullroots["fastertools:mcp/roots-handler@0.1.1"],
       "fastertools:mcp/completion-handler@0.1.1": nullcompletion["fastertools:mcp/completion-handler@0.1.1"],
       ...
   };
   ```

### Phase 4: Consider Meta-Components

Create a `null-all` component that exports all handler interfaces with null implementations for simpler composition:
```wac
let handler = new rust:handler { ... };
let nullall = new fastertools:null-all { ... };

// Could wire all null handlers from one component
let server = new fastertools:wasmcp-server {
    "fastertools:mcp/core@0.1.1": handler["fastertools:mcp/core@0.1.1"],
    "fastertools:mcp/tool-handler@0.1.1": handler["fastertools:mcp/tool-handler@0.1.1"],
    // All other handlers come from null-all
    ...
};
```

## Benefits

- **Type Safety**: Composition fails early if handlers are missing
- **Flexibility**: Mix and match real and null implementations
- **Discoverability**: Registry makes WIT definitions accessible to all tooling
- **Modularity**: Each handler can be developed independently

## Testing Strategy

1. Test each null component individually
2. Test various composition patterns
3. Ensure all handler combinations work
4. Validate registry publication and consumption