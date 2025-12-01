.PHONY: all setup build clean wit work compose server test

# Default target
all: server

work: setup wit build compose

wit:
	@wit-deps update

# Install required tools
setup:
	@rustup target add wasm32-wasip2

# Build routing-config component
build:
	@echo "Building routing-config component..."
	@cargo build --target wasm32-wasip2 --release

# Build all dependencies (filter-middleware, calculator, todo-list-auth)
build-deps:
	@echo "Building filter-middleware..."
	@cd ../../crates/filter-middleware && cargo build --target wasm32-wasip2 --release
	@echo "Building calculator-rs..."
	@cd ../calculator-rs && cargo build --target wasm32-wasip2 --release
	@echo "Building todo-list-auth..."
	@cd ../todo-list-auth && cargo build --target wasm32-wasip2 --release

# Compose all components into final MCP server
# Note: routing_config.wasm exposes BOTH configs (routing://config + config://routing-team-override)
compose: build build-deps
	@echo "Composing filtered MCP server with multi-config..."
	@../../cli/target/aarch64-apple-darwin/release/wasmcp compose server \
		../../target/wasm32-wasip2/release/filter_middleware.wasm \
		target/wasm32-wasip2/release/routing_config.wasm \
		../calculator-rs/target/wasm32-wasip2/release/calculator.wasm \
		../todo-list-auth/target/wasm32-wasip2/release/todo_list_auth.wasm \
		-o mcp-server.wasm \
		--runtime spin \
		--force
	@echo "✓ Composed server with multi-config: mcp-server.wasm"

# Alias for compose
server: compose

# Run test scenarios
test: server
	@./scripts/test-filtering.sh

# Clean build artifacts
clean:
	@cargo clean
	@rm -f mcp-server.wasm
