# Makefile for {{project_name}}

.PHONY: all setup build clean

# Output directory
OUT_DIR = target/wasm32-wasip2/release

# Default target - setup and build
all: setup build

# Install required tools
setup:
	@rustup target add wasm32-wasip2

# Build the component
build:
	@cargo build --target wasm32-wasip2 --release

# Clean build artifacts
clean:
	@cargo clean
