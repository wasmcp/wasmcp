# Hash MCP Server - Go Example
#
# Demonstrates building MCP tools using TinyGo and WebAssembly Components

.PHONY: all build clean generate fix-codegen test

# Build the WebAssembly component
all: build

# Generate Go bindings from WIT and apply codegen fix
generate:
	@echo "Generating Go bindings from WIT..."
	go generate ./...
	@echo "Applying codegen fix for wit-bindgen-go bug..."
	./fix-codegen.sh

# Build the component
build: generate
	@echo "Building WebAssembly component..."
	tinygo build -target=wasip2 -wit-package ./wit -wit-world hash -o hash.wasm .
	@ls -lh hash.wasm | awk '{print "✓ Built hash.wasm (" $$5 ")"}'

# Clean generated files and build artifacts
clean:
	rm -rf gen/
	rm -f hash.wasm
	@echo "✓ Cleaned build artifacts"

# Test the component (requires wasmcp CLI)
test: build
	@if command -v wasmcp >/dev/null 2>&1; then \
		echo "Testing hash component..."; \
		wasmcp test hash.wasm; \
	else \
		echo "⚠ wasmcp CLI not found, skipping tests"; \
	fi
