.PHONY: all setup build clean

VENV = venv
PIP = $(VENV)/bin/pip
COMPONENTIZE_PY = $(VENV)/bin/componentize-py
COMPONENT_NAME = strings

# Default target - build (setup if needed)
all: $(VENV) build

# Setup Python virtual environment and install dependencies
$(VENV):
	@python3 -m venv $(VENV)
	@$(PIP) install --upgrade pip --quiet
	@$(PIP) install componentize-py --quiet

setup: $(VENV)

# Generate bindings and build the component
build:
	@rm -rf wit_world
	@. $(VENV)/bin/activate && componentize-py --wit-path wit --world strings bindings .
	@. $(VENV)/bin/activate && componentize-py --wit-path wit --world strings componentize app -o $(COMPONENT_NAME).wasm

# Clean build artifacts
clean:
	@rm -rf $(VENV) wit_world __pycache__ $(COMPONENT_NAME).wasm
