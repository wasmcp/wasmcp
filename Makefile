.PHONY: build clean otel-exporter http-transport compose

# Build everything
build: compose

# Build the otel-exporter component
otel-exporter:
	cd components/otel-exporter && cargo component build --target wasm32-wasip2 --release

# Build the http-transport component
http-transport:
	cd components/http-transport && cargo component build --target wasm32-wasip2 --release

# Compose otel-exporter with http-transport to create a complete transport
compose: otel-exporter http-transport
	wac compose --dep wasmcp:otel-exporter=./target/wasm32-wasip2/release/otel_exporter.wasm --dep mcp:transport-http=./target/wasm32-wasip2/release/mcp_transport_http.wasm transport-composition.wac -o target/wasm32-wasip2/release/mcp_transport_http_with_otel.wasm
	@echo "Composed transport available at: target/wasm32-wasip2/release/mcp_transport_http_with_otel.wasm"

# Clean build artifacts
clean:
	cargo clean
	cd components/otel-exporter && cargo clean
	cd components/http-transport && cargo clean

# For test app compatibility
test-compose: compose
	cp target/wasm32-wasip2/release/mcp_transport_http_with_otel.wasm /tmp/wasmcp-test/mcp_transport_http.wasm
	@echo "Composed component copied to test app"