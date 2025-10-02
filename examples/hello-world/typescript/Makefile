# Makefile for typescript

.PHONY: all build compose compose-stdio run run-stdio server-test server-test-stdio clean setup help

# Component and server files
COMPONENT_NAME = typescript
SERVER = mcp-server.wasm
SERVER_STDIO = mcp-server-stdio.wasm

# Default target
all: compose

# Help target
help:
	@echo "typescript Makefile targets:"
	@echo "  make setup            - Install npm dependencies"
	@echo "  make build            - Build the TypeScript component"
	@echo "  make compose          - Compose the MCP server (HTTP transport)"
	@echo "  make compose-stdio    - Compose the MCP server (stdio transport)"
	@echo "  make run              - Run the HTTP server"
	@echo "  make run-stdio        - Run the stdio server with MCP Inspector"
	@echo "  make server-test      - Test the HTTP server endpoints"
	@echo "  make server-test-stdio - Test the stdio server"
	@echo "  make clean            - Clean build artifacts"

# Setup npm dependencies
setup:
	@echo "Installing npm dependencies..."
	@npm install
	@echo "✅ Dependencies installed!"

# Build the TypeScript component
build: setup
	@echo "Fetching WIT dependencies..."
	@wkg wit fetch
	@echo "Building TypeScript component..."
	@mkdir -p target
	@npm run build
	@echo "✅ TypeScript component built successfully!"
	@echo "Component: target/typescript.wasm"

# Compose MCP server (HTTP transport)
compose: build
	@echo "Composing MCP server (HTTP transport)..."
	@wasmcp compose --tools target/$(COMPONENT_NAME).wasm -o $(SERVER)
	@echo "✅ Composed: $(SERVER)"

# Compose MCP server (stdio transport)
compose-stdio: build
	@echo "Composing MCP server (stdio transport)..."
	@wasmcp compose --tools target/$(COMPONENT_NAME).wasm --transport stdio -o $(SERVER_STDIO)
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

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf node_modules build target src/generated wit/deps wkg.lock package-lock.json deps $(SERVER) $(SERVER_STDIO)
	@echo "✅ Clean complete!"
