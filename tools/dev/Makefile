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

.PHONY: build install clean help info

# Build the dev-tools binary (as example)
build:
	@echo "Building dev-tools example for $(TARGET)..."
	@cargo build --release --example dev-tools --target $(TARGET)
	@echo "✓ Built: target/$(TARGET)/release/examples/dev-tools"

# Install dev-tools to ~/.cargo/bin using cargo install
install:
	@echo "Installing dev-tools to ~/.cargo/bin..."
	@cargo install --path . --example dev-tools --target $(TARGET)
	@echo "✓ Installed as 'dev-tools' in ~/.cargo/bin"

# Clean build artifacts
clean:
	@cargo clean

# Show detected configuration
info:
	@echo "Platform: $(UNAME_S) $(UNAME_M)"
	@echo "Target: $(TARGET)"

# Show help
help:
	@echo "dev-tools Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  build    - Build dev-tools example binary"
	@echo "  install  - Install to ~/.cargo/bin as 'dev-tools'"
	@echo "  clean    - Clean build artifacts"
	@echo "  info     - Show platform and target info"
	@echo "  help     - Show this help"
	@echo ""
	@echo "Detected configuration:"
	@echo "  Platform: $(UNAME_S) $(UNAME_M)"
	@echo "  Target: $(TARGET)"
