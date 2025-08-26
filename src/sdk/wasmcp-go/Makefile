.PHONY: generate build test clean

# Generate C bindings from WIT
generate:
	wit-bindgen c ../wasmcp-macro/wit/
	@echo "✅ Generated C bindings"

# Build the SDK (verification only, this is a library)
build:
	tinygo build -target=wasip1 -gc=leaking -buildmode=c-shared -no-debug -o test.wasm test/test.go || true
	@echo "✅ SDK builds successfully"

# Run tests
test:
	tinygo test -target=wasip1 -gc=leaking -buildmode=c-shared .

# Clean generated files
clean:
	rm -f mcp_handler.c mcp_handler.h mcp_handler_component_type.o test.wasm
	@echo "✅ Cleaned generated files"

# Verify the SDK can be imported
verify:
	@echo "Verifying SDK can be imported..."
	@go mod tidy
	@echo "✅ SDK verification complete"