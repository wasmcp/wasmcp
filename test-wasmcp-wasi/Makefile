.PHONY: build test clean registry-push check-cargo-component

# Check if cargo-component is installed, install if not
check-cargo-component:
	@command -v cargo-component >/dev/null 2>&1 || { \
		echo "cargo-component not found, installing..."; \
		echo "This is required for building Rust WebAssembly components"; \
		if command -v cargo-binstall >/dev/null 2>&1; then \
			echo "Using cargo-binstall for faster installation"; \
			cargo binstall cargo-component -y; \
		else \
			echo "cargo-binstall not found, installing from source (this may take a few minutes)"; \
			cargo install cargo-component; \
		fi; \
		echo "cargo-component installed successfully!"; \
	}

build: check-cargo-component
	cd handler && cargo component build --release --target wasm32-wasip1

test:
	cd handler && cargo test

clean:
	cd handler && cargo clean

registry-push: build
	@echo "Pushing to registry..."
	@VERSION=$$(cd handler && cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version') && \
	USERNAME=$$(git config user.name | tr '[:upper:]' '[:lower:]' | tr ' ' '-') && \
	PROJECT={{project-name | snake_case}} && \
	echo "Pushing ghcr.io/$$USERNAME/$$PROJECT:$$VERSION" && \
	wkg oci push ghcr.io/$$USERNAME/$$PROJECT:$$VERSION handler/target/wasm32-wasip1/release/$$PROJECT.wasm

up:
	spin up