# Makefile for rust-tools

.PHONY: all build clean help test

# Default target
all: build

# Help target
help:
	@echo "rust-tools Makefile targets:"
	@echo "  make build       - Build the Rust component"
	@echo "  make clean       - Clean build artifacts"
	@echo "  make test        - Run tests (if any)"

# Build the Rust component
build:
	@echo "Fetching WIT dependencies..."
	@wkg wit fetch
	@echo "Building Rust component..."
	@cargo component build --release
	@echo "✅ Component built: target/wasm32-wasip2/release/rust_tools.wasm"

# Run tests
test:
	@cargo test

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@rm -rf wit/deps wkg.lock
	@echo "✅ Clean complete!"
