# Makefile for http-transport

.PHONY: work setup wit build build-draft build-draft2 clean

# Output directory and component name
OUT_DIR = ../../target/wasm32-wasip2/release
COMPONENT_NAME = http_transport

# Default target - builds both versions
work: setup wit build-draft2 build-draft

# Install required tools
setup:
	@rustup target add wasm32-wasip2

wit:
	@wkg wit fetch

# Build the component for wasmtime/wash (uses @0.2.0-draft)
build-draft:
	@echo "Building http-transport for wasmtime/wash (draft)..."
	@cargo build --target wasm32-wasip2 --release
	@echo "✓ Draft version built: $(OUT_DIR)/$(COMPONENT_NAME).wasm"

# Build the component for spin (uses @0.2.0-draft2)
build-draft2:
	@echo "Building http-transport for spin (draft2)..."
	@cargo build --target wasm32-wasip2 --release --features draft2
	@mv $(OUT_DIR)/$(COMPONENT_NAME).wasm $(OUT_DIR)/$(COMPONENT_NAME)_draft2.wasm
	@echo "✓ Draft2 version built: $(OUT_DIR)/$(COMPONENT_NAME)_draft2.wasm"

# Alias for backward compatibility
build: build-draft

# Clean build artifacts
clean:
	@cargo clean