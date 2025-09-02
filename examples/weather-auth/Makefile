.PHONY: build test clean run help

# Component packages from registry
# These must be published before this example can work:
# 1. cd ../../components/http-transport && make publish-auth
# 2. cd ../../components/authorization && make publish  
# 3. cd ../weather-py && make publish
TRANSPORT_PKG = fastertools:mcp-http-tools-auth-transport@0.1.0
AUTH_PKG = fastertools:mcp-authorization@0.1.0
PROVIDER_PKG = fastertools:weather-py-provider@0.1.0

# Output file
MCP_SERVER_WASM = mcp-http-auth-server.wasm

# Temporary files
TRANSPORT_TMP = .transport-tmp.wasm
AUTH_TMP = .auth-tmp.wasm
PROVIDER_TMP = .provider-tmp.wasm
TRANSPORT_AUTH_TMP = .transport-auth-tmp.wasm

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Compose all components into auth-enabled MCP server
	@echo "Downloading components from registry..."
	@echo "1. Fetching auth-enabled transport..."
	@wkg get $(TRANSPORT_PKG) -o $(TRANSPORT_TMP) 2>/dev/null || { \
		echo "❌ Failed to fetch transport. Ensure it's published:"; \
		echo "   wkg publish --package '$(TRANSPORT_PKG)' <path-to-transport.wasm>"; \
		exit 1; \
	}
	@echo "2. Fetching authorization component..."
	@wkg get $(AUTH_PKG) -o $(AUTH_TMP) 2>/dev/null || { \
		echo "❌ Failed to fetch authorization. Ensure it's published:"; \
		echo "   wkg publish --package '$(AUTH_PKG)' <path-to-auth.wasm>"; \
		exit 1; \
	}
	@echo "3. Fetching weather provider..."
	@wkg get $(PROVIDER_PKG) -o $(PROVIDER_TMP) 2>/dev/null || { \
		echo "❌ Failed to fetch provider. Ensure it's published:"; \
		echo "   wkg publish --package '$(PROVIDER_PKG)' <path-to-provider.wasm>"; \
		exit 1; \
	}
	@echo ""
	@echo "Composing authorization-enabled MCP server..."
	@echo "Step 1: Plugging authorization into transport..."
	@wac plug --plug $(AUTH_TMP) $(TRANSPORT_TMP) -o $(TRANSPORT_AUTH_TMP)
	@echo "Step 2: Plugging provider into auth-transport..."
	@wac plug --plug $(PROVIDER_TMP) $(TRANSPORT_AUTH_TMP) -o $(MCP_SERVER_WASM)
	@rm -f $(TRANSPORT_TMP) $(AUTH_TMP) $(PROVIDER_TMP) $(TRANSPORT_AUTH_TMP)
	@echo "✅ Created $(MCP_SERVER_WASM) ($$( ls -lh $(MCP_SERVER_WASM) 2>/dev/null | awk '{print $$5}' ))"

build-local: ## Build using local components (for development)
	@echo "Building with local components..."
	@echo "1. Using local auth-enabled transport..."
	@cp ../../components/http-transport/target/wasm32-wasip2/release/mcp_transport_http.wasm $(TRANSPORT_TMP)
	@echo "2. Using local authorization component..."
	@cp ../../components/authorization/target/wasm32-wasip1/release/mcp_authorization.wasm $(AUTH_TMP)
	@echo "3. Using local weather-rs provider..."
	@cp ../weather-rs/target/wasm32-wasip1/release/weather_rs.wasm $(PROVIDER_TMP)
	@echo ""
	@echo "Composing authorization-enabled MCP server..."
	@echo "Step 1: Plugging authorization into transport..."
	@wac plug --plug $(AUTH_TMP) $(TRANSPORT_TMP) -o $(TRANSPORT_AUTH_TMP)
	@echo "Step 2: Plugging provider into auth-transport..."
	@wac plug --plug $(PROVIDER_TMP) $(TRANSPORT_AUTH_TMP) -o $(MCP_SERVER_WASM)
	@rm -f $(TRANSPORT_TMP) $(AUTH_TMP) $(PROVIDER_TMP) $(TRANSPORT_AUTH_TMP)
	@echo "✅ Created $(MCP_SERVER_WASM) ($$( ls -lh $(MCP_SERVER_WASM) 2>/dev/null | awk '{print $$5}' ))"

clean: ## Clean build artifacts
	@rm -f $(MCP_SERVER_WASM) .transport-auth.wasm
	@echo "✅ Cleaned build artifacts"

run: build ## Build and run with available runtime
	@$(MAKE) serve

serve: ## Serve already built mcp-http-auth-server.wasm
	@if command -v wasmtime >/dev/null 2>&1; then \
		echo "Starting server with wasmtime..."; \
		echo "Server will require OAuth authentication"; \
		wasmtime serve -Scli $(MCP_SERVER_WASM); \
	elif command -v spin >/dev/null 2>&1; then \
		echo "Starting server with spin..."; \
		echo "Server will require OAuth authentication"; \
		spin up --from $(MCP_SERVER_WASM); \
	else \
		echo "Error: No compatible runtime found. Please install wasmtime or spin."; \
		echo "  Install wasmtime: curl https://wasmtime.dev/install.sh -sSf | bash"; \
		echo "  Install spin: curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash"; \
		exit 1; \
	fi

# Testing endpoints

test-discovery: ## Test OAuth discovery endpoints
	@echo "Testing OAuth Protected Resource Metadata..."
	@curl -s http://localhost:8080/.well-known/oauth-protected-resource | python3 -m json.tool
	@echo ""
	@echo "Testing OAuth Authorization Server Metadata..."
	@curl -s http://localhost:8080/.well-known/oauth-authorization-server | python3 -m json.tool

test-no-auth: ## Test request without authentication (should fail)
	@echo "Testing request without token (should return 401)..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}' | python3 -m json.tool

test-with-auth: ## Test request with authentication (needs valid token)
	@echo "Testing request with Bearer token..."
	@echo "Note: Replace TOKEN with a valid JWT token"
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Authorization: Bearer TOKEN" \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}' | python3 -m json.tool

# Generate a test JWT token (for development only)
gen-test-token: ## Generate a test JWT token (requires Python with PyJWT)
	@echo "Generating test JWT token..."
	@python3 -c "import jwt; import time; \
		payload = { \
			'sub': 'test-user', \
			'iss': 'https://auth.example.com', \
			'aud': 'https://mcp.example.com', \
			'exp': int(time.time()) + 3600, \
			'iat': int(time.time()), \
			'scope': 'mcp:tools:read mcp:tools:write mcp:resources:read' \
		}; \
		secret = 'test-secret-key'; \
		token = jwt.encode(payload, secret, algorithm='HS256'); \
		print('Test token (valid for 1 hour):'); \
		print(token)"

# Environment setup for authorization
env-setup: ## Show required environment variables
	@echo "Set these environment variables for authorization:"
	@echo ""
	@echo "# JWT Validation"
	@echo "export MCP_EXPECTED_ISSUER=https://auth.example.com"
	@echo "export MCP_EXPECTED_AUDIENCE=https://mcp.example.com"
	@echo "export MCP_JWKS_URI=https://auth.example.com/.well-known/jwks.json"
	@echo ""
	@echo "# OAuth Discovery"
	@echo "export MCP_RESOURCE_URL=https://mcp.example.com"
	@echo "export MCP_AUTH_SERVER=https://auth.example.com"
	@echo "export MCP_AUTH_ENDPOINT=https://auth.example.com/authorize"
	@echo "export MCP_TOKEN_ENDPOINT=https://auth.example.com/token"
	@echo ""
	@echo "# For testing with self-signed tokens (development only)"
	@echo "export MCP_EXPECTED_ISSUER="
	@echo "export MCP_EXPECTED_AUDIENCE="

# Policy management
policy-default: ## Show default authorization policy
	@echo "Default authorization policy:"
	@cat ../../components/authorization/policies/default.rego

policy-rbac: ## Show RBAC authorization policy
	@echo "RBAC authorization policy:"
	@cat ../../components/authorization/policies/rbac.rego

policy-tool: ## Show tool authorization policy
	@echo "Tool authorization policy:"
	@cat ../../components/authorization/policies/tool-authorization.rego

# Development helpers
logs: ## Show server logs (if running with logging enabled)
	@echo "Note: Run server with RUST_LOG=debug for detailed logs"

validate-components: ## Validate all components are built
	@echo "Checking components..."
	@if [ -f $(AUTH_WASM) ]; then \
		echo "✅ Authorization component: $(AUTH_WASM)"; \
		echo "   Size: $$( ls -lh $(AUTH_WASM) | awk '{print $$5}' )"; \
	else \
		echo "❌ Authorization component not found"; \
	fi
	@if [ -f $(TRANSPORT_WASM) ]; then \
		echo "✅ Transport component: $(TRANSPORT_WASM)"; \
		echo "   Size: $$( ls -lh $(TRANSPORT_WASM) | awk '{print $$5}' )"; \
	else \
		echo "❌ Transport component not found"; \
	fi
	@if [ -f $(PROVIDER_WASM) ]; then \
		echo "✅ Provider component: $(PROVIDER_WASM)"; \
		echo "   Size: $$( ls -lh $(PROVIDER_WASM) | awk '{print $$5}' )"; \
	else \
		echo "❌ Provider component not found"; \
	fi

test-all: test-discovery test-no-auth ## Run all tests