# Makefile for {{ project_name }}

.PHONY: all build compose compose-stdio run run-stdio test server-test-stdio clean setup help

# Python virtual environment
VENV = venv
PYTHON = $(VENV)/bin/python3
PIP = $(VENV)/bin/pip
COMPONENTIZE_PY = $(VENV)/bin/componentize-py

# Output directory
OUT_DIR = target

# Component file name
COMPONENT_NAME = {{ package_name }}

# Server files
SERVER = mcp-server.wasm
SERVER_STDIO = mcp-server-stdio.wasm

# Default target
all: compose

# Help target
help:
	@echo "{{ project_name }} Makefile targets:"
	@echo "  make setup            - Setup Python virtual environment"
	@echo "  make build            - Build the Python component"
	@echo "  make compose          - Compose the MCP server (HTTP transport)"
	@echo "  make compose-stdio    - Compose the MCP server (stdio transport)"
	@echo "  make run              - Run the HTTP server"
	@echo "  make run-stdio        - Run the stdio server with MCP Inspector"
	@echo "  make test             - Test the HTTP server"
	@echo "  make server-test-stdio - Test the stdio server"
	@echo "  make clean            - Clean build artifacts"

# Setup Python virtual environment
setup: $(VENV)

$(VENV):
	@echo "Creating virtual environment..."
	@python3 -m venv $(VENV)
	@$(PIP) install --upgrade pip --quiet
	@echo "Installing componentize-py..."
	@$(PIP) install componentize-py --quiet
	@echo "✅ Virtual environment ready"

# Generate Python bindings from WIT
bindgen: $(VENV)
	@echo "Fetching WIT dependencies..."
	@wkg wit fetch
	@echo "Generating Python bindings from WIT..."
	@rm -rf wit_world
	@$(COMPONENTIZE_PY) \
		--wit-path wit \
		--world {{ handler_type }}-handler \
		bindings .
	@echo "✅ Python bindings generated"

# Build the Python component using componentize-py
build: bindgen
	@echo "Building Python component..."
	@mkdir -p $(OUT_DIR)
	@$(COMPONENTIZE_PY) \
		--wit-path wit \
		--world {{ handler_type }}-handler \
		componentize \
		app \
		-o $(OUT_DIR)/$(COMPONENT_NAME).wasm
	@echo "✅ Python component built successfully!"
	@echo "Component: $(OUT_DIR)/$(COMPONENT_NAME).wasm"

# Compose MCP server (HTTP transport)
compose: build
	@echo "Composing MCP server (HTTP transport)..."
	@wasmcp compose --{{ handler_type }} $(OUT_DIR)/$(COMPONENT_NAME).wasm -o $(SERVER)
	@echo "✅ Composed: $(SERVER)"

# Compose MCP server (stdio transport)
compose-stdio: build
	@echo "Composing MCP server (stdio transport)..."
	@wasmcp compose --{{ handler_type }} $(OUT_DIR)/$(COMPONENT_NAME).wasm --transport stdio -o $(SERVER_STDIO)
	@echo "✅ Composed: $(SERVER_STDIO)"

# Run the HTTP server
run: compose
	@echo "Starting MCP server on http://0.0.0.0:8080..."
	@wasmtime serve -Scommon $(SERVER)

# Run the stdio server with MCP Inspector
run-stdio: compose-stdio
	@echo "Starting MCP server with stdio transport (MCP Inspector)..."
	@npx @modelcontextprotocol/inspector wasmtime run -Shttp $(SERVER_STDIO)

# Test the HTTP server
test:
	@echo "Testing {{ handler_type }}/list..."
	@npx @modelcontextprotocol/inspector@0.16.8 --cli http://localhost:8080/mcp \
		--method {{ handler_type }}/list | jq '.'

# Test the stdio server
server-test-stdio: compose-stdio
	@echo "Testing {{ handler_type }}/list (stdio)..."
	@npx @modelcontextprotocol/inspector@0.16.8 --cli --transport stdio --method {{ handler_type }}/list wasmtime run -Shttp $(SERVER_STDIO) | jq '.'

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf $(OUT_DIR) wit_world __pycache__ $(VENV) wit/deps wkg.lock deps $(SERVER) $(SERVER_STDIO)
	@echo "✅ Clean complete!"
