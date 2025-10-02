# Makefile for ts-logger

.PHONY: all build clean setup help

# Default target
all: build

# Help target
help:
	@echo "ts-logger Makefile targets:"
	@echo "  make setup  - Install npm dependencies"
	@echo "  make build  - Build the TypeScript component"
	@echo "  make clean  - Clean build artifacts"

# Setup npm dependencies
setup:
	@echo "Installing npm dependencies..."
	@npm install
	@echo "✅ Dependencies installed!"

# Build the TypeScript component
build: setup
	@echo "Fetching WIT dependencies..."
	@wkg wit fetch
	@echo "Building TypeScript component..."
	@mkdir -p target
	@npm run build
	@echo "✅ TypeScript component built successfully!"
	@echo "Component: target/ts_logger.wasm"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf node_modules build target src/generated wit/deps wkg.lock package-lock.json
	@echo "✅ Clean complete!"
