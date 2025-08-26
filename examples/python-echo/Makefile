.PHONY: build test clean compose run run-wasmtime help install-deps

# Default gateway version to use
GATEWAY_VERSION ?= 0.0.5
GATEWAY_WASM ?= wasmcp-spin.wasm
HANDLER_WASM ?= handler/app.wasm
VENV ?= .venv

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

install-deps: ## Install Python dependencies
	@if [ ! -d "$(VENV)" ]; then \
		echo "Creating virtual environment..."; \
		python3 -m venv $(VENV); \
	fi
	@echo "Installing dependencies..."
	@$(VENV)/bin/pip install spin-sdk==3.4.1
	@$(VENV)/bin/pip install componentize-py
	@$(VENV)/bin/pip install -e ../../src/sdk/wasmcp-python

build: install-deps ## Build the handler component
	@echo "Building Python handler component using build.sh..."
	@cd handler && chmod +x build.sh && LOCAL_BUILD=true ./build.sh
	@echo "✅ Built handler/app.wasm"

test: install-deps ## Test the handler locally (Python only)
	@echo "Testing handler locally..."
	cd handler && PYTHONPATH=src ../$(VENV)/bin/python -c "\
import src.app as app;\
print('Handler created:', app.handler);\
print('Tools:', list(app.handler.tools.keys()));\
print('Resources:', list(app.handler.resources.keys()));\
print('Prompts:', list(app.handler.prompts.keys()));\
echo_tool = app.handler.tools['echo'];\
result = echo_tool.call({'message': 'Hello from test!'});\
print('Echo test result:', result)\
"

clean: ## Clean build artifacts
	rm -rf $(VENV)
	rm -f $(HANDLER_WASM)
	rm -f composed.wasm
	find . -name "*.pyc" -delete
	find . -name "__pycache__" -delete

compose: build ## Compose handler with gateway using wac plug
	@echo "Composing handler with gateway..."
	wac plug wasmcp-server.wasm --plug $(HANDLER_WASM) -o composed.wasm
	@echo "✅ Created composed.wasm"

run: build ## Run with Spin (handles dependencies automatically)
	spin up

run-wasmtime: compose ## Run with wasmtime (requires manual composition)
	wasmtime serve -Scli -Skeyvalue -Shttp composed.wasm

# Development targets
dev: build compose ## Build and compose for development
	@echo "Ready to run: make run or make run-wasmtime"

# Testing the composed component
test-tools: ## Test tools/list endpoint
	@echo "Testing tools/list..."
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}' | python3 -m json.tool

test-echo: ## Test echo tool
	@echo "Testing echo tool..."
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"echo","arguments":{"message":"Hello from Python MCP!"}},"id":1}' | python3 -m json.tool

test-weather: ## Test weather tool
	@echo "Testing weather tool..."
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"weather","arguments":{"location":"San Francisco"}},"id":1}' | python3 -m json.tool

test-reverse: ## Test reverse tool
	@echo "Testing reverse tool..."
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"reverse","arguments":{"text":"Hello World"}},"id":1}' | python3 -m json.tool

test-resources: ## Test resources/list endpoint
	@echo "Testing resources/list..."
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"resources/list","params":{},"id":1}' | python3 -m json.tool

test-prompts: ## Test prompts/list endpoint
	@echo "Testing prompts/list..."
	@curl -s -X POST http://localhost:3000/mcp \
		-H "Content-Type: application/json" \
		-d '{"jsonrpc":"2.0","method":"prompts/list","params":{},"id":1}' | python3 -m json.tool

test-all: test-tools test-echo test-weather test-reverse test-resources test-prompts ## Run all endpoint tests