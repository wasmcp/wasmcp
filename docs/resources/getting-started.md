# wasmcp Documentation Index

START HERE if you have no existing context about wasmcp. This resource routes you to the right documentation based on your task.

## Quick Decision: What Are You Doing?

**Contributing to wasmcp CLI itself?**
→ Install `wasmcp-developer` agent (see Agent Installation below)
→ Read: architecture, composition-modes, WIT interfaces
→ Check: `cli/CLAUDE.md` and root `CLAUDE.md` in repository

**Building MCP tools with wasmcp?**
→ Install `wasmcp-toolbuilder` agent (see Agent Installation below)
→ Start: building-servers (complete workflow)
→ Then: composition-modes (server vs handler)
→ Reference: reference (CLI syntax)

## Learning Paths

### Beginner (Never used wasmcp)
1. **building-servers** - Complete workflow from zero to deployed server
2. **reference** - CLI command syntax when you need it
3. **composition-modes** - Understanding server vs handler modes

### Intermediate (Built a few tools)
1. **registry** - Save component aliases and composition profiles
2. **composition-modes** - Multi-layer composition patterns
3. **reference** - Advanced CLI options (--version, --override-*, etc.)

### Advanced (Understanding internals)
1. **architecture** - Internal design, composition pipeline, handler interfaces
2. **composition-modes** - Detection priority, auto-wrapping logic
3. **WIT interfaces** - Protocol type definitions

## Task-Based Navigation

**"I want to create my first MCP tool"**
→ building-servers (covers: wasmcp new, build, compose, test)

**"I'm composing multiple tools and getting errors"**
→ composition-modes (error scenarios, troubleshooting)
→ reference (check command syntax)

**"I need to understand server vs handler mode"**
→ composition-modes (detailed comparison, when to use each)

**"I want to reuse components across projects"**
→ registry (aliases, profiles, OCI packages)

**"How do I specify components in compose?"**
→ reference (path vs OCI vs alias detection)

**"I need to understand how wasmcp works internally"**
→ architecture (capability/middleware pattern, composition pipeline)

**"What interfaces should my component export?"**
→ WIT interfaces (wasmcp:protocol/tools, resources, prompts)
→ building-servers (templates handle this automatically)

## Troubleshooting Decision Tree

**Error: "Component doesn't export server-handler"**
→ composition-modes (component types section)
→ Solution: Use `compose server` not `compose handler`

**Error: "instance does not have export wasmcp:protocol/tools"**
→ composition-modes (detection priority section)
→ Check: Component already wrapped/composed?

**Error: Version mismatch or missing dependencies**
→ reference (--version flag, --deps-dir)
→ registry (dependency management)

**Build failures**
→ building-servers (build commands for each language)
→ Check: Correct wasm32-wasip2 target?

**Composition succeeds but server doesn't work**
→ Verify: `wasm-tools component wit output.wasm`
→ Check: `wasm-tools validate output.wasm`
→ Test: `wasmtime serve -Scli output.wasm`

## Resource Type Guide

**Tutorial (Step-by-step):**
- building-servers

**Guide (Conceptual):**
- architecture
- composition-modes

**Reference (Syntax lookup):**
- reference
- WIT interfaces

**Utility (Configuration):**
- registry (config.toml format)
- Registry resources (JSON data)

## Agent Installation

wasmcp provides two specialized Claude agents for different workflows:

### wasmcp-developer (CLI Development)
**Purpose:** Contributing to wasmcp codebase
**Target:** Rust developers, CLI maintainers
**Specialization:** wac-graph, composition pipeline, MCP server implementation

**Install:**
1. Read agent config: `wasmcp://claude/agents/developer`
2. Download: `https://raw.githubusercontent.com/wasmcp/wasmcp/main/docs/claude/agents/wasmcp-developer.md`
3. Save to: `~/.claude/agents/wasmcp-developer.md`
4. Invoke in Claude: Use Task tool with `wasmcp-developer` agent

### wasmcp-toolbuilder (Tool Development)
**Purpose:** Building MCP tools with wasmcp
**Target:** MCP tool creators, users
**Specialization:** Component creation, composition, debugging, testing

**Install:**
1. Read agent config: `wasmcp://claude/agents/toolbuilder`
2. Download: `https://raw.githubusercontent.com/wasmcp/wasmcp/main/docs/claude/agents/wasmcp-toolbuilder.md`
3. Save to: `~/.claude/agents/wasmcp-toolbuilder.md`
4. Invoke in Claude: Use Task tool with `wasmcp-toolbuilder` agent

**Why use agents?**
- Context-optimized: Agents automatically read relevant resources
- Workflow-aware: Understand dev-server.sh, testing, composition patterns
- Cost-effective: toolbuilder uses sonnet model for guidance tasks

## Common Question Quick Answers

**Q: What's the difference between wasmcp and MCP?**
A: MCP is the protocol. wasmcp is a tool to build MCP servers using WebAssembly components. Read: architecture

**Q: Do I need to know Rust?**
A: No, templates work with Rust, Python, JS. Pick your language with `wasmcp new --language`. Read: building-servers

**Q: Can I use existing WASM components?**
A: Yes, any component exporting correct interfaces. Read: reference (component formats)

**Q: What's a "handler" vs a "server"?**
A: Handler = composable middleware. Server = runnable with transport. Read: composition-modes

**Q: How do I publish components for others?**
A: Push to OCI registry (ghcr.io, etc.). Others reference as `namespace:name@version`. Read: reference (OCI format)

**Q: Can I compose wasmcp servers together?**
A: No. Compose handlers into handlers, then final handler into server. Read: composition-modes (multi-layer)

**Q: Where's the config file?**
A: `~/.config/wasmcp/config.toml` - Read: registry

**Q: How do I debug composition issues?**
A: Use `-v` flag for verbose output. Read: composition-modes (troubleshooting)

## Next Steps

After reading this index:
1. Choose your path above (Beginner/Intermediate/Advanced)
2. Install the appropriate agent (developer vs toolbuilder)
3. Read the resources in suggested order
4. Use reference resource as syntax lookup tool
