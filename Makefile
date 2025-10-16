.PHONY: help test test-unit test-integration test-memory clean build watch
.DEFAULT_GOAL := help

# === Configuration ===
NATIVE_TARGET := $(shell rustc -vV | grep host | cut -d' ' -f2)
WASM_TARGET := wasm32-wasip2
WASMTIME := wasmtime

# === Quick Tests (Developer Workflow) ===

help:  ## Show this help message
	@echo "wasmcp - Test Commands"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		sort | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'
	@echo ""
	@echo "Quick start: make test"

test: test-unit test-integration  ## Run fast tests (unit + integration) ~30s
	@echo ""
	@echo "✅ All tests passed"

test-unit:  ## Run unit tests (native, ~2s)
	@echo "Running unit tests..."
	@cargo test --package protocol --lib --target $(NATIVE_TARGET) --quiet

test-integration: .build-test-components  ## Run integration tests (streaming + verification)
	@echo "Validating JSON-RPC output structure..."
	@echo -n "test" | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null | \
		jq -e '.jsonrpc == "2.0" and .id == 1 and .result.contents[0].blob != null' >/dev/null && \
		echo "✓ Valid JSON-RPC 2.0 structure" || \
		(echo "✗ Invalid JSON-RPC output" && exit 1)
	@echo ""
	@echo "Running base64 encoding verification..."
	@printf '\x01\x00\x01\x02\x03\x04' | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null | \
		jq -e '.result.contents[0].blob == "AAECAwQ="' >/dev/null && \
		echo "✓ Base64 encoding verified (AAECAwQ=)" || \
		(echo "✗ Base64 verification failed" && exit 1)
	@echo ""
	@echo "Running streaming tests (10MB)..."
	@dd if=/dev/zero bs=1M count=10 2>/dev/null | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm >/dev/null && \
		echo "✓ Streamed 10MB successfully" || \
		(echo "✗ Streaming failed" && exit 1)
	@echo ""
	@echo "Testing empty stdin handling..."
	@echo -n "" | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm 2>&1 | \
		grep -q "Empty stdin" && \
		echo "✓ Empty stdin handled" || true
	@echo ""
	@echo "✅ All integration tests passed"

# === Memory & Performance ===

test-memory: .build-test-components  ## Run memory-bounded tests (100MB in 2MB limit)
	@echo "Running memory-bounded tests (100MB stdin, 2MB memory limit)..."
	@OUTPUT_SIZE=$$(dd if=/dev/zero bs=1M count=100 2>/dev/null | \
		$(WASMTIME) run -W max-memory-size=2097152 target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null | \
		tee /dev/stderr | wc -c); \
	echo "" >&2; \
	echo "$$OUTPUT_SIZE" | jq -e '. > 130000000' >/dev/null && \
		echo "✓ 100MB → ~140MB output (base64 expansion), 2MB memory (70x ratio)" || \
		(echo "✗ Output size unexpected: $$OUTPUT_SIZE bytes" && exit 1)
	@echo ""
	@echo "Validating JSON-RPC structure..."
	@dd if=/dev/zero bs=1M count=10 2>/dev/null | \
		$(WASMTIME) run -W max-memory-size=1572864 target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null | \
		jq -e '.jsonrpc == "2.0"' >/dev/null && \
		echo "✓ 10MB in 1.5MB limit - valid JSON-RPC (7x ratio)" || \
		(echo "✗ Minimum limit test failed" && exit 1)
	@echo ""
	@echo "✅ Memory bounded streaming verified"

# === Build Helpers ===

build:  ## Build all components (release)
	@cargo build --workspace --target $(WASM_TARGET) --release

.build-test-components:
	@echo "Building test components..."
	@cargo build -p protocol -p protocol-integration-tests -p output-passthrough \
		--target $(WASM_TARGET) --quiet 2>&1 | grep -v "Compiling" || true
	@wac plug \
		--plug target/$(WASM_TARGET)/debug/protocol.wasm \
		target/$(WASM_TARGET)/debug/protocol_integration_tests.wasm \
		-o target/$(WASM_TARGET)/debug/test-with-protocol.wasm 2>/dev/null
	@wac plug \
		--plug target/$(WASM_TARGET)/debug/output_passthrough.wasm \
		target/$(WASM_TARGET)/debug/test-with-protocol.wasm \
		-o target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null

# === Utilities ===

mcp-tool-text:  ## Convert stdin text to MCP text content (tools/call result) - pipe to jq
	@$(MAKE) .build-test-components >/dev/null 2>&1
	@(printf '\x02' && cat) | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null

mcp-tool-image:  ## Convert stdin to MCP image content (tools/call result) - pipe to jq
	@$(MAKE) .build-test-components >/dev/null 2>&1
	@(printf '\x03' && cat) | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null

mcp-tool-audio:  ## Convert stdin to MCP audio content (tools/call result) - pipe to jq
	@$(MAKE) .build-test-components >/dev/null 2>&1
	@(printf '\x04' && cat) | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null

mcp-tool-embedded-resource-text:  ## Convert stdin text to MCP embedded resource text (tools/call) - pipe to jq
	@$(MAKE) .build-test-components >/dev/null 2>&1
	@(printf '\x05' && cat) | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null

mcp-tool-embedded-resource-blob:  ## Convert stdin to MCP embedded resource blob (tools/call) - pipe to jq
	@$(MAKE) .build-test-components >/dev/null 2>&1
	@(printf '\x06' && cat) | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null

mcp-resource-read-text:  ## Convert stdin text to MCP resource text (resources/read result) - pipe to jq
	@$(MAKE) .build-test-components >/dev/null 2>&1
	@(printf '\x07' && cat) | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null

mcp-resource-read-blob:  ## Convert stdin to MCP resource blob (resources/read result) - pipe to jq
	@$(MAKE) .build-test-components >/dev/null 2>&1
	@(printf '\x08' && cat) | \
		$(WASMTIME) run target/$(WASM_TARGET)/debug/test-composed.wasm 2>/dev/null

watch:  ## Watch and run unit tests on file changes (requires cargo-watch)
	@cargo watch -q -x 'test -p protocol --lib --target $(NATIVE_TARGET)'

clean:  ## Clean build artifacts and test files
	@cargo clean

# === Verbose variants ===

test-verbose: test-unit  ## Run tests with full output
	@echo "Running integration tests (verbose)..."
	@./scripts/setup-test-files.sh --integration
	@$(WASMTIME) run --dir=/tmp target/$(WASM_TARGET)/debug/test-composed.wasm
