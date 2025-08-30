.PHONY: build test clean run help

# Component paths
PROVIDER_WASM = provider.wasm
MCP_SERVER_WASM = mcp-http-server.wasm

# Transport package from registry
TRANSPORT_PKG = fastertools:mcp-transport-http@0.1.0

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: ## Check tools and install dependencies
	@echo "Checking for required tools..."
	@command -v node >/dev/null 2>&1 || { echo "❌ node not found. Please install Node.js: https://nodejs.org"; exit 1; }
	@command -v npm >/dev/null 2>&1 || { echo "❌ npm not found. Please install Node.js: https://nodejs.org"; exit 1; }
	@command -v wkg >/dev/null 2>&1 || { \
		echo "❌ wkg not found. Install with: cargo install wkg"; \
		exit 1; \
	}
	@command -v wac >/dev/null 2>&1 || { \
		echo "❌ wac not found. Install with: cargo install wac-cli"; \
		exit 1; \
	}
	@command -v wasmtime >/dev/null 2>&1 || { \
		echo "⚠️  wasmtime not found. Install from: https://wasmtime.dev"; \
	}
	@echo "✅ All tools ready!"
	@echo "Installing npm dependencies..."
	@npm install
	@echo "Fetching WIT dependencies..."
	@wkg wit fetch
	@echo "✅ Setup complete!"

build: ## Build and compose everything into mcp-http-server.wasm
	@echo "Installing dependencies..."
	@npm install
	@echo "Bundling JavaScript modules..."
	@npm run bundle
	@echo "Building provider component..."
	@npx jco componentize bundled.js --wit wit --world-name weather-js-new --out $(PROVIDER_WASM)
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

run: build ## Build and run with wasmtime serve
	wasmtime serve -Scli mcp-http-server.wasm

serve: ## Serve already built mcp-http-server.wasm
	wasmtime serve -Scli mcp-http-server.wasm

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

test-call: ## Test calling a tool
	@echo "Testing tool call..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello World"}},"id":3}' | python3 -m json.tool