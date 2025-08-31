.PHONY: build test clean run help setup deps-check

# Component paths
PROVIDER_WASM = {{project-name | snake_case}}-provider.wasm
MCP_SERVER_WASM = mcp-http-server.wasm

# Transport package from registry (tools-only server)
TRANSPORT_PKG = fastertools:mcp-http-tools-server@0.1.0

# Go/TinyGo configuration
TINYGO = tinygo
WASM_TOOLS = wasm-tools

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
	@which $(WASM_TOOLS) > /dev/null || { \
		echo "❌ wasm-tools not found. Please install from: https://github.com/bytecodealliance/wasm-tools/releases"; \
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
	@wit-bindgen-go generate --world {{project-name | kebab_case}} --out internal ./{{project-name | kebab_case}}:provider.wasm
	@echo "✅ Bindings generated"

build-provider: bindgen ## Build the provider component
	@echo "Building provider component with TinyGo..."
	@$(TINYGO) build -target=wasip2 -o $(PROVIDER_WASM) \
		-scheduler=asyncify \
		--wit-package {{project-name | kebab_case}}:provider.wasm \
		--wit-world {{project-name | kebab_case}} \
		main.go concurrent_http.go
	@echo "✅ Provider component built"

build: ## Build and compose everything into mcp-http-server.wasm
	@$(MAKE) deps
	@$(MAKE) build-provider
	@echo "Downloading transport from registry..."
	@wkg get $(TRANSPORT_PKG) -o .transport-tmp.wasm 2>/dev/null || true
	@echo "Plugging provider into transport..."
	@wac plug --plug $(PROVIDER_WASM) .transport-tmp.wasm -o $(MCP_SERVER_WASM)
	@rm -f .transport-tmp.wasm
	@echo "✅ Created mcp-http-server.wasm ($$( ls -lh $(MCP_SERVER_WASM) 2>/dev/null | awk '{print $$5}' ))"

build-optimized: ## Build optimized version without debug info
	@$(MAKE) deps
	@echo "Building optimized provider component..."
	@$(TINYGO) build -target=wasip2 -o $(PROVIDER_WASM) \
		-scheduler=asyncify \
		--wit-package {{project-name | kebab_case}}:provider.wasm \
		--wit-world {{project-name | kebab_case}} \
		-no-debug \
		main.go concurrent_http.go
	@echo "Downloading transport from registry..."
	@wkg get $(TRANSPORT_PKG) -o .transport-tmp.wasm 2>/dev/null || true
	@echo "Plugging provider into transport..."
	@wac plug --plug $(PROVIDER_WASM) .transport-tmp.wasm -o $(MCP_SERVER_WASM)
	@rm -f .transport-tmp.wasm
	@echo "✅ Created optimized mcp-http-server.wasm ($$( ls -lh $(MCP_SERVER_WASM) 2>/dev/null | awk '{print $$5}' ))"

clean: ## Clean build artifacts
	@rm -f $(PROVIDER_WASM) $(MCP_SERVER_WASM) .transport-tmp.wasm
	@rm -rf internal
	@rm -f *.wasm
	@echo "✅ Build artifacts cleaned"

clean-all: clean ## Clean everything including Go cache
	@go clean -cache
	@go clean -modcache
	@echo "✅ Everything cleaned"

run: build ## Build and run with available runtime
	@$(MAKE) serve

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
	@echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello from Go!"}},"id":3}' | \
		curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool

test-weather: ## Test weather tool
	@echo "Testing get_weather tool..."
	@echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_weather","arguments":{"location":"Tokyo"}},"id":4}' | \
		curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool

test-multi: ## Test multi_weather tool
	@echo "Testing multi_weather tool..."
	@echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"multi_weather","arguments":{"cities":["Tokyo","London","New York"]}},"id":5}' | \
		curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d @- | python3 -m json.tool

test-all: test-init test-tools test-echo test-weather test-multi ## Run all tests

# Component introspection
inspect: ## Inspect the built component
	@if [ -f $(MCP_SERVER_WASM) ]; then \
		echo "Component exports:"; \
		$(WASM_TOOLS) component wit $(MCP_SERVER_WASM) | head -20; \
	else \
		echo "Component not built yet. Run 'make build' first."; \
	fi