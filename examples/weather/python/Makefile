.PHONY: build test clean run help setup venv-check

# Component paths
HANDLER_WASM = weather-handler.wasm
COMPOSED_WASM = composed.wasm

# Server package from registry
SERVER_PKG = fastertools:wasmcp-server-tools@0.1.3

# Python virtual environment
VENV = venv
PYTHON = $(VENV)/bin/python3
PIP = $(VENV)/bin/pip
COMPONENTIZE_PY = $(VENV)/bin/componentize-py

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

setup: ## Run initial setup (creates venv, installs deps)
	@./setup.sh

$(VENV): ## Create virtual environment
	@echo "Creating virtual environment..."
	@python3 -m venv $(VENV)
	@$(PIP) install --upgrade pip --quiet
	@$(PIP) install -r requirements.txt --quiet
	@echo "✅ Virtual environment ready"

venv-check: ## Check if venv is activated
	@if [ -z "$$VIRTUAL_ENV" ]; then \
		echo "⚠️  Virtual environment not activated. Run: source venv/bin/activate"; \
		echo "   Or use 'make setup' for initial setup"; \
		exit 1; \
	fi

build: $(VENV) ## Build and compose everything into composed.wasm
	@echo "Building handler component..."
	@$(COMPONENTIZE_PY) \
		--wit-path wit \
		--world tools-handler \
		componentize \
		app \
		-o $(HANDLER_WASM)
	@echo "Downloading server from registry..."
	@wkg get $(SERVER_PKG) -o .server-tmp.wasm 2>/dev/null || true
	@echo "Plugging handler into server..."
	@wac plug --plug $(HANDLER_WASM) .server-tmp.wasm -o $(COMPOSED_WASM)
	@rm -f .server-tmp.wasm
	@echo "✅ Created composed.wasm ($$( ls -lh $(COMPOSED_WASM) 2>/dev/null | awk '{print $$5}' ))"

build-bindings: $(VENV) ## Generate Python bindings for development
	@echo "Generating Python bindings..."
	@$(COMPONENTIZE_PY) --wit-path wit --world tools-handler bindings . 2>/dev/null || true

clean: ## Clean build artifacts
	rm -f $(HANDLER_WASM) $(COMPOSED_WASM) .server-tmp.wasm
	rm -rf wit_world __pycache__

clean-all: clean ## Clean everything including venv
	rm -rf $(VENV)

run: build ## Build and run with wasmtime serve
	wasmtime serve -Scli composed.wasm

serve: ## Serve already built composed.wasm
	wasmtime serve -Scli composed.wasm

test-local: $(VENV) ## Test the Python code locally (without WASM)
	@echo "Testing Python implementation locally..."
	@$(PYTHON) app.py

# Testing the composed component - same format as Rust/JS examples
test-init: ## Test initialize endpoint
	@echo "Testing initialize..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}},"id":1}' | python3 -m json.tool

test-tools: ## Test tools/list endpoint
	@echo "Testing tools/list..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":2}' | python3 -m json.tool

test-echo: ## Test echo tool
	@echo "Testing echo tool..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello World"}},"id":4}' | python3 -m json.tool

test-weather: ## Test weather tool
	@echo "Testing get_weather tool..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"get_weather","arguments":{"location":"London"}},"id":3}' | python3 -m json.tool

test-multi: ## Test multi_weather tool
	@echo "Testing multi_weather tool..."
	@curl -s -X POST http://localhost:8080/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"multi_weather","arguments":{"cities":["London","Paris","Berlin"]}},"id":5}' | python3 -m json.tool