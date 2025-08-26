.PHONY: build test clean compose run run-wasmtime help

# Get the wasmcp-server component
SERVER_VERSION ?= 0.0.5
SERVER_WASM ?= wasmcp-server.wasm

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the handler component
	tinygo build -target=wasip1 -gc=leaking -buildmode=c-shared -no-debug -o handler.wasm main.go
	wasm-tools component new handler.wasm --adapt wasi_snapshot_preview1.reactor.wasm -o handler.component.wasm
	@echo "✅ Built handler.component.wasm"

test: ## Test the handler (requires wasmtime)
	@echo "Testing not yet implemented for Go handlers"

clean: ## Clean build artifacts
	rm -f handler.wasm handler.component.wasm composed.wasm
	@echo "✅ Cleaned build artifacts"

compose: build ## Compose handler with server
	wac plug handler.component.wasm $(SERVER_WASM) -o composed.wasm
	@echo "✅ Created composed.wasm"

run: build ## Run with Spin
	spin up

run-wasmtime: compose ## Run with wasmtime
	wasmtime serve -Scli composed.wasm

# Testing endpoints
test-tools: ## Test tools/list endpoint
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}' | python3 -m json.tool

test-echo: ## Test echo tool
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello from Go!"}},"id":1}' | python3 -m json.tool