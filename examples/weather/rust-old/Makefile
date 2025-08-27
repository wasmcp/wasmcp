.PHONY: build test clean compose run help

# Component paths
HANDLER_WASM = target/wasm32-wasip1/release/rust_weather.wasm
NULL_RESOURCES_WASM = ../../../components/null-resources/target/wasm32-wasip1/release/null_resources.wasm
NULL_PROMPTS_WASM = ../../../components/null-prompts/target/wasm32-wasip1/release/null_prompts.wasm
SERVER_WASM = server.wasm
COMPOSED_WASM = composed.wasm

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the handler component (tools only)
	cargo component build --release --target wasm32-wasip1
	@echo "✅ Built handler component (tools only)"

build-nulls: ## Build null components for missing capabilities
	@echo "Building null-resources component..."
	cd ../../../components/null-resources && cargo component build --release
	@echo "Building null-prompts component..."
	cd ../../../components/null-prompts && cargo component build --release
	@echo "✅ Built null components"

clean: ## Clean build artifacts
	cargo clean
	rm -f $(COMPOSED_WASM)

compose: build build-nulls ## Compose handler with null components and server using WAC
	@if [ ! -f $(SERVER_WASM) ]; then echo "Error: $(SERVER_WASM) not found."; exit 1; fi
	@echo "Composing weather handler with null components and server..."
	wac compose compose.wac \
		-d example:weather=$(HANDLER_WASM) \
		-d fastertools:null-resources=$(NULL_RESOURCES_WASM) \
		-d fastertools:null-prompts=$(NULL_PROMPTS_WASM) \
		-d wasmcp:server=$(SERVER_WASM) \
		--registry https://ghcr.io \
		-o $(COMPOSED_WASM)
	@echo "✅ Created composed.wasm ($$(ls -lh $(COMPOSED_WASM) | awk '{print $$5}'))"

run: compose ## Run with wasmtime serve
	wasmtime serve -Scli $(COMPOSED_WASM)

# Testing the composed component
test-init: ## Test initialize endpoint
	@echo "Testing initialize..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"initialize","params":{"protocol_version":"2025-06-18","capabilities":{},"client_info":{"name":"test","version":"1.0"}},"id":1}' | python3 -m json.tool

test-tools: ## Test tools/list endpoint
	@echo "Testing tools/list..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}' | python3 -m json.tool

test-resources: ## Test resources/list endpoint (should return empty from null component)
	@echo "Testing resources/list..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"resources/list","params":{},"id":3}' | python3 -m json.tool

test-prompts: ## Test prompts/list endpoint (should return empty from null component)
	@echo "Testing prompts/list..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"prompts/list","params":{},"id":4}' | python3 -m json.tool

test-echo: ## Test echo tool
	@echo "Testing echo tool..."
	@echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":"{\"message\":\"Hello from MCP!\"}"},"id":5}' | \
		curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool

test-weather: ## Test weather tool
	@echo "Testing weather tool..."
	@echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_weather","arguments":"{\"location\":\"San Francisco\"}"},"id":6}' | \
		curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool