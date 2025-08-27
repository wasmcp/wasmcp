.PHONY: build test clean release

# Build the gateway component for release with wasm32-wasip2 target
release: ## Build release version with wasip2 target
	cargo component build --release --target wasm32-wasip2
	@echo "✅ Built gateway component: target/wasm32-wasip2/release/wasmcp_spin.wasm"

build: ## Build debug version
	cargo component build --target wasm32-wasip2

test: ## Run tests
	cargo test

clean: ## Clean build artifacts
	cargo clean

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)