.PHONY: build build-auth test clean run run-auth help

# Component paths
PROVIDER_WASM = target/wasm32-wasip1/release/weather_rs.wasm
MCP_SERVER_WASM = mcp-http-server.wasm
MCP_AUTH_SERVER_WASM = mcp-http-auth-server.wasm
AUTH_WASM = ../../components/authorization/target/wasm32-wasip1/release/mcp_authorization.wasm
TRANSPORT_WASM = ../../components/http-transport/target/wasm32-wasip1/release/mcp_transport_http.wasm

# Transport packages from registry (fallback if local not available)
TRANSPORT_PKG = fastertools:mcp-http-tools-server@0.1.0
TRANSPORT_AUTH_PKG = fastertools:mcp-http-tools-auth-server@0.1.0

# Provider package for publishing
PROVIDER_PKG = fastertools:weather-rs-provider@0.1.0

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: ## Check and install required tools
	@chmod +x setup.sh
	@./setup.sh

build: ## Build and compose everything into mcp-http-server.wasm (no auth)
	@echo "Building provider component..."
	@cargo component build --release --target wasm32-wasip1
	@echo "Building transport component (no auth)..."
	@cd ../../components/http-transport && cargo component build --release --no-default-features --features tools
	@echo "Plugging provider into transport..."
	@wac plug --plug $(PROVIDER_WASM) $(TRANSPORT_WASM) -o $(MCP_SERVER_WASM)
	@echo "✅ Created mcp-http-server.wasm ($$( ls -lh $(MCP_SERVER_WASM) | awk '{print $$5}' ))"

build-auth: ## Build and compose with OAuth authorization enabled
	@echo "Building provider component..."
	@cargo component build --release --target wasm32-wasip1
	@echo "Building authorization component..."
	@cd ../../components/authorization && cargo component build --release --target wasm32-wasip1
	@echo "Building auth-enabled transport..."
	@cd ../../components/http-transport && cargo component build --release --features "tools auth"
	@echo "Composing with authorization..."
	@echo "  Step 1: Plugging authorization into transport..."
	@wac plug --plug $(AUTH_WASM) $(TRANSPORT_WASM) -o .transport-with-auth.wasm
	@echo "  Step 2: Plugging provider into auth-enabled transport..."
	@wac plug --plug $(PROVIDER_WASM) .transport-with-auth.wasm -o $(MCP_AUTH_SERVER_WASM)
	@rm -f .transport-with-auth.wasm
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

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean
	rm -f $(MCP_SERVER_WASM) $(MCP_AUTH_SERVER_WASM) .transport-tmp.wasm .transport-auth-tmp.wasm .transport-with-auth.wasm

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
				mcp-http-auth-server.wasm; \
		else \
			echo "Using default test configuration. Set these environment variables to override:"; \
			echo "  JWT_ISSUER    - OAuth issuer URL (e.g., https://your-domain.authkit.app)"; \
			echo "  JWT_AUDIENCE  - Expected audience (e.g., client_YOUR_CLIENT_ID)"; \
			echo "  JWT_JWKS_URI  - JWKS endpoint (e.g., https://your-domain.authkit.app/oauth2/jwks)"; \
			echo ""; \
			wasmtime serve -Scli -Sconfig \
				-Sconfig-var="jwt.expected_issuer=https://test.example.com/" \
				-Sconfig-var="jwt.expected_audience=https://mcp.example.com" \
				mcp-http-auth-server.wasm; \
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

publish: build ## Build and publish provider component to registry
	@echo "Publishing weather Rust provider to registry..."
	wkg publish --package '$(PROVIDER_PKG)' $(PROVIDER_WASM)
	@echo "✅ Published $(PROVIDER_PKG)"