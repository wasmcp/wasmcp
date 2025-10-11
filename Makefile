.PHONY: test test-unit test-integration test-memory test-compose clean build

# Default target
all: test

# Run all tests (unit + integration)
test: test-unit test-integration

# Run unit tests for protocol crate on native target
# These test pure logic (base64 encoding, JSON utilities, etc.)
#
# We must override the wasm32-wasip2 target set in .cargo/config.toml
# because unit tests need to run natively for fast iteration
test-unit:
	@echo "Running unit tests on native target..."
	cargo test -p protocol --target $$(rustc -vV | grep host | cut -d' ' -f2)

# Run integration tests by composing WASI components
# These test full protocol stack including streaming, component composition, and WASI integration
#
# Process:
# 1. Create pre-generated test files
# 2. Build all components (protocol, protocol-integration-tests, output-passthrough)
# 3. Compose them using wac plug
# 4. Run composed component through wasmtime
test-integration:
	@echo "Preparing test files..."
	@mkdir -p /tmp/wasmcp-test
	@dd if=/dev/zero of=/tmp/wasmcp-test/test_10mb.bin bs=1048576 count=10 2>/dev/null
	@echo "Building components for integration tests..."
	@cargo build -p protocol -p protocol-integration-tests -p output-passthrough --target wasm32-wasip2 -q
	@echo "Composing components with wac..."
	@wac plug --plug target/wasm32-wasip2/debug/protocol.wasm target/wasm32-wasip2/debug/protocol_integration_tests.wasm -o target/wasm32-wasip2/debug/test-with-protocol.wasm
	@wac plug --plug target/wasm32-wasip2/debug/output_passthrough.wasm target/wasm32-wasip2/debug/test-with-protocol.wasm -o target/wasm32-wasip2/debug/test-composed.wasm
	@echo "Running composed integration tests..."
	@wasmtime run --dir=/tmp target/wasm32-wasip2/debug/test-composed.wasm

# Build all components
build:
	cargo build --workspace --target wasm32-wasip2 --release

# Clean build artifacts
clean:
	cargo clean

# Run tests with verbose output
test-verbose: test-unit
	@echo "Running integration tests with verbose output..."
	@cargo build -p protocol -p protocol-integration-tests -p output-passthrough --target wasm32-wasip2 -q
	@wac plug --plug target/wasm32-wasip2/debug/protocol.wasm target/wasm32-wasip2/debug/protocol_integration_tests.wasm -o target/wasm32-wasip2/debug/test-with-protocol.wasm
	@wac plug --plug target/wasm32-wasip2/debug/output_passthrough.wasm target/wasm32-wasip2/debug/test-with-protocol.wasm -o target/wasm32-wasip2/debug/test-composed.wasm
	@wasmtime run --dir=/tmp target/wasm32-wasip2/debug/test-composed.wasm 2>&1

# Watch and run unit tests on file changes (requires cargo-watch)
test-watch:
	cargo watch -x 'test -p protocol --target $$(rustc -vV | grep host | cut -d' ' -f2)'

# Run integration tests with memory profiling enabled
# This builds with the memory-profiling feature flag and runs comprehensive memory tests
test-memory:
	@echo "Preparing test files..."
	@mkdir -p /tmp/wasmcp-test
	@dd if=/dev/zero of=/tmp/wasmcp-test/test_10mb.bin bs=1048576 count=10 2>/dev/null
	@dd if=/dev/zero of=/tmp/wasmcp-test/test_1mb.bin bs=1048576 count=1 2>/dev/null
	@dd if=/dev/zero of=/tmp/wasmcp-test/test_5mb.bin bs=1048576 count=5 2>/dev/null
	@dd if=/dev/zero of=/tmp/wasmcp-test/test_25mb.bin bs=1048576 count=25 2>/dev/null
	@dd if=/dev/zero of=/tmp/wasmcp-test/test_50mb.bin bs=1048576 count=50 2>/dev/null
	@dd if=/dev/zero of=/tmp/wasmcp-test/test_100mb.bin bs=1048576 count=100 2>/dev/null
	@echo "Building components with memory profiling enabled..."
	@cargo build -p protocol -p output-passthrough --target wasm32-wasip2 -q
	@cargo build -p protocol-integration-tests --target wasm32-wasip2 --features memory-profiling -q
	@echo "Composing components with wac..."
	@wac plug --plug target/wasm32-wasip2/debug/protocol.wasm target/wasm32-wasip2/debug/protocol_integration_tests.wasm -o target/wasm32-wasip2/debug/test-with-protocol-mem.wasm
	@wac plug --plug target/wasm32-wasip2/debug/output_passthrough.wasm target/wasm32-wasip2/debug/test-with-protocol-mem.wasm -o target/wasm32-wasip2/debug/test-composed-mem.wasm
	@echo "Running memory profiling tests..."
	@echo ""
	@wasmtime run --dir=/tmp target/wasm32-wasip2/debug/test-composed-mem.wasm
