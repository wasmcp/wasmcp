.PHONY: build test clean compose run help

# Server component configuration
SERVER_WASM = wasmcp-server.wasm

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the handler component
	cargo component build --release --target wasm32-wasip1
	@echo "✅ Built handler component"

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean
	rm -f composed.wasm

compose: build ## Compose handler with server using wac plug
	@if [ ! -f $(SERVER_WASM) ]; then echo "Error: $(SERVER_WASM) not found."; exit 1; fi
	@echo "Composing handler with server..."
	wac plug --plug target/wasm32-wasip1/release/test_mcp_handler.wasm $(SERVER_WASM) -o composed.wasm
	@echo "✅ Created composed.wasm"

run: compose ## Run with wasmtime serve
	wasmtime serve -Scli composed.wasm

# Testing the composed component
test-init: ## Test initialize endpoint
	@echo "Testing initialize..."
	@curl -s -X POST http://localhost:8080 \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":"0.1.0","capabilities":{},"client_info":{"name":"test","version":"1.0"}},"id":1}' | python3 -m json.tool

test-tools: ## Test tools/list endpoint
	@echo "Testing tools/list..."
	@curl -s -X POST http://localhost:8080 \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}' | python3 -m json.tool

test-echo: ## Test echo tool
	@echo "Testing echo tool..."
	@echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":"{\"message\":\"Hello from MCP!\"}"},"id":3}' | \
		curl -s -X POST http://localhost:8080 \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool

test-weather: ## Test weather tool
	@echo "Testing weather tool..."
	@echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_weather","arguments":"{\"location\":\"San Francisco\"}"},"id":4}' | \
		curl -s -X POST http://localhost:8080 \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool