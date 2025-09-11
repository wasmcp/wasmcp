# Top-level Makefile for wasmcp project

.PHONY: help build test clean setup

# Default target
help:
	@echo "Available targets:"
	@echo "  setup       - Install required tools and dependencies"
	@echo "  build       - Build all workspace members"
	@echo "  test        - Run all tests"
	@echo "  clean       - Clean build artifacts"
	@echo ""
	@echo "Component-specific targets:"
	@echo "  build-core  - Build wasmcp-core library"
	@echo "  test-core   - Test wasmcp-core library"
	@echo "  build-http  - Build http-transport component"
	@echo "  test-http   - Test http-transport component"
	@echo ""
	@echo "WASM-specific targets:"
	@echo "  test-wasm   - Test wasmcp-core in WASM environment"
	@echo "  build-wasm  - Build http-transport as WASM component"
	@echo ""
	@echo "wasmcp server targets:"
	@echo "  build-wasmcp     - Build wasmcp MCP server"
	@echo "  run-wasmcp       - Run wasmcp server locally"
	@echo "  test-wasmcp      - Test wasmcp MCP tools"
	@echo "  test-wasmcp-init - Test wasmcp initialize endpoint"

# Setup development environment
setup:
	@echo "Checking for required tools..."
	@command -v cargo >/dev/null 2>&1 || { echo "cargo is required but not installed. Aborting." >&2; exit 1; }
	@command -v rustc >/dev/null 2>&1 || { echo "rustc is required but not installed. Aborting." >&2; exit 1; }
	@echo "Installing wasm-pack for WASM testing..."
	@command -v wasm-pack >/dev/null 2>&1 || curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
	@echo "Setup complete!"

# Build all workspace members
build: build-core build-http

# Build wasmcp-core library
build-core:
	@echo "Building wasmcp-core library..."
	@cargo build -p wasmcp-core

# Build http-transport component
build-http:
	@echo "Building http-transport component..."
	@cd components/http-transport && cargo component build

# Test all workspace members
test: test-core test-http

# Test wasmcp-core library (native)
test-core:
	@echo "Testing wasmcp-core library (native)..."
	@cargo test -p wasmcp-core

# Test wasmcp-core library (WASM)
test-wasm:
	@echo "Testing wasmcp-core library (WASM)..."
	@cd crates/wasmcp-core && wasm-pack test --node

# Test http-transport component
test-http:
	@echo "Testing http-transport component..."
	@cd components/http-transport && cargo test

# Build http-transport as WASM component
build-wasm:
	@echo "Building http-transport as WASM component..."
	@cd components/http-transport && cargo component build --release

# Clean all build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@find . -name "target" -type d -exec rm -rf {} + 2>/dev/null || true
	@find . -name "Cargo.lock" -type f -not -path "./Cargo.lock" -exec rm {} + 2>/dev/null || true
	@echo "Clean complete!"

# Development helpers
.PHONY: check fmt clippy

# Run cargo check
check:
	@cargo check --workspace

# Format code
fmt:
	@cargo fmt --all

# Run clippy
clippy:
	@cargo clippy --workspace -- -D warnings

# wasmcp MCP server targets
.PHONY: build-wasmcp run-wasmcp test-wasmcp test-wasmcp-init test-wasmcp-tools

# Build wasmcp server
build-wasmcp:
	@echo "Building wasmcp MCP server..."
	@cargo build -p wasmcp

# Run wasmcp server locally
run-wasmcp: build-wasmcp
	@echo "Killing any existing process on port 3000..."
	@lsof -ti:3000 | xargs kill -9 2>/dev/null || true
	@echo "Starting wasmcp MCP server on http://127.0.0.1:3000"
	@cargo run -p wasmcp
