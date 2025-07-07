.PHONY: help sync-versions validate-versions bump-all-patch bump-all-minor bump-all-major

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

sync-versions: ## Sync versions across the repository
	@echo "Syncing versions..."
	@./scripts/sync-versions.sh

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

# Build targets
build-gateway: ## Build mcp-http-component
	cd src/mcp-http-component && cargo component build --release

build-rust-sdk: ## Build ftl-sdk-rust
	cd src/ftl-sdk-rust && cargo build --release

build-ts-sdk: ## Build ftl-sdk-typescript
	cd src/ftl-sdk-typescript && npm run build

build-all: build-gateway build-rust-sdk build-ts-sdk ## Build all components

# Test targets
test-rust: ## Run Rust tests
	cd src/mcp-http-component && cargo test
	cd src/ftl-sdk-rust && cargo test

test-ts: ## Run TypeScript tests
	cd src/ftl-sdk-typescript && npm test

test-all: test-rust test-ts ## Run all tests

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