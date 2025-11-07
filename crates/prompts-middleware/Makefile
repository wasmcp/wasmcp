.PHONY: work setup wit build build-wasm clean lint

work: setup wit build

setup:
	@rustup target add wasm32-wasip2

wit:
	@wit-deps update

lint:
	@cargo clippy --fix --allow-dirty -- -D warnings
	@cargo fmt --all

build: lint build-wasm

build-wasm:
	@echo "Building ..."
	@cargo build --target wasm32-wasip2 --release
	@echo "✓ Build successful"

clean:
	@cargo clean
