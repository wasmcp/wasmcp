# Makefile for {{project_name}}

.PHONY: all setup build clean

# Python virtual environment
VENV = venv
PIP = $(VENV)/bin/pip
COMPONENTIZE_PY = $(VENV)/bin/componentize-py

# Output directory and component name
OUT_DIR = target
COMPONENT_NAME = {{project_name}}

# Default target - setup and build
all: setup build

# Setup Python virtual environment and install dependencies
setup:
	@python3 -m venv $(VENV)
	@$(PIP) install --upgrade pip --quiet
	@$(PIP) install componentize-py --quiet

# Generate bindings and build the component
build:
	@rm -rf wit_world
	@. $(VENV)/bin/activate && $(COMPONENTIZE_PY) --wit-path wit --world {{project_name}} bindings .
	@mkdir -p $(OUT_DIR)
	@. $(VENV)/bin/activate && $(COMPONENTIZE_PY) --wit-path wit --world {{project_name}} componentize app -o $(OUT_DIR)/$(COMPONENT_NAME).wasm

# Clean build artifacts
clean:
	@rm -rf $(OUT_DIR) wit_world $(VENV)
