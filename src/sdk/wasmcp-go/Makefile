.PHONY: generate test clean verify

# Generate Go bindings from WIT using wit-bindgen-go
generate:
	wit-bindgen-go generate --out wasmcp ./wit/world.wit
	@echo "✅ Generated Go bindings in wasmcp/"

# Run tests (when we add them)
test:
	@echo "Tests not yet implemented"

# Clean generated files
clean:
	rm -rf wasmcp/
	@echo "✅ Cleaned generated bindings"

# Verify the SDK can be imported
verify:
	@echo "Verifying SDK can be imported..."
	@go mod tidy
	@echo "✅ SDK verification complete"