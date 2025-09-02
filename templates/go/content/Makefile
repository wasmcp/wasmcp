.PHONY: build build-auth test clean run run-auth help setup deps-check

# Component paths
PROVIDER_WASM = {{project-name | snake_case}}-provider.wasm
MCP_SERVER_WASM = mcp-http-server.wasm
MCP_AUTH_SERVER_WASM = mcp-http-auth-server.wasm

# Registry packages
TRANSPORT_PKG = fastertools:mcp-transport-http-tools@0.1.0
TRANSPORT_AUTH_PKG = fastertools:mcp-transport-http-tools-auth@0.1.0
AUTH_PKG = fastertools:mcp-authorization@0.1.0

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
	@wit-bindgen-go generate --world {{project-name | kebab_case}} --out internal ./{{project-name | kebab_case}}:provider.wasm
	@echo "✅ Bindings generated"

build-provider: bindgen ## Build the provider component
	@echo "Building provider component with TinyGo..."
	@$(TINYGO) build -target=wasip2 -o $(PROVIDER_WASM) \
		-scheduler=asyncify \
		--wit-package {{project-name | kebab_case}}:provider.wasm \
		--wit-world {{project-name | kebab_case}} \
		.
	@echo "✅ Provider component built"

build: ## Build and compose everything into mcp-http-server.wasm (no auth)
	@$(MAKE) deps
	@$(MAKE) build-provider
	@echo "Downloading transport from registry..."
	@wkg get $(TRANSPORT_PKG) -o .transport-tmp.wasm
	@echo "Plugging provider into transport..."
	@wac plug --plug $(PROVIDER_WASM) .transport-tmp.wasm -o $(MCP_SERVER_WASM)
	@rm -f .transport-tmp.wasm
	@echo "✅ Created mcp-http-server.wasm ($$( ls -lh $(MCP_SERVER_WASM) | awk '{print $$5}' ))"

build-auth: ## Build and compose with OAuth authorization enabled
	@$(MAKE) deps
	@$(MAKE) build-provider
	@echo "Downloading components from registry..."
	@echo "  Fetching authorization component..."
	@wkg get $(AUTH_PKG) -o .auth-tmp.wasm
	@echo "  Fetching auth-enabled transport..."
	@wkg get $(TRANSPORT_AUTH_PKG) -o .transport-auth-tmp.wasm
	@echo "Composing with authorization..."
	@echo "  Step 1: Plugging authorization into transport..."
	@wac plug --plug .auth-tmp.wasm .transport-auth-tmp.wasm -o .transport-with-auth.wasm
	@echo "  Step 2: Plugging provider into auth-enabled transport..."
	@wac plug --plug $(PROVIDER_WASM) .transport-with-auth.wasm -o $(MCP_AUTH_SERVER_WASM)
	@rm -f .auth-tmp.wasm .transport-auth-tmp.wasm .transport-with-auth.wasm
	@echo "✅ Created mcp-http-auth-server.wasm ($$( ls -lh $(MCP_AUTH_SERVER_WASM) | awk '{print $$5}' ))"
	@echo ""
	@echo "To run with auth:"
	@echo "  make run-auth"
	@echo ""
	@echo "Or with your JWT configuration:"
	@echo "  export JWT_ISSUER=https://your-domain.authkit.app"
	@echo "  export JWT_AUDIENCE=client_YOUR_CLIENT_ID"
	@echo "  export JWT_JWKS_URI=https://your-domain.authkit.app/oauth2/jwks"
	@echo "  make serve-auth"

build-optimized: ## Build optimized version without debug info
	@$(MAKE) deps
	@echo "Building optimized provider component..."
	@$(TINYGO) build -target=wasip2 -o $(PROVIDER_WASM) \
		-scheduler=asyncify \
		--wit-package {{project-name | kebab_case}}:provider.wasm \
		--wit-world {{project-name | kebab_case}} \
		-no-debug \
		.
	@echo "Downloading transport from registry..."
	@wkg get $(TRANSPORT_PKG) -o .transport-tmp.wasm
	@echo "Plugging provider into transport..."
	@wac plug --plug $(PROVIDER_WASM) .transport-tmp.wasm -o $(MCP_SERVER_WASM)
	@rm -f .transport-tmp.wasm
	@echo "✅ Created optimized mcp-http-server.wasm ($$( ls -lh $(MCP_SERVER_WASM) | awk '{print $$5}' ))"

clean: ## Clean build artifacts
	@rm -f $(PROVIDER_WASM) $(MCP_SERVER_WASM) $(MCP_AUTH_SERVER_WASM) .transport-tmp.wasm .transport-auth-tmp.wasm .transport-with-auth.wasm .auth-tmp.wasm
	@rm -rf internal
	@rm -f *.wasm
	@echo "✅ Build artifacts cleaned"

clean-all: clean ## Clean everything including Go cache
	@go clean -cache
	@go clean -modcache
	@echo "✅ Everything cleaned"

run: build ## Build and run without auth
	@$(MAKE) serve

run-auth: build-auth ## Build and run with OAuth authorization
	@$(MAKE) serve-auth

serve: ## Serve already built mcp-http-server.wasm (no auth)
	@if command -v wasmtime >/dev/null 2>&1; then \
		echo "Starting server with wasmtime (no auth)..."; \
		wasmtime serve -Scli $(MCP_SERVER_WASM); \
	elif command -v spin >/dev/null 2>&1; then \
		echo "Starting server with spin (no auth)..."; \
		spin up --from $(MCP_SERVER_WASM); \
	else \
		echo "Error: No compatible runtime found. Please install wasmtime or spin."; \
		echo "  Install wasmtime: curl https://wasmtime.dev/install.sh -sSf | bash"; \
		echo "  Install spin: curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash"; \
		exit 1; \
	fi

serve-auth: ## Serve already built mcp-http-auth-server.wasm with OAuth
	@if command -v wasmtime >/dev/null 2>&1; then \
		echo "Starting server with wasmtime (OAuth enabled)..."; \
		if [ -n "$$JWT_ISSUER" ] && [ -n "$$JWT_AUDIENCE" ] && [ -n "$$JWT_JWKS_URI" ]; then \
			echo "Using environment variables for JWT configuration:"; \
			echo "  Issuer: $$JWT_ISSUER"; \
			echo "  Audience: $$JWT_AUDIENCE"; \
			echo "  JWKS URI: $$JWT_JWKS_URI"; \
			wasmtime serve -Scli -Sconfig \
				-Sconfig-var="jwt.expected_issuer=$$JWT_ISSUER" \
				-Sconfig-var="jwt.expected_audience=$$JWT_AUDIENCE" \
				-Sconfig-var="jwt.jwks_uri=$$JWT_JWKS_URI" \
				-Sconfig-var="oauth.resource_url=http://0.0.0.0:8080/mcp" \
				$(MCP_AUTH_SERVER_WASM); \
		else \
			echo "Using default test configuration. Set these environment variables to override:"; \
			echo "  JWT_ISSUER    - OAuth issuer URL (e.g., https://your-domain.authkit.app)"; \
			echo "  JWT_AUDIENCE  - Expected audience (e.g., client_YOUR_CLIENT_ID)"; \
			echo "  JWT_JWKS_URI  - JWKS endpoint (e.g., https://your-domain.authkit.app/oauth2/jwks)"; \
			echo ""; \
			wasmtime serve -Scli -Sconfig \
				-Sconfig-var="jwt.expected_issuer=https://test.example.com/" \
				-Sconfig-var="jwt.expected_audience=https://mcp.example.com" \
				-Sconfig-var="oauth.resource_url=http://0.0.0.0:8080/mcp" \
				$(MCP_AUTH_SERVER_WASM); \
		fi \
	else \
		echo "Error: wasmtime is required for auth mode (WASI config support)."; \
		echo "  Install wasmtime: curl https://wasmtime.dev/install.sh -sSf | bash"; \
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