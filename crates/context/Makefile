# Makefile for rust

.PHONY: work setup wit build

# Output directory and component name
OUT_DIR = target/wasm32-wasip2/release
COMPONENT_NAME = rust

# Default target - full workflow
work: setup wit build

# Install required tools
setup:
	@rustup target add wasm32-wasip2

wit:
	@wkg wit fetch

# Build the component
build:
	@cargo build --target wasm32-wasip2 --release