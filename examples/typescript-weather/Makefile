.PHONY: build test clean install compose run run-wasmtime help

# Default gateway version to use
GATEWAY_VERSION ?= 0.0.5
GATEWAY_WASM ?= wasmcp-spin.wasm

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

install: ## Install dependencies
	cd handler && npm install

build: install ## Build the handler component
	cd handler && npm run build

test: install ## Run tests
	cd handler && npm test

clean: ## Clean build artifacts
	cd handler && npm run clean
	cd handler && rm -rf node_modules
	rm -f $(GATEWAY_WASM) composed.wasm

get-gateway: ## Download the gateway component
	@if [ ! -f $(GATEWAY_WASM) ]; then \
		if command -v wkg >/dev/null 2>&1; then \
			echo "Downloading gateway component v$(GATEWAY_VERSION) with wkg..."; \
			wkg get ghcr.io/fastertools:wasmcp-spin@$(GATEWAY_VERSION) -o $(GATEWAY_WASM); \
		else \
			echo "Note: wkg not found. For manual composition, download the gateway component:"; \
			echo "  https://github.com/fastertools/wasmcp-spin/releases"; \
			echo "Or simply use 'spin up' which handles dependencies automatically."; \
			exit 1; \
		fi \
	else \
		echo "Gateway component already exists at $(GATEWAY_WASM)"; \
	fi

compose: build ## Compose handler with gateway using wac plug
	@echo "Composing handler with gateway..."
	wac plug wasmcp-spin.wasm --plug handler/dist/handler.wasm -o composed.wasm
	@echo "✅ Created composed.wasm"

run: build ## Run with Spin (handles dependencies automatically)
	spin up

run-wasmtime: compose ## Run with wasmtime (requires manual composition)
	wasmtime serve -Scli -Shttp composed.wasm

# Development targets
dev: build compose ## Build and compose for development
	@echo "Ready to run: make run or make run-wasmtime"

# Testing the composed component
test-tools: ## Test tools/list endpoint
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}' | python3 -m json.tool

test-echo: ## Test echo tool
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":"{\"message\":\"Hello from MCP!\"}"},"id":1}' | python3 -m json.tool

# Advanced: build gateway locally (requires Rust)
build-gateway-local: ## Build gateway from source
	@echo "Building gateway from source..."
	@if [ -d ../../../src/components/wasmcp-spin ]; then \
		cd ../../../src/components/wasmcp-spin && cargo component build --release && \
		cp target/wasm32-wasip1/release/wasmcp_spin.wasm $(OLDPWD)/$(GATEWAY_WASM); \
		echo "✅ Built and copied gateway component"; \
	else \
		echo "❌ Gateway source not found. Use 'make get-gateway' instead"; \
		exit 1; \
	fi