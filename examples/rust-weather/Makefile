.PHONY: build test clean compose run run-wasmtime help

# Default gateway version to use
GATEWAY_VERSION ?= 0.0.5
GATEWAY_WASM ?= wasmcp-spin.wasm

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the handler component
	cd handler && cargo component build --release

test: ## Run tests
	cd handler && cargo test

clean: ## Clean build artifacts
	cd handler && cargo clean
	rm -f composed.wasm

compose: build ## Compose handler with gateway using wac plug
	@echo "Composing handler with gateway..."
	wac plug wasmcp-spin.wasm --plug handler/target/wasm32-wasip1/release/rust_weather.wasm -o composed.wasm
	@echo "✅ Created composed.wasm"

run: build ## Run with Spin (handles dependencies automatically)
	spin up

run-wasmtime: compose ## Run with wasmtime (requires manual composition)
	wasmtime serve -Scli -Skeyvalue -Shttp composed.wasm

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