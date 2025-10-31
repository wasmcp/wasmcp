.PHONY: work setup wit build build-draft build-draft2 clean

OUT_DIR = ../../target/wasm32-wasip2/release
COMPONENT_NAME = sessions

work: setup wit build-draft2 build-draft

setup:
	@rustup target add wasm32-wasip2

wit:
	@wit-deps update
	@WIT_DEPS_PATH=wit-draft2 wit-deps update

build-draft:
	@echo "Building sessions for wasmtime/wash (draft)..."
	@cargo build --target wasm32-wasip2 --release
	@echo "✓ Draft version built: $(OUT_DIR)/$(COMPONENT_NAME).wasm"

build-draft2:
	@echo "Building sessions for spin (draft2)..."
	@cargo build --target wasm32-wasip2 --release --features draft2
	@mv $(OUT_DIR)/$(COMPONENT_NAME).wasm $(OUT_DIR)/$(COMPONENT_NAME)_draft2.wasm
	@echo "✓ Draft2 version built: $(OUT_DIR)/$(COMPONENT_NAME)_draft2.wasm"

build: build-draft

clean:
	@cargo clean
