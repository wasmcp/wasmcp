.PHONY: all setup build clean wit work compose server

# Default target
all: server

work: setup wit build compose

wit:
	@wit-deps update

# Install required tools
setup:
	@rustup target add wasm32-wasip2

# Build the component
build:
	@cargo build --target wasm32-wasip2 --release

# Compose into MCP server
compose: build
	@../../cli/target/aarch64-apple-darwin/release/wasmcp compose server \
		target/wasm32-wasip2/release/todo_list_auth.wasm \
		-o mcp-server.wasm \
		--runtime spin \
		--force

# Alias for compose
server: compose

# Clean build artifacts
clean:
	@cargo clean
	@rm -f mcp-server.wasm
