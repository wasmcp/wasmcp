.PHONY: build test clean compose run help

# Component paths
HANDLER_WASM = target/wasm32-wasip1/release/test_async_single.wasm
COMPOSED_WASM = composed.wasm

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the handler component (tools only by default)
	cargo component build --release --target wasm32-wasip1
	@echo "✅ Built handler component"

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean
	rm -f $(COMPOSED_WASM) null-resources.wasm null-prompts.wasm wasmcp-server.wasm

get-deps: ## Download all dependencies from registry if not available locally
	@if [ ! -f null-resources.wasm ]; then \
		echo "Downloading null-resources component from registry..."; \
		wkg get fastertools:null-resources@0.1.0 -o null-resources.wasm; \
	fi
	@if [ ! -f null-prompts.wasm ]; then \
		echo "Downloading null-prompts component from registry..."; \
		wkg get fastertools:null-prompts@0.1.0 -o null-prompts.wasm; \
	fi
	@if [ ! -f wasmcp-server.wasm ]; then \
		echo "Downloading server component from registry..."; \
		wkg get fastertools:wasmcp-server@0.1.0 -o wasmcp-server.wasm; \
	fi

compose: build get-deps ## Compose handler with null components and server using WAC
	@echo "Composing handler with null components and server..."
	wac compose compose.wac \
		-d test-async-single:handler=$(HANDLER_WASM) \
		-d fastertools:null-resources=null-resources.wasm \
		-d fastertools:null-prompts=null-prompts.wasm \
		-d fastertools:wasmcp-server=wasmcp-server.wasm \
		-o $(COMPOSED_WASM)
	@echo "✅ Created composed.wasm ($$( ls -lh $(COMPOSED_WASM) | awk '{print $$5}' ))"

run: compose ## Run with wasmtime serve
	wasmtime serve -Scli composed.wasm

# Testing the composed component
test-init: ## Test initialize endpoint
	@echo "Testing initialize..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | python3 -m json.tool

test-tools: ## Test tools/list endpoint
	@echo "Testing tools/list..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}' | python3 -m json.tool

test-resources: ## Test resources/list (returns empty from null component)
	@echo "Testing resources/list..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"resources/list","params":{},"id":3}' | python3 -m json.tool

test-prompts: ## Test prompts/list (returns empty from null component)
	@echo "Testing prompts/list..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"prompts/list","params":{},"id":4}' | python3 -m json.tool