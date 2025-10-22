.PHONY: all setup build clean wit-deps

# Default target
all: build

# Install required tools and dependencies
setup:
	@go mod download
	@go get -tool go.bytecodealliance.org/cmd/wit-bindgen-go
	@go mod tidy

# Resolve WIT dependencies
wit-deps:
	@wit-deps -m wit/deps.toml -d wit/deps -l wit/deps.lock

# Build the component
build: wit-deps
	@go generate ./...
	@tinygo build -target=wasip2 \
		-o calculator.wasm \
		--wit-package wit \
		--wit-world calculator-go \
		main.go

# Clean build artifacts
clean:
	@rm -rf internal/ calculator.wasm wit/deps/ wit/deps.lock
