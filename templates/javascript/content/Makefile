.PHONY: build test clean install registry-push

build: install
	cd handler && npm run build

install:
	cd handler && npm install

test: install
	cd handler && npm test

clean:
	cd handler && npm run clean
	cd handler && rm -rf node_modules

registry-push: build
	@echo "Pushing to registry..."
	@VERSION=$$(cd handler && node -p "require('./package.json').version") && \
	USERNAME=$$(git config user.name | tr '[:upper:]' '[:lower:]' | tr ' ' '-') && \
	PROJECT={{project-name | snake_case}} && \
	echo "Pushing ghcr.io/$$USERNAME/$$PROJECT:$$VERSION" && \
	wkg oci push ghcr.io/$$USERNAME/$$PROJECT:$$VERSION handler/dist/handler.wasm

up:
	spin up
