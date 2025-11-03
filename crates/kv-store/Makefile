.PHONY: work setup wit build build-draft build-draft2 clean

OUT_DIR = ../../target/wasm32-wasip2/release
COMPONENT_NAME = kv_store

work: setup wit build-draft2 build-draft

setup:
	@rustup target add wasm32-wasip2

wit:
	@wit-deps update
	@wit-deps -d wit-draft2/deps -l wit-draft2/deps.lock -m wit-draft2/deps.toml

build-draft:
	@echo "Building kv-store for wasmtime/wash (draft)..."
	@cargo build --target wasm32-wasip2 --release
	@echo "✓ Draft version built: $(OUT_DIR)/$(COMPONENT_NAME).wasm"

build-draft2:
	@echo "Building kv-store for spin (draft2)..."
	@cargo build --target wasm32-wasip2 --release --features draft2
	@mv $(OUT_DIR)/$(COMPONENT_NAME).wasm $(OUT_DIR)/$(COMPONENT_NAME)-d2.wasm
	@echo "✓ Draft2 version built: $(OUT_DIR)/$(COMPONENT_NAME)-d2.wasm"

build: build-draft

clean:
	@cargo clean
