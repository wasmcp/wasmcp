# Makefile for initialize-handler component

COMPONENT = initialize-handler
PACKAGE = wasmcp:initialize-handler
VERSION = 0.3.0-alpha.59

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

.PHONY: build publish clean help

# Default target
build:
	@echo "Building $(COMPONENT) in $(MODE) mode..."
	@cargo component build $(BUILD_FLAGS)
	@echo "$(COMPONENT) built successfully!"

# Publish to registry
publish: build
	@echo "Publishing $(PACKAGE)@$(VERSION)..."
	wkg publish --package '$(PACKAGE)@$(VERSION)' \
		target/wasm32-wasip1/$(TARGET_DIR)/initialize_handler.wasm
	@echo "Published $(PACKAGE)@$(VERSION)"

clean:
	@cargo clean

help:
	@echo "$(COMPONENT) component"
	@echo ""
	@echo "Targets:"
	@echo "  build    - Build the component"
	@echo "  publish  - Build and publish to wkg registry"
	@echo "  clean    - Remove build artifacts"