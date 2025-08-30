.PHONY: build test clean run help setup venv-check

# Component paths
PROVIDER_WASM = weather_py_new-provider.wasm
MCP_SERVER_WASM = mcp-http-server.wasm

# Transport package from registry
TRANSPORT_PKG = fastertools:mcp-transport-http@0.1.0

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
	@chmod +x setup.sh
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

build: $(VENV) ## Build and compose everything into mcp-http-server.wasm
	@echo "Building provider component..."
	@$(COMPONENTIZE_PY) \
		--wit-path wit \
		--world weather-py-new \
		--import-interface-name "wasi:http/types@0.2.3=types" \
		componentize \
		app \
		-o $(PROVIDER_WASM)
	@echo "Downloading transport from registry..."
	@wkg get $(TRANSPORT_PKG) -o .transport-tmp.wasm 2>/dev/null || true
	@echo "Plugging provider into transport..."
	@wac plug --plug $(PROVIDER_WASM) .transport-tmp.wasm -o $(MCP_SERVER_WASM)
	@rm -f .transport-tmp.wasm
	@echo "✅ Created mcp-http-server.wasm ($$( ls -lh $(MCP_SERVER_WASM) 2>/dev/null | awk '{print $$5}' ))"

bindgen: $(VENV) ## Generate Python bindings for development
	@echo "Generating Python bindings..."
	@rm -rf wit_world
	@$(COMPONENTIZE_PY) \
		--wit-path wit \
		--world weather-py-new \
		--import-interface-name "wasi:http/types@0.2.3=types" \
		--import-interface-name "wasi:http/outgoing-handler@0.2.3=outgoing_handler" \
		--import-interface-name "wasi:io/poll@0.2.0=io_poll" \
		--import-interface-name "wasi:io/streams@0.2.0=io_streams" \
		bindings .

clean: ## Clean build artifacts
	rm -f $(PROVIDER_WASM) $(MCP_SERVER_WASM) .transport-tmp.wasm
	rm -rf wit_world __pycache__

clean-all: clean ## Clean everything including venv
	rm -rf $(VENV)

run: build ## Build and run with wasmtime serve
	wasmtime serve -Scli mcp-http-server.wasm

serve: ## Serve already built mcp-http-server.wasm
	wasmtime serve -Scli mcp-http-server.wasm

test-local: $(VENV) ## Test the Python code locally (without WASM)
	@echo "Testing Python implementation locally..."
	@$(PYTHON) app.py

# Testing the composed component
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