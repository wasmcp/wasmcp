.PHONY: build test clean compose run run-wasmtime help

# Default server version to use
SERVER_VERSION ?= 0.1.0
SERVER_WASM ?= wasmcp-server.wasm

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the handler component
	@WIT_DIR=$$(mktemp -d) && \
	go run github.com/fastertools/wasmcp/src/sdk/wasmcp-go/cmd/extract-wit $$WIT_DIR && \
	echo '{"inherits":["wasip2"],"wit-package":"'$$WIT_DIR'","wit-world":"wasmcp:mcp/mcp-handler"}' > $$WIT_DIR/target.json && \
	tinygo build -target=$$WIT_DIR/target.json -scheduler=asyncify -no-debug -o handler.wasm . ; \
	EXIT_CODE=$$? ; \
	rm -rf $$WIT_DIR ; \
	if [ $$EXIT_CODE -eq 0 ]; then echo "✅ Built handler.wasm"; fi ; \
	exit $$EXIT_CODE

test: ## Run tests
	@echo "Testing not yet implemented for Go handlers"

clean: ## Clean build artifacts
	rm -f handler.wasm composed.wasm
	@echo "✅ Cleaned build artifacts"

compose: build ## Compose handler with server
	@echo "Composing handler with server..."
	wac plug --plug handler.wasm $(SERVER_WASM) -o composed.wasm
	@echo "✅ Created composed.wasm"

run: build ## Run with Spin (handles dependencies automatically)
	spin up

run-wasmtime: compose ## Run with wasmtime (requires manual composition)
	wasmtime serve -S cli -S http composed.wasm

# Development targets
dev: build compose ## Build and compose for development
	@echo "Ready to run: make run or make run-wasmtime"

# Testing the composed component
test-tools: ## Test tools/list endpoint
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}' | python3 -m json.tool

test-echo: ## Test echo tool
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello from MCP!"}},"id":1}' | python3 -m json.tool

test-weather: ## Test weather tool
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"weather","arguments":{"location":"San Francisco"}},"id":1}' | python3 -m json.tool