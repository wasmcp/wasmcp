.PHONY: build build-auth test clean run run-auth help

# Component paths
PROVIDER_WASM = {{project-name}}_provider.wasm
MCP_SERVER_WASM = mcp-http-server.wasm

# Registry packages - auth is now built into transport
TRANSPORT_PKG = wasmcp:mcp-transport-http-tools@0.1.0

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: ## Check tools and install dependencies
	@chmod +x setup.sh
	@./setup.sh

build: ## Build and compose everything into mcp-http-server.wasm (no auth)
	@echo "Installing dependencies..."
	@npm install
	@echo "Bundling JavaScript modules..."
	@npm run bundle
	@echo "Building provider component..."
	@npx jco componentize bundled.js --wit wit --world-name {{project-name}} --out $(PROVIDER_WASM)
	@echo "Downloading transport from registry..."
	@wkg get $(TRANSPORT_PKG) -o .transport-tmp.wasm
	@echo "Plugging provider into transport..."
	@wac plug --plug $(PROVIDER_WASM) .transport-tmp.wasm -o $(MCP_SERVER_WASM)
	@rm -f .transport-tmp.wasm bundled.js
	@echo "✅ Created mcp-http-server.wasm ($$( ls -lh $(MCP_SERVER_WASM) | awk '{print $$5}' ))"


build-types: ## Generate TypeScript type definitions
	jco types wit -o types

clean: ## Clean build artifacts
	rm -f $(PROVIDER_WASM) $(MCP_SERVER_WASM) .transport-tmp.wasm bundled.js
	rm -rf dist types

run: build ## Build and run
	@$(MAKE) serve

serve: ## Serve already built mcp-http-server.wasm
	@if command -v wasmtime >/dev/null 2>&1; then \
		echo "Starting server with wasmtime..."; \
		wasmtime serve -Scli mcp-http-server.wasm; \
	elif command -v spin >/dev/null 2>&1; then \
		echo "Starting server with spin..."; \
		spin up --from mcp-http-server.wasm; \
	else \
		echo "Error: No compatible runtime found. Please install wasmtime or spin."; \
		echo "  Install wasmtime: curl https://wasmtime.dev/install.sh -sSf | bash"; \
		echo "  Install spin: curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash"; \
		exit 1; \
	fi


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

test-echo: ## Test echo tool
	@echo "Testing echo tool..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello World"}},"id":3}' | python3 -m json.tool

test-weather: ## Test weather tool
	@echo "Testing get_weather tool..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_weather","arguments":{"location":"London"}},"id":4}' | python3 -m json.tool

test-multi: ## Test multi_weather tool
	@echo "Testing multi_weather tool..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"multi_weather","arguments":{"cities":["London","Paris","Berlin"]}},"id":5}' | python3 -m json.tool

test-all: test-init test-tools test-echo test-weather test-multi ## Run all tests
