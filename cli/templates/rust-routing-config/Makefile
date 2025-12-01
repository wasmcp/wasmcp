.PHONY: all setup build clean

all: build

# Install required tools
setup:
	@rustup target add wasm32-wasip2

# Build the component
build:
	@cargo build --target wasm32-wasip2 --release

# Clean build artifacts
clean:
	@cargo clean
