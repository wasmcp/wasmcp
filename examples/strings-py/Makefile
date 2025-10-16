# Makefile for strings

.PHONY: work setup build

# Python virtual environment
VENV = venv
PIP = $(VENV)/bin/pip
COMPONENTIZE_PY = $(VENV)/bin/componentize-py

# Output directory and component name
OUT_DIR = target
COMPONENT_NAME = strings

# Default target - full workflow
work: setup build

# Setup Python virtual environment and install dependencies
setup:
	@python3 -m venv $(VENV)
	@$(PIP) install --upgrade pip --quiet
	@$(PIP) install componentize-py --quiet

# Generate bindings and build the component
build:
	@rm -rf wit_world
	@$(COMPONENTIZE_PY) --wit-path wit --world strings bindings .
	@mkdir -p $(OUT_DIR)
	@$(COMPONENTIZE_PY) --wit-path wit --world strings componentize app -o $(OUT_DIR)/$(COMPONENT_NAME).wasm
