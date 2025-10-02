# Makefile for go-logger

.PHONY: all build clean setup bindgen help

# Output directory
OUT_DIR := target

# Component file name
COMPONENT_NAME := go_logger

# Default target
all: build

# Help target
help:
	@echo "go-logger Makefile targets:"
	@echo "  make setup   - Install Go tools and dependencies"
	@echo "  make bindgen - Generate Go bindings from WIT"
	@echo "  make build   - Build the Go component"
	@echo "  make clean   - Clean build artifacts"

# Setup Go tools
setup:
	@echo "Setting up Go environment..."
	@GOFLAGS="-tags=setup" go mod download
	@go install go.bytecodealliance.org/cmd/wit-bindgen-go@latest
	@echo "✅ Go environment ready"

# Generate bindings from WIT
bindgen: setup
	@echo "Fetching WIT dependencies..."
	@wkg wit fetch
	@echo "Generating Go bindings from WIT..."
	@rm -rf gen
	@wit-bindgen-go generate --versioned --out gen --package-root github.com/go_logger/gen wit
	@echo "Updating Go dependencies..."
	@go mod tidy
	@echo "✅ Go bindings generated"

# Build the Go component using TinyGo
build: bindgen
	@echo "Building Go component with TinyGo..."
	@mkdir -p $(OUT_DIR)
	@tinygo build -target=wasip2 \
		-o $(OUT_DIR)/$(COMPONENT_NAME).wasm \
		--wit-package wit \
		--wit-world middleware \
		-scheduler=asyncify \
		-no-debug \
		.
	@echo "✅ Go component built successfully!"
	@echo "Component: $(OUT_DIR)/$(COMPONENT_NAME).wasm"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf $(OUT_DIR) gen wit/deps wkg.lock
	@echo "✅ Clean complete!"
