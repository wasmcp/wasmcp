# Makefile for building and composing MCP server configurations

# Build mode (debug or release)
MODE ?= release

# Build flags
ifeq ($(MODE),release)
	BUILD_FLAGS = --release
	TARGET_DIR = release
else
	BUILD_FLAGS =
	TARGET_DIR = debug
endif

# Component paths (infrastructure only)
COMPONENTS = request error-writer initialize-writer tools-writer resources-writer \
             http-transport stdio-transport initialize-handler

.PHONY: all clean build publish-all help

# Default target
all: build

# Help target
help:
	@echo "MCP Infrastructure Components"
	@echo ""
	@echo "Usage: make [target] [MODE=debug|release]"
	@echo ""
	@echo "Targets:"
	@echo "  all                    - Build all infrastructure components"
	@echo "  build                  - Build all infrastructure components"
	@echo "  publish-all            - Build and publish all components to wkg registry"
	@echo "  clean                  - Remove all built artifacts"
	@echo "  help                  - Show this help message"
	@echo ""
	@echo "Examples:"
	@echo "  make                        # Build all components in release mode"
	@echo "  make MODE=debug            # Build all components in debug mode"
	@echo "  make publish-all           # Build and publish to registry"

# Build all components
build:
	@echo "Building all components in $(MODE) mode..."
	@for component in $(COMPONENTS); do \
		echo "Building $$component..."; \
		(cd $$component && cargo component build $(BUILD_FLAGS)) || exit 1; \
	done
	@echo "All components built successfully!"

# Clean all build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@for component in $(COMPONENTS); do \
		echo "Cleaning $$component..."; \
		(cd $$component && cargo clean); \
	done
	@echo "Clean complete!"

clean-all:
	@for component in $(COMPONENTS); do \
		echo "Cleaning $$component..."; \
		(cd $$component && cargo clean); \
	done
	@echo "All components cleaned successfully!"

# Publish all components to registry
publish-all: build
	@echo "Publishing all components to wkg registry..."
	@echo "Publishing wasmcp:request@0.3.0-alpha.59..."
	@wkg publish --package 'wasmcp:request@0.3.0-alpha.59' ../target/wasm32-wasip1/$(TARGET_DIR)/request.wasm
	@echo "Publishing wasmcp:error-writer@0.3.0-alpha.59..."
	@wkg publish --package 'wasmcp:error-writer@0.3.0-alpha.59' ../target/wasm32-wasip1/$(TARGET_DIR)/error_writer.wasm
	@echo "Publishing wasmcp:initialize-writer@0.3.0-alpha.59..."
	@wkg publish --package 'wasmcp:initialize-writer@0.3.0-alpha.59' ../target/wasm32-wasip1/$(TARGET_DIR)/initialize_writer.wasm
	@echo "Publishing wasmcp:tools-writer@0.3.0-alpha.59..."
	@wkg publish --package 'wasmcp:tools-writer@0.3.0-alpha.59' ../target/wasm32-wasip1/$(TARGET_DIR)/tools_writer.wasm
	@echo "Publishing wasmcp:resources-writer@0.3.0-alpha.59..."
	@wkg publish --package 'wasmcp:resources-writer@0.3.0-alpha.59' ../target/wasm32-wasip1/$(TARGET_DIR)/resources_writer.wasm
	@echo "Publishing wasmcp:http-transport@0.3.0-alpha.59..."
	@wkg publish --package 'wasmcp:http-transport@0.3.0-alpha.59' ../target/wasm32-wasip1/$(TARGET_DIR)/http_transport.wasm
	@echo "Publishing wasmcp:stdio-transport@0.3.0-alpha.59..."
	@wkg publish --package 'wasmcp:stdio-transport@0.3.0-alpha.59' ../target/wasm32-wasip1/$(TARGET_DIR)/stdio_transport.wasm
	@echo "Publishing wasmcp:initialize-handler@0.3.0-alpha.59..."
	@wkg publish --package 'wasmcp:initialize-handler@0.3.0-alpha.59' ../target/wasm32-wasip1/$(TARGET_DIR)/initialize_handler.wasm
	@echo "All components published successfully!"

