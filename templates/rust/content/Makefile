.PHONY: build test clean compose run help

# Component paths
HANDLER_WASM = target/wasm32-wasip1/release/{{project-name | snake_case}}.wasm
NULL_RESOURCES_URL = https://github.com/fastertools/wasmcp/releases/latest/download/null-resources.wasm
NULL_PROMPTS_URL = https://github.com/fastertools/wasmcp/releases/latest/download/null-prompts.wasm
SERVER_WASM = wasmcp-server.wasm
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
	rm -f $(COMPOSED_WASM) null-resources.wasm null-prompts.wasm

get-nulls: ## Download null components if not available locally
	@if [ ! -f null-resources.wasm ]; then \
		echo "Downloading null-resources.wasm..."; \
		curl -L $(NULL_RESOURCES_URL) -o null-resources.wasm; \
	fi
	@if [ ! -f null-prompts.wasm ]; then \
		echo "Downloading null-prompts.wasm..."; \
		curl -L $(NULL_PROMPTS_URL) -o null-prompts.wasm; \
	fi

compose: build get-nulls ## Compose handler with null components and server using WAC
	@if [ ! -f $(SERVER_WASM) ]; then echo "Error: $(SERVER_WASM) not found."; exit 1; fi
	@echo "Composing handler with null components and server..."
	wac compose compose.wac \
		-d {{project-name | kebab_case}}:handler=$(HANDLER_WASM) \
		-d fastertools:null-resources=null-resources.wasm \
		-d fastertools:null-prompts=null-prompts.wasm \
		-d wasmcp:server=$(SERVER_WASM) \
		--registry https://ghcr.io \
		-o $(COMPOSED_WASM)
	@echo "✅ Created composed.wasm ($$( ls -lh $(COMPOSED_WASM) | awk '{print $$5}' ))"

run: compose ## Run with wasmtime serve
	wasmtime serve -Scli composed.wasm

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