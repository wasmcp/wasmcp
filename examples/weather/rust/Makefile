.PHONY: build test clean compose run help

# Component paths
HANDLER_WASM = target/wasm32-wasip1/release/rust.wasm
COMPOSED_WASM = composed.wasm
SERVER_WASM = wasmcp-server-tools.wasm

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build-handler: ## Build just the handler component
	cargo component build --release --target wasm32-wasip1
	@echo "✅ Built handler component"

build: build-handler get-deps ## Build and compose everything into composed.wasm
	@echo "Plugging handler into tools-only server..."
	wac plug --plug $(HANDLER_WASM) $(SERVER_WASM) -o $(COMPOSED_WASM)
	@echo "✅ Created composed.wasm ($$( ls -lh $(COMPOSED_WASM) | awk '{print $$5}' ))"

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean
	rm -f $(COMPOSED_WASM) $(SERVER_WASM)

get-deps: ## Build server component locally if not available
	@if [ ! -f $(SERVER_WASM) ]; then \
		echo "Building tools-only server locally..."; \
		cd ../../../components/server && cargo component build --features tools --release && \
		cp target/wasm32-wasip1/release/wasmcp_server.wasm ../../examples/weather/rust/$(SERVER_WASM); \
	fi

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