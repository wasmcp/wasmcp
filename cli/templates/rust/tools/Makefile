# Makefile for {{ project_name }}

.PHONY: work setup build

# Output directory and component name
OUT_DIR = target/wasm32-wasip2/release
COMPONENT_NAME = {{ package_name }}

# Default target - full workflow
work: setup build

# Install required tools
setup:
	@rustup target add wasm32-wasip2

# Build the component
build:
	@cargo build --target wasm32-wasip2 --release
