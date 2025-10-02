# Makefile for python-resources

.PHONY: all build clean setup help

# Python virtual environment
VENV = venv
PYTHON = $(VENV)/bin/python3
PIP = $(VENV)/bin/pip
COMPONENTIZE_PY = $(VENV)/bin/componentize-py

# Output directory
OUT_DIR = target

# Component file name
COMPONENT_NAME = python_resources

# Default target
all: build

# Help target
help:
	@echo "python-resources Makefile targets:"
	@echo "  make setup  - Setup Python virtual environment"
	@echo "  make build  - Build the Python component"
	@echo "  make clean  - Clean build artifacts"

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
		--world resources-handler \
		bindings .
	@echo "✅ Python bindings generated"

# Build the Python component using componentize-py
build: bindgen
	@echo "Building Python component..."
	@mkdir -p $(OUT_DIR)
	@$(COMPONENTIZE_PY) \
		--wit-path wit \
		--world resources-handler \
		componentize \
		app \
		-o $(OUT_DIR)/$(COMPONENT_NAME).wasm
	@echo "✅ Python component built successfully!"
	@echo "Component: $(OUT_DIR)/$(COMPONENT_NAME).wasm"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@rm -rf $(OUT_DIR) wit_world __pycache__ $(VENV) wit/deps wkg.lock
	@echo "✅ Clean complete!"
