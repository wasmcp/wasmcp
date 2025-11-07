.PHONY: work setup wit build clean lint

OUT_DIR = ../../target/wasm32-wasip2/release
COMPONENT_NAME = session_store

work: setup wit build

setup:
	@rustup target add wasm32-wasip2

wit:
	@wit-deps update

lint:
	@cargo clippy --fix --allow-dirty -- -D warnings
	@cargo fmt --all

build: lint
	@echo "Building sessions..."
	@cargo build --target wasm32-wasip2 --release
	@echo "✓ Built: $(OUT_DIR)/$(COMPONENT_NAME).wasm"

clean:
	@cargo clean
