.PHONY: build test clean run help

# Component paths
PROVIDER_WASM = target/wasm32-wasip1/release/weather_rs.wasm
MCP_SERVER_WASM = mcp-http-server.wasm

# Transport package from registry (tools-only server)
TRANSPORT_PKG = fastertools:mcp-http-tools-server@0.1.0

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: ## Check and install required tools
	@echo "Checking for required tools..."
	@command -v cargo >/dev/null 2>&1 || { echo "❌ cargo not found. Please install Rust: https://rustup.rs"; exit 1; }
	@command -v cargo-component >/dev/null 2>&1 || { \
		echo "📦 Installing cargo-component..."; \
		cargo install --locked cargo-component; \
	}
	@command -v wkg >/dev/null 2>&1 || { \
		echo "📦 Installing wkg..."; \
		cargo install --locked wkg; \
	}
	@command -v wac >/dev/null 2>&1 || { \
		echo "📦 Installing wac..."; \
		cargo install --locked wac-cli; \
	}
	@command -v wasmtime >/dev/null 2>&1 || { \
		echo "⚠️  wasmtime not found. Install from: https://wasmtime.dev"; \
	}
	@echo "✅ All tools ready!"

build: ## Build and compose everything into mcp-http-server.wasm
	@echo "Building provider component..."
	@cargo component build --release --target wasm32-wasip1
	@echo "Downloading transport from registry..."
	@wkg get $(TRANSPORT_PKG) -o .transport-tmp.wasm
	@echo "Plugging provider into transport..."
	@wac plug --plug $(PROVIDER_WASM) .transport-tmp.wasm -o $(MCP_SERVER_WASM)
	@rm -f .transport-tmp.wasm
	@echo "✅ Created mcp-http-server.wasm ($$( ls -lh $(MCP_SERVER_WASM) | awk '{print $$5}' ))"

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean
	rm -f $(MCP_SERVER_WASM) .transport-tmp.wasm

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