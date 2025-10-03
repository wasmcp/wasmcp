# Makefile for ts-logger

.PHONY: work setup build

# Default target - full workflow
work: setup build

# Install npm dependencies
setup:
	@npm install

# Build the TypeScript component
build:
	@mkdir -p target
	@npm run build
