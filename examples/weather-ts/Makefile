.PHONY: all setup build clean wit work

OUT_DIR = dist
COMPONENT_NAME = weather

work: setup wit build

wit:
	@wit-deps update

# Default target - build (setup if needed)
all: node_modules build

# Install dependencies
node_modules:
	@npm install --silent

setup: node_modules

# Build the component
build:
	@mkdir -p $(OUT_DIR)
	@npm run build --silent

# Clean build artifacts
clean:
	@rm -rf node_modules build dist src/generated
