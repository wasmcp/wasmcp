.PHONY: build test clean serve help setup deps-check

# Build directory
BUILD_DIR = build

# Component paths
PROVIDER_WASM = $(BUILD_DIR)/weather-go-provider.wasm
MCP_SERVER_WASM = $(BUILD_DIR)/mcp-http-server.wasm

# Registry packages
TRANSPORT_PKG = wasmcp:mcp-transport-http-tools@0.2.0

# Go/TinyGo configuration
TINYGO = tinygo

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: ## Run initial setup (checks dependencies)
	@chmod +x setup.sh
	@./setup.sh

deps-check: ## Check if required tools are installed
	@echo "Checking for required tools..."
	@which $(TINYGO) > /dev/null || { \
		echo "❌ TinyGo not found. Please install from: https://tinygo.org/getting-started/"; \
		exit 1; \
	}
	@which wkg > /dev/null || { \
		echo "❌ wkg not found. Please install from: https://github.com/bytecodealliance/wasm-pkg-tools/releases"; \
		exit 1; \
	}
	@which wac > /dev/null || { \
		echo "❌ wac not found. Please install from: https://github.com/bytecodealliance/wac/releases"; \
		exit 1; \
	}
	@echo "✅ All required tools are installed"

deps: deps-check ## Install Go dependencies
	@echo "Installing Go dependencies..."
	@go mod download
	@go mod tidy
	@echo "✅ Dependencies installed"

wit-deps: ## Fetch WIT dependencies
	@echo "Fetching WIT dependencies..."
	@wkg wit fetch
	@echo "✅ WIT dependencies fetched"

wit-build: wit-deps ## Build WIT package
	@echo "Building WIT package..."
	@wkg wit build
	@echo "✅ WIT package built"

bindgen: wit-build ## Generate Go bindings
	@echo "Generating Go bindings..."
	@rm -rf internal
	@wit-bindgen-go generate --world weather-go --out internal ./weather-go:provider.wasm
	@echo "✅ Bindings generated"

build-provider: bindgen ## Build the provider component
	@echo "Building provider component with TinyGo..."
	@$(TINYGO) build -target=wasip2 -o $(PROVIDER_WASM) \
		-scheduler=asyncify \
		--wit-package weather-go:provider.wasm \
		--wit-world weather-go \
		.
	@echo "✅ Provider component built"

build: ## Build and compose everything into build/mcp-http-server.wasm
	@mkdir -p $(BUILD_DIR)
	@$(MAKE) deps
	@$(MAKE) build-provider
	@echo "Downloading transport from registry..."
	@wkg get $(TRANSPORT_PKG) -o $(BUILD_DIR)/.transport-tmp.wasm
	@echo "Composing components..."
	@wac plug --plug $(PROVIDER_WASM) $(BUILD_DIR)/.transport-tmp.wasm -o $(MCP_SERVER_WASM)
	@rm -f $(BUILD_DIR)/.transport-tmp.wasm
	@echo "✅ Created $(MCP_SERVER_WASM) ($$( ls -lh $(MCP_SERVER_WASM) | awk '{print $$5}' ))"


build-optimized: ## Build optimized version without debug info
	@mkdir -p $(BUILD_DIR)
	@$(MAKE) deps
	@echo "Building optimized provider component..."
	@$(TINYGO) build -target=wasip2 -o $(PROVIDER_WASM) \
		-scheduler=asyncify \
		--wit-package weather-go:provider.wasm \
		--wit-world weather-go \
		-no-debug \
		.
	@echo "Downloading transport from registry..."
	@wkg get $(TRANSPORT_PKG) -o $(BUILD_DIR)/.transport-tmp.wasm
	@echo "Plugging provider into transport..."
	@wac plug --plug $(PROVIDER_WASM) $(BUILD_DIR)/.transport-tmp.wasm -o $(MCP_SERVER_WASM)
	@rm -f $(BUILD_DIR)/.transport-tmp.wasm
	@echo "✅ Created optimized $(MCP_SERVER_WASM) ($$( ls -lh $(MCP_SERVER_WASM) | awk '{print $$5}' ))"

clean: ## Clean build artifacts
	@rm -rf $(BUILD_DIR)
	@rm -rf internal
	@rm -f *.wasm
	@echo "✅ Build artifacts cleaned"

clean-all: clean ## Clean everything including Go cache
	@go clean -cache
	@go clean -modcache
	@echo "✅ Everything cleaned"

serve: ## Serve already built mcp-http-server.wasm
	@if command -v wasmtime >/dev/null 2>&1; then \
		echo "Starting server with wasmtime..."; \
		wasmtime serve -Scli $(MCP_SERVER_WASM); \
	elif command -v spin >/dev/null 2>&1; then \
		echo "Starting server with spin..."; \
		spin up --from $(MCP_SERVER_WASM); \
	else \
		echo "Error: No compatible runtime found. Please install wasmtime or spin."; \
		echo "  Install wasmtime: curl https://wasmtime.dev/install.sh -sSf | bash"; \
		echo "  Install spin: curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash"; \
		exit 1; \
	fi


# Testing the composed component
test-init: ## Test initialize endpoint
	@echo "Testing initialize..."
	@echo '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | \
		curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool

test-tools: ## Test tools/list endpoint
	@echo "Testing tools/list..."
	@echo '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}' | \
		curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool

test-echo: ## Test echo tool
	@echo "Testing echo tool..."
	@echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello World"}},"id":3}' | \
		curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool

test-weather: ## Test weather tool
	@echo "Testing get_weather tool..."
	@echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_weather","arguments":{"location":"London"}},"id":4}' | \
		curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool

test-multi: ## Test multi_weather tool
	@echo "Testing multi_weather tool..."
	@echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"multi_weather","arguments":{"cities":["London","Paris","Berlin"]}},"id":5}' | \
		curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool

test-all: test-init test-tools test-echo test-weather test-multi ## Run all tests

# Auth testing targets
test-auth-no-token: ## Test auth mode without token (should fail)
	@echo "Testing auth mode without token (expecting 401)..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}' | python3 -m json.tool

test-auth-discovery: ## Test OAuth discovery endpoints
	@echo "Testing OAuth resource metadata..."
	@curl -s http://localhost:8080/.well-known/oauth-protected-resource | python3 -m json.tool
	@echo ""
	@echo "Testing OAuth authorization server metadata..."
	@curl -s http://localhost:8080/.well-known/oauth-authorization-server | python3 -m json.tool
