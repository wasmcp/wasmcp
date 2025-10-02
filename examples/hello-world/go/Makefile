# Makefile for go

.PHONY: all build compose compose-stdio run run-stdio server-test server-test-stdio clean setup bindgen help

# Output directory
OUT_DIR := target

# Component and server files
COMPONENT_NAME := go
SERVER := mcp-server.wasm
SERVER_STDIO := mcp-server-stdio.wasm

# Default target
all: compose

# Help target
help:
	@echo "go Makefile targets:"
	@echo "  make setup            - Install Go tools and dependencies"
	@echo "  make bindgen          - Generate Go bindings from WIT"
	@echo "  make build            - Build the Go component"
	@echo "  make compose          - Compose the MCP server (HTTP transport)"
	@echo "  make compose-stdio    - Compose the MCP server (stdio transport)"
	@echo "  make run              - Run the HTTP server"
	@echo "  make run-stdio        - Run the stdio server with MCP Inspector"
	@echo "  make server-test      - Test the HTTP server endpoints"
	@echo "  make server-test-stdio - Test the stdio server"
	@echo "  make clean            - Clean build artifacts"

# Setup Go tools
setup:
	@echo "Setting up Go environment..."
	@GOFLAGS="-tags=setup" go mod download
	@go install go.bytecodealliance.org/cmd/wit-bindgen-go@latest
	@echo "✅ Go environment ready"

# Generate bindings from WIT
bindgen: setup
	@echo "Fetching WIT dependencies..."
	@wkg wit fetch
	@echo "Generating Go bindings from WIT..."
	@rm -rf gen
	@wit-bindgen-go generate --versioned --out gen --package-root github.com/go/gen wit
	@echo "Fixing generated code bugs..."
	@find gen -name "*.wit.go" -exec sed -i 's/content_: content_/content: content_/g' {} +
	@echo "Updating Go dependencies..."
	@go mod tidy
	@echo "✅ Go bindings generated"

# Build the Go component using TinyGo
build: bindgen
	@echo "Building Go component with TinyGo..."
	@mkdir -p $(OUT_DIR)
	@tinygo build -target=wasip2 \
		-o $(OUT_DIR)/$(COMPONENT_NAME).wasm \
		--wit-package wit \
		--wit-world tools-handler \
		-scheduler=asyncify \
		-no-debug \
		.
	@echo "✅ Go component built successfully!"
	@echo "Component: $(OUT_DIR)/$(COMPONENT_NAME).wasm"

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

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf $(OUT_DIR) gen wit/deps wkg.lock deps $(SERVER) $(SERVER_STDIO)
	@echo "✅ Clean complete!"
