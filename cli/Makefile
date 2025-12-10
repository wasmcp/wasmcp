# Makefile for wasmcp CLI (native binary only)

# Detect platform
UNAME_S := $(shell uname -s)
UNAME_M := $(shell uname -m)

# Determine native target
ifeq ($(UNAME_S),Darwin)
    ifeq ($(UNAME_M),arm64)
        TARGET := aarch64-apple-darwin
    else
        TARGET := x86_64-apple-darwin
    endif
else ifeq ($(UNAME_S),Linux)
    ifeq ($(UNAME_M),aarch64)
        TARGET := aarch64-unknown-linux-gnu
    else ifeq ($(UNAME_M),arm64)
        TARGET := aarch64-unknown-linux-gnu
    else
        TARGET := x86_64-unknown-linux-gnu
    endif
else
    $(error Unsupported platform: $(UNAME_S))
endif

# Build directories
BUILD_DIR := target/$(TARGET)
RELEASE_BIN := $(BUILD_DIR)/release/wasmcp
DEBUG_BIN := $(BUILD_DIR)/debug/wasmcp

.PHONY: all build release debug clean test check install cross

# Default target
work: release 

# Release build (optimized)
release: build test
	@echo "Building wasmcp CLI for $(TARGET) (release)..."
	cargo build --release --target $(TARGET)
	@echo "✓ Built: $(RELEASE_BIN)"

# Debug build (faster compile)
debug: build test
	@echo "Building wasmcp CLI for $(TARGET) (debug)..."
	cargo build --target $(TARGET)
	@echo "✓ Built: $(DEBUG_BIN)"

# Alias for release
build: check

# Clean build artifacts
clean:
	cargo clean

# Run tests
test:
	cargo test --target $(TARGET)

# Run clippy and fmt checks
check:
	cargo clippy --allow-dirty --fix --target $(TARGET) -- -D warnings
	cargo fmt

# Install to ~/.cargo/bin
install: release
	@echo "Installing wasmcp to ~/.cargo/bin..."
	cargo install --path . --target $(TARGET)
	@echo "✓ Installed: wasmcp"

# Run clippy for Linux target (matches CI behavior)
cross:
	@echo "Running clippy for x86_64-unknown-linux-gnu (CI target)..."
	cross clippy --all-targets --target x86_64-unknown-linux-gnu -- -D warnings
	@echo "✓ Clippy passed for x86_64-unknown-linux-gnu"

# Show detected configuration
info:
	@echo "Platform: $(UNAME_S) $(UNAME_M)"
	@echo "Target: $(TARGET)"
	@echo "Release binary: $(RELEASE_BIN)"
	@echo "Debug binary: $(DEBUG_BIN)"

# Help
help:
	@echo "wasmcp CLI Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  make           - Build release binary (default)"
	@echo "  make release   - Build optimized release binary"
	@echo "  make debug     - Build debug binary (faster)"
	@echo "  make test      - Run tests"
	@echo "  make check     - Run clippy and fmt checks"
	@echo "  make cross     - Run clippy for x86_64-linux (matches CI)"
	@echo "  make clean     - Remove build artifacts"
	@echo "  make install   - Install to ~/.cargo/bin"
	@echo "  make info      - Show platform and target info"
	@echo "  make help      - Show this help"
	@echo ""
	@echo "Detected configuration:"
	@echo "  Platform: $(UNAME_S) $(UNAME_M)"
	@echo "  Target: $(TARGET)"
