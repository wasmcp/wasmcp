# Makefile for {{project_name}}

.PHONY: all setup build clean

# Output directory
OUT_DIR = target

# Default target - setup and build
all: setup build

# Install dependencies
setup:
	@npm install

# Build the component
build:
	@mkdir -p $(OUT_DIR)
	@npm run build

# Clean build artifacts
clean:
	@rm -rf $(OUT_DIR) build dist src/generated node_modules
