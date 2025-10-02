# Hello World Examples

Simple MCP servers - one echo tool in four languages.

## What's Here

Each subdirectory contains the same echo tool implemented in a different language:

- **python/** - Python with componentize-py (~36MB server)
- **rust/** - Rust with cargo-component (~220KB server)
- **typescript/** - TypeScript with jco (~11MB server)
- **go/** - Go with TinyGo (~340KB server)

All implement identical functionality - pick your preferred language.

## Quick Start

```bash
# Pick a language
cd python/  # or rust/, typescript/, go/

# Build and run (HTTP transport)
make run

# Test it
make server-test

# Or build and run with stdio transport
make run-stdio

# Test stdio server
make server-test-stdio
```

Server runs on `http://0.0.0.0:8080` (HTTP) or stdio (stdio transport)

## What Each Does

Single tool: **echo**
- Input: `{"message": "Hello"}`
- Output: `"Echo: Hello"`

## Comparing Implementations

All four use the same WIT interface but different language bindings:

| Language   | Build Tool      | Binary Size (unoptimized)
|------------|----------------|-------------
| Python     | componentize-py | ~36MB
| Rust       | cargo-component | ~220KB
| TypeScript | jco            | ~11MB
| Go         | wit-bindgen-go         | ~340KB

## File Structure

Each example has:
```
language/
├── [source files]   # app.py, src/lib.rs, src/index.ts, or main.go
├── wit/
│   └── world.wit    # WIT world definition
├── Makefile         # build, compose, compose-stdio, run, run-stdio, server-test, server-test-stdio, clean
└── README.md        # Language-specific instructions
```

## Transport Types

Examples support two transport types:

- **HTTP** (default) - JSON-RPC over HTTP via `wasmtime serve`
  - Build: `make compose`
  - Run: `make run` (server on http://0.0.0.0:8080)
  - Test: `make server-test`

- **Stdio** - Newline-delimited JSON-RPC over stdin/stdout for MCP clients
  - Build: `make compose-stdio`
  - Run: `make run-stdio` (launches MCP Inspector)
  - Test: `make server-test-stdio`
