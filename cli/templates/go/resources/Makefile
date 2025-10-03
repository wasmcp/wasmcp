# Makefile for {{ project_name }}

.PHONY: work setup build

# Output directory and component name
OUT_DIR = target
COMPONENT_NAME = {{ package_name }}

# Default target - full workflow
work: setup build

# Install Go tools and dependencies
setup:
	@GOFLAGS="-tags=setup" go mod download
	@go install go.bytecodealliance.org/cmd/wit-bindgen-go@latest

# Generate bindings and build the component
build:
	@rm -rf gen
	@wit-bindgen-go generate --versioned --out gen --package-root github.com/{{ package_name }}/gen wit
	@find gen -name "*.wit.go" -exec sed -i 's/content_: content_/content: content_/g' {} +
	@go mod tidy
	@mkdir -p $(OUT_DIR)
	@tinygo build -target=wasip2 -o $(OUT_DIR)/$(COMPONENT_NAME).wasm --wit-package wit --wit-world {{ world_name }} -scheduler=asyncify -no-debug .
