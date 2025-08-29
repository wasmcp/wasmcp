.PHONY: build test clean run help

# Component paths
HANDLER_WASM = target/wasm32-wasip1/release/rust.wasm
COMPOSED_WASM = composed.wasm

# Server package from registry
SERVER_PKG = fastertools:wasmcp-server-tools@0.1.2

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build and compose everything into composed.wasm
	@echo "Building handler component..."
	@cargo component build --release --target wasm32-wasip1
	@echo "Downloading server from registry..."
	@wkg get $(SERVER_PKG) -o .server-tmp.wasm
	@echo "Plugging handler into server..."
	@wac plug --plug $(HANDLER_WASM) .server-tmp.wasm -o $(COMPOSED_WASM)
	@rm -f .server-tmp.wasm
	@echo "✅ Created composed.wasm ($$( ls -lh $(COMPOSED_WASM) | awk '{print $$5}' ))"

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean
	rm -f $(COMPOSED_WASM) .server-tmp.wasm

run: build ## Build and run with wasmtime serve
	wasmtime serve -Scli composed.wasm

serve: ## Serve already built composed.wasm
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

test-weather: ## Test weather tool
	@echo "Testing get_weather tool..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_weather","arguments":{"location":"London"}},"id":3}' | python3 -m json.tool

test-echo: ## Test echo tool
	@echo "Testing echo tool..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello World"}},"id":4}' | python3 -m json.tool