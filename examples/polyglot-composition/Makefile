.PHONY: build clean compose compose-stdio run run-stdio test test-stdio help

# Server files
SERVER = mcp-server.wasm
SERVER_STDIO = mcp-server-stdio.wasm

# Default target
all: compose

# Help target
help:
	@echo "Polyglot Composition Makefile targets:"
	@echo "  make build            - Build all components (Rust, Python, TypeScript, Go)"
	@echo "  make compose          - Compose the MCP server (HTTP transport)"
	@echo "  make compose-stdio    - Compose the MCP server (stdio transport)"
	@echo "  make run              - Run the HTTP server"
	@echo "  make run-stdio        - Run the stdio server with MCP Inspector"
	@echo "  make test             - Test the HTTP server endpoints"
	@echo "  make test-stdio       - Test the stdio server"
	@echo "  make clean            - Clean build artifacts"

build:
	@echo "Building all components..."
	cd rust-tools && make build
	cd python-resources && make build
	cd ts-logger && make build
	cd go-logger && make build
	@echo "✅ All components built"

compose: build
	@echo "Composing MCP server (HTTP transport)..."
	@wasmcp compose \
		--middleware ts-logger/target/ts_logger.wasm \
		--middleware go-logger/target/go_logger.wasm \
		--tools rust-tools/target/wasm32-wasip1/release/rust_tools.wasm \
		--resources python-resources/target/python_resources.wasm \
		-o $(SERVER)
	@echo "✅ Composed: $(SERVER)"

compose-stdio: build
	@echo "Composing MCP server (stdio transport)..."
	@wasmcp compose \
		--middleware ts-logger/target/ts_logger.wasm \
		--middleware go-logger/target/go_logger.wasm \
		--tools rust-tools/target/wasm32-wasip1/release/rust_tools.wasm \
		--resources python-resources/target/python_resources.wasm \
		--transport stdio \
		-o $(SERVER_STDIO)
	@echo "✅ Composed: $(SERVER_STDIO)"

run: compose
	@echo "Starting MCP server on http://0.0.0.0:8080..."
	wasmtime serve -Scommon $(SERVER)

run-stdio: compose-stdio
	@echo "Starting MCP server with stdio transport (MCP Inspector)..."
	@npx @modelcontextprotocol/inspector wasmtime run -Shttp $(SERVER_STDIO)

test:
	@echo "Testing tools/list..."
	@npx @modelcontextprotocol/inspector@0.16.8 --cli http://localhost:8080/mcp \
		--method tools/list | jq -r '.tools[0].name'
	@echo "Testing resources/list..."
	@npx @modelcontextprotocol/inspector@0.16.8 --cli http://localhost:8080/mcp \
		--method resources/list | jq -r '.resources[0].name'

test-stdio: compose-stdio
	@echo "Testing tools/list (stdio)..."
	@npx @modelcontextprotocol/inspector@0.16.8 --cli --transport stdio --method tools/list wasmtime run -Shttp $(SERVER_STDIO) | jq '.'
	@echo "Testing resources/list (stdio)..."
	@npx @modelcontextprotocol/inspector@0.16.8 --cli --transport stdio --method resources/list wasmtime run -Shttp $(SERVER_STDIO) | jq '.'

clean:
	cd rust-tools && make clean || true
	cd python-resources && make clean || true
	cd ts-logger && make clean || true
	cd go-logger && make clean || true
	rm -rf deps $(SERVER) $(SERVER_STDIO)
	@echo "✅ Cleaned"
