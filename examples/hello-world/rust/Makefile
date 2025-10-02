# Makefile for rust

.PHONY: all build compose compose-stdio run run-stdio server-test server-test-stdio clean help test

# Rust target
TARGET = wasm32-wasip1

# Output directory
OUT_DIR = target/$(TARGET)/release

# Component and server files
COMPONENT_NAME = rust
SERVER = mcp-server.wasm
SERVER_STDIO = mcp-server-stdio.wasm

# Default target
all: compose

# Help target
help:
	@echo "rust Makefile targets:"
	@echo "  make build            - Build the Rust component"
	@echo "  make compose          - Compose the MCP server (HTTP transport)"
	@echo "  make compose-stdio    - Compose the MCP server (stdio transport)"
	@echo "  make run              - Run the HTTP server"
	@echo "  make run-stdio        - Run the stdio server with MCP Inspector"
	@echo "  make server-test      - Test the HTTP server endpoints"
	@echo "  make server-test-stdio - Test the stdio server"
	@echo "  make test             - Run Rust tests"
	@echo "  make clean            - Clean build artifacts"

# Build the Rust component
build:
	@echo "Fetching WIT dependencies..."
	@wkg wit fetch
	@echo "Building Rust component..."
	@cargo component build --release
	@echo "✅ Component built: $(OUT_DIR)/$(COMPONENT_NAME).wasm"

# Compose MCP server (HTTP transport)
compose: build
	@echo "Composing MCP server (HTTP transport)..."
	@wasmcp compose --tools $(OUT_DIR)/$(COMPONENT_NAME).wasm -o $(SERVER)
	@echo "✅ Composed: $(SERVER)"

# Compose MCP server (stdio transport)
compose-stdio: build
	@echo "Composing MCP server (stdio transport)..."
	@wasmcp compose --tools $(OUT_DIR)/$(COMPONENT_NAME).wasm --transport stdio -o $(SERVER_STDIO)
	@echo "✅ Composed: $(SERVER_STDIO)"

# Run the HTTP server
run: compose
	@echo "Starting MCP server on http://0.0.0.0:8080..."
	@wasmtime serve -Scommon $(SERVER)

# Run the stdio server with MCP Inspector
run-stdio: compose-stdio
	@echo "Starting MCP server with stdio transport (MCP Inspector)..."
	@npx @modelcontextprotocol/inspector wasmtime run $(SERVER_STDIO)

# Test the HTTP server
server-test:
	@echo "Testing tools/list..."
	@npx @modelcontextprotocol/inspector@0.16.8 --cli http://localhost:8080/mcp \
		--method tools/list | jq -r '.tools[0].name'
	@echo "Testing tools/call (echo)..."
	@npx @modelcontextprotocol/inspector@0.16.8 --cli http://localhost:8080/mcp \
		--method tools/call \
		--tool-name echo \
		--tool-arg message="Test from Makefile" | jq '.'

# Test the stdio server
server-test-stdio: compose-stdio
	@echo "Testing tools/list (stdio)..."
	@npx @modelcontextprotocol/inspector@0.16.8 --cli --transport stdio --method tools/list wasmtime run $(SERVER_STDIO) | jq '.'

# Run Rust tests
test:
	@cargo test

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@rm -rf wit/deps wkg.lock deps $(SERVER) $(SERVER_STDIO)
	@echo "✅ Clean complete!"
