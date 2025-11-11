.PHONY: all setup build clean wit work

# Default target
all: build

work: setup wit build

wit:
	@wit-deps update

# Install required tools
setup:
	@rustup target add wasm32-wasip2

# Build the component
build:
	@cargo build --target wasm32-wasip2 --release

# Clean build artifacts
clean:
	@cargo clean
