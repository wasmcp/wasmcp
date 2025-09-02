.PHONY: build test clean release publish-tools publish-auth help

# Component output paths
COMPONENT_WASM = target/wasm32-wasip2/release/mcp_transport_http.wasm

# Package versions
TOOLS_PKG = fastertools:mcp-http-tools-server@0.1.0
AUTH_PKG = fastertools:mcp-http-tools-auth-transport@0.1.0

# Build the transport component for release with wasm32-wasip2 target
release: ## Build release version with wasip2 target
	cargo component build --release --target wasm32-wasip2
	@echo "✅ Built transport component: $(COMPONENT_WASM)"

build: ## Build debug version
	cargo component build --target wasm32-wasip2

build-tools: ## Build with tools feature only
	cargo component build --release --target wasm32-wasip2 --features tools
	@echo "✅ Built tools-enabled transport: $(COMPONENT_WASM)"

build-auth: ## Build with auth and tools features
	cargo component build --release --target wasm32-wasip2 --features auth,tools
	@echo "✅ Built auth-enabled transport: $(COMPONENT_WASM)"

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean

publish-tools: build-tools ## Build and publish tools-only transport
	@echo "Publishing tools-only transport to registry..."
	wkg publish --package '$(TOOLS_PKG)' $(COMPONENT_WASM)
	@echo "✅ Published $(TOOLS_PKG)"

publish-auth: build-auth ## Build and publish auth-enabled transport
	@echo "Publishing auth-enabled transport to registry..."
	wkg publish --package '$(AUTH_PKG)' $(COMPONENT_WASM)
	@echo "✅ Published $(AUTH_PKG)"

publish-all: publish-tools publish-auth ## Publish both transport variants
	@echo "✅ Published all transport variants"

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)