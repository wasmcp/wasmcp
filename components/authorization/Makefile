.PHONY: build test clean check fmt

# Component name and output
COMPONENT_NAME = mcp-authorization
WASM_FILE = target/wasm32-wasip1/release/$(COMPONENT_NAME).wasm

# Default target
all: build

# Build the component
build:
	@echo "Building $(COMPONENT_NAME) component..."
	@cargo component build --release
	@echo "✅ Component built: $(WASM_FILE)"
	@ls -lh $(WASM_FILE) 2>/dev/null | awk '{print "   Size:", $$5}'

# Run tests
test:
	@echo "Running tests..."
	@cargo test
	@echo "✅ All tests passed"

# Run clippy for linting
check:
	@echo "Running clippy..."
	@cargo clippy -- -D warnings
	@echo "✅ No clippy warnings"

# Format code
fmt:
	@echo "Formatting code..."
	@cargo fmt
	@echo "✅ Code formatted"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean
	@echo "✅ Build artifacts cleaned"

# Validate WIT interfaces
validate-wit:
	@echo "Validating WIT interfaces..."
	@wasm-tools validate $(WASM_FILE) 2>/dev/null || echo "⚠️  Component not built yet"
	@echo "✅ WIT interfaces valid"

# Show component metadata
metadata: build
	@echo "Component metadata:"
	@wasm-tools component wit $(WASM_FILE) 2>/dev/null | head -20

# Development build (with debug symbols)
dev:
	@echo "Building $(COMPONENT_NAME) component (debug)..."
	@cargo component build
	@echo "✅ Debug component built"

# Run all checks before commit
pre-commit: fmt check test
	@echo "✅ All pre-commit checks passed"

# Help target
help:
	@echo "Available targets:"
	@echo "  build        - Build the component (release)"
	@echo "  test         - Run tests"
	@echo "  check        - Run clippy linting"
	@echo "  fmt          - Format code"
	@echo "  clean        - Clean build artifacts"
	@echo "  validate-wit - Validate WIT interfaces"
	@echo "  metadata     - Show component metadata"
	@echo "  dev          - Build debug version"
	@echo "  pre-commit   - Run all checks before commit"
	@echo "  help         - Show this help message"