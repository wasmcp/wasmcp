.PHONY: build build-auth test clean run run-auth help

# Component paths
PROVIDER_WASM = weather_js-provider.wasm
MCP_SERVER_WASM = mcp-http-server.wasm
MCP_AUTH_SERVER_WASM = mcp-http-auth-server.wasm

# Registry packages
TRANSPORT_PKG = fastertools:mcp-transport-http-tools@0.1.0
TRANSPORT_AUTH_PKG = fastertools:mcp-transport-http-tools-auth@0.1.0
AUTH_PKG = fastertools:mcp-authorization@0.1.0

# Provider package for publishing
PROVIDER_PKG = fastertools:weather_js-provider@0.1.0

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
	@npx jco componentize bundled.js --wit wit --world-name weather-js --out $(PROVIDER_WASM)
	@echo "Downloading transport from registry..."
	@wkg get $(TRANSPORT_PKG) -o .transport-tmp.wasm
	@echo "Plugging provider into transport..."
	@wac plug --plug $(PROVIDER_WASM) .transport-tmp.wasm -o $(MCP_SERVER_WASM)
	@rm -f .transport-tmp.wasm bundled.js
	@echo "✅ Created mcp-http-server.wasm ($$( ls -lh $(MCP_SERVER_WASM) | awk '{print $$5}' ))"

build-auth: ## Build and compose with OAuth authorization enabled
	@echo "Installing dependencies..."
	@npm install
	@echo "Bundling JavaScript modules..."
	@npm run bundle
	@echo "Building provider component..."
	@npx jco componentize bundled.js --wit wit --world-name weather-js --out $(PROVIDER_WASM)
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
	@rm -f .auth-tmp.wasm .transport-auth-tmp.wasm .transport-with-auth.wasm bundled.js
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

build-types: ## Generate TypeScript type definitions
	jco types wit -o types

clean: ## Clean build artifacts
	rm -f $(PROVIDER_WASM) $(MCP_SERVER_WASM) $(MCP_AUTH_SERVER_WASM) .transport-tmp.wasm .transport-auth-tmp.wasm .transport-with-auth.wasm .auth-tmp.wasm bundled.js
	rm -rf dist types

run: build ## Build and run without auth
	@$(MAKE) serve

run-auth: build-auth ## Build and run with OAuth authorization
	@$(MAKE) serve-auth

serve: ## Serve already built mcp-http-server.wasm (no auth)
	@if command -v wasmtime >/dev/null 2>&1; then \
		echo "Starting server with wasmtime (no auth)..."; \
		wasmtime serve -Scli mcp-http-server.wasm; \
	elif command -v spin >/dev/null 2>&1; then \
		echo "Starting server with spin (no auth)..."; \
		spin up --from mcp-http-server.wasm; \
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
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"multi_weather","arguments":{"cities":["Tokyo","London","New York"]}},"id":5}' | python3 -m json.tool

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
