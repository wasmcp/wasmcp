.PHONY: help sync-versions validate-versions bump-all-patch bump-all-minor bump-all-major

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

sync-versions: ## Sync versions across the repository
	@echo "Syncing versions..."
	@./scripts/sync-versions.sh

sync-wit: ## Sync WIT files to mcp-http-component
	@./scripts/sync-wit.sh

validate-versions: ## Check if versions are in sync
	@echo "Validating versions..."
	@./scripts/sync-versions.sh
	@if [ -n "$$(git status --porcelain)" ]; then \
		echo "❌ Version inconsistency detected!"; \
		git status --short; \
		exit 1; \
	else \
		echo "✅ All versions are in sync!"; \
	fi

# Individual component bumps
bump-gateway-patch: ## Bump mcp-http-component patch version
	@current=$$(grep 'mcp-http-component = ' versions.toml | cut -d'"' -f2); \
	new=$$(echo $$current | awk -F. '{print $$1"."$$2"."$$3+1}'); \
	./scripts/bump-version.sh mcp-http-component $$new

bump-gateway-minor: ## Bump mcp-http-component minor version
	@current=$$(grep 'mcp-http-component = ' versions.toml | cut -d'"' -f2); \
	new=$$(echo $$current | awk -F. '{print $$1"."$$2+1".0"}'); \
	./scripts/bump-version.sh mcp-http-component $$new

bump-rust-patch: ## Bump ftl-sdk-rust patch version
	@current=$$(grep 'ftl-sdk-rust = ' versions.toml | cut -d'"' -f2); \
	new=$$(echo $$current | awk -F. '{print $$1"."$$2"."$$3+1}'); \
	./scripts/bump-version.sh ftl-sdk-rust $$new

bump-rust-minor: ## Bump ftl-sdk-rust minor version
	@current=$$(grep 'ftl-sdk-rust = ' versions.toml | cut -d'"' -f2); \
	new=$$(echo $$current | awk -F. '{print $$1"."$$2+1".0"}'); \
	./scripts/bump-version.sh ftl-sdk-rust $$new

bump-ts-patch: ## Bump ftl-sdk-typescript patch version
	@current=$$(grep 'ftl-sdk-typescript = ' versions.toml | cut -d'"' -f2); \
	new=$$(echo $$current | awk -F. '{print $$1"."$$2"."$$3+1}'); \
	./scripts/bump-version.sh ftl-sdk-typescript $$new

bump-ts-minor: ## Bump ftl-sdk-typescript minor version
	@current=$$(grep 'ftl-sdk-typescript = ' versions.toml | cut -d'"' -f2); \
	new=$$(echo $$current | awk -F. '{print $$1"."$$2+1".0"}'); \
	./scripts/bump-version.sh ftl-sdk-typescript $$new

# Bump all packages
bump-all-patch: ## Bump all packages patch version
	@echo "Bumping all packages (patch)..."
	@$(MAKE) bump-gateway-patch
	@$(MAKE) bump-rust-patch
	@$(MAKE) bump-ts-patch
	@echo ""
	@echo "✅ All packages bumped!"
	@echo ""
	@echo "Don't forget to:"
	@echo "  1. Review changes: git diff"
	@echo "  2. Commit: git commit -am 'chore: bump all packages patch version'"
	@echo "  3. Create tags:"
	@echo "     git tag mcp-http-component-v$$(grep 'mcp-http-component = ' versions.toml | cut -d'"' -f2)"
	@echo "     git tag ftl-sdk-rust-v$$(grep 'ftl-sdk-rust = ' versions.toml | cut -d'"' -f2)"
	@echo "     git tag ftl-sdk-typescript-v$$(grep 'ftl-sdk-typescript = ' versions.toml | cut -d'"' -f2)"
	@echo "  4. Push: git push origin main --tags"

bump-all-minor: ## Bump all packages minor version
	@echo "Bumping all packages (minor)..."
	@$(MAKE) bump-gateway-minor
	@$(MAKE) bump-rust-minor
	@$(MAKE) bump-ts-minor
	@echo ""
	@echo "✅ All packages bumped!"
	@echo ""
	@echo "Note: Minor version bumps should include new features."
	@echo "Make sure your changes warrant a minor version bump."

# Install targets
install-rust-tools: ## Install required Rust tools
	@which cargo-component > /dev/null || cargo binstall cargo-component --no-confirm
	@which wasm-tools > /dev/null || cargo binstall wasm-tools --no-confirm

install-ts-deps: ## Install TypeScript dependencies
	cd src/ftl-sdk-typescript && npm ci

install-deps: install-rust-tools install-ts-deps ## Install all dependencies

# Build targets
build-gateway: ## Build mcp-http-component
	cd src/mcp-http-component && cargo component build --release

build-rust-sdk: ## Build ftl-sdk-rust
	cd src/ftl-sdk-rust && cargo build --release

build-ts-sdk: ## Build ftl-sdk-typescript
	cd src/ftl-sdk-typescript && npm run build

build-all: build-gateway build-rust-sdk build-ts-sdk ## Build all components

# Test targets
test-gateway: ## Test mcp-http-component
	cd src/mcp-http-component && cargo test

test-rust-sdk: ## Test ftl-sdk-rust
	cd src/ftl-sdk-rust && cargo test

test-rust: test-gateway test-rust-sdk ## Run all Rust tests

test-ts: ## Run TypeScript tests
	cd src/ftl-sdk-typescript && npm test

test-all: test-rust test-ts ## Run all tests

# CI targets
ci-setup: install-deps sync-wit ## Setup CI environment

ci-build: ci-setup build-all ## CI build pipeline

ci-test: test-all ## CI test pipeline

ci: ci-build ci-test ## Run full CI pipeline

# Clean targets
clean: ## Clean all build artifacts
	cd src/mcp-http-component && cargo clean
	cd src/ftl-sdk-rust && cargo clean
	cd src/ftl-sdk-typescript && rm -rf dist node_modules

# Release helper
show-versions: ## Show current versions
	@echo "Current versions:"
	@echo "  mcp-http-component: $$(grep 'mcp-http-component = ' versions.toml | cut -d'"' -f2)"
	@echo "  ftl-sdk-rust:       $$(grep 'ftl-sdk-rust = ' versions.toml | cut -d'"' -f2)"
	@echo "  ftl-sdk-typescript: $$(grep 'ftl-sdk-typescript = ' versions.toml | cut -d'"' -f2)"
	@echo "  WIT package:        $$(grep '^mcp = ' versions.toml | cut -d'"' -f2)"

get-gateway-version: ## Get mcp-http-component version
	@grep 'mcp-http-component = ' versions.toml | cut -d'"' -f2

get-rust-sdk-version: ## Get ftl-sdk-rust version
	@grep 'ftl-sdk-rust = ' versions.toml | cut -d'"' -f2

get-ts-sdk-version: ## Get ftl-sdk-typescript version
	@grep 'ftl-sdk-typescript = ' versions.toml | cut -d'"' -f2

# Publishing targets
publish-gateway: ## Publish mcp-http-component to ghcr.io
	@echo "Publishing mcp-http-component..."
	@version=$$(make get-gateway-version); \
	cd src/mcp-http-component && \
	wkg oci push ghcr.io/bowlofarugula/mcp-http-component:$$version \
		target/wasm32-wasip1/release/mcp_http_component.wasm && \
	wkg oci push ghcr.io/bowlofarugula/mcp-http-component:latest \
		target/wasm32-wasip1/release/mcp_http_component.wasm
	@echo "✅ Published mcp-http-component v$$(make get-gateway-version)"

publish-rust-sdk: ## Publish ftl-sdk-rust to crates.io
	@echo "Publishing ftl-sdk-rust to crates.io..."
	cd src/ftl-sdk-rust && cargo publish
	@echo "✅ Published ftl-sdk v$$(make get-rust-sdk-version)"

publish-rust-sdk-dry: ## Dry run publish ftl-sdk-rust
	cd src/ftl-sdk-rust && cargo publish --dry-run

publish-ts-sdk: ## Publish ftl-sdk-typescript to npm
	@echo "Publishing @fastertools/ftl-sdk to npm..."
	cd src/ftl-sdk-typescript && npm publish --access public
	@echo "✅ Published @fastertools/ftl-sdk v$$(make get-ts-sdk-version)"

publish-ts-sdk-dry: ## Dry run publish ftl-sdk-typescript
	cd src/ftl-sdk-typescript && npm publish --dry-run --access public

publish-all: ## Publish all packages (use with caution!)
	@echo "⚠️  Publishing all packages..."
	@echo "This will publish:"
	@echo "  - mcp-http-component v$$(make get-gateway-version) to ghcr.io"
	@echo "  - ftl-sdk v$$(make get-rust-sdk-version) to crates.io"
	@echo "  - @fastertools/ftl-sdk v$$(make get-ts-sdk-version) to npm"
	@echo ""
	@echo "Press Ctrl+C to cancel, or Enter to continue..."
	@read confirm
	@$(MAKE) publish-gateway
	@$(MAKE) publish-rust-sdk
	@$(MAKE) publish-ts-sdk
	@echo ""
	@echo "🎉 All packages published!"

publish-dry-run: ## Dry run all publishes
	@echo "Dry run for all packages:"
	@echo ""
	@echo "=== mcp-http-component ==="
	@echo "Would publish v$$(make get-gateway-version) to ghcr.io"
	@echo ""
	@echo "=== ftl-sdk-rust ==="
	@$(MAKE) publish-rust-sdk-dry
	@echo ""
	@echo "=== ftl-sdk-typescript ==="
	@$(MAKE) publish-ts-sdk-dry

# Release workflow targets
release-patch: ## Full release workflow for patch version
	@echo "Starting patch release..."
	@$(MAKE) bump-all-patch
	@echo ""
	@echo "Changes made. Please:"
	@echo "1. Review: git diff"
	@echo "2. Commit: git commit -am 'chore: release patch version'"
	@echo "3. Tag and push:"
	@echo "   git tag mcp-http-component-v$$(make get-gateway-version)"
	@echo "   git tag ftl-sdk-rust-v$$(make get-rust-sdk-version)"
	@echo "   git tag ftl-sdk-typescript-v$$(make get-ts-sdk-version)"
	@echo "   git push origin main --tags"
	@echo ""
	@echo "GitHub Actions will handle publishing when tags are pushed."

release-minor: ## Full release workflow for minor version
	@echo "Starting minor release..."
	@$(MAKE) bump-all-minor
	@echo ""
	@echo "Changes made. Please:"
	@echo "1. Review: git diff"
	@echo "2. Commit: git commit -am 'chore: release minor version'"
	@echo "3. Tag and push (same as patch release)"