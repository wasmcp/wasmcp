.PHONY: build install clean help

# Build the dev-tools binary for native target
build:
	@cargo build --release --target aarch64-apple-darwin

# Install dev-tools to /usr/local/bin (requires sudo)
install: build
	@cp ../target/aarch64-apple-darwin/release/dev-tools /usr/local/bin/wasmcp-dev
	@echo "✓ Installed as 'wasmcp-dev'"

# Clean build artifacts
clean:
	@cargo clean

# Show help
help:
	@echo "dev-tools Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  build    - Build dev-tools binary"
	@echo "  install  - Install to /usr/local/bin as 'wasmcp-dev'"
	@echo "  clean    - Clean build artifacts"
	@echo "  help     - Show this help"
