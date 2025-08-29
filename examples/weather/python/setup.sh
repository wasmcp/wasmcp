#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Setting up Python MCP Handler...${NC}"
echo ""

# Check Python version
echo "Checking Python version..."
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}Python 3 is not installed. Please install Python 3.10 or later.${NC}"
    exit 1
fi

PYTHON_VERSION=$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
REQUIRED_VERSION="3.10"

if [[ $(echo "$PYTHON_VERSION < $REQUIRED_VERSION" | bc) -eq 1 ]]; then
    echo -e "${RED}Python $PYTHON_VERSION found, but Python $REQUIRED_VERSION or later is required.${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Python $PYTHON_VERSION${NC}"

# Create virtual environment
echo ""
echo "Creating Python virtual environment..."
if [ ! -d "venv" ]; then
    python3 -m venv venv
    echo -e "${GREEN}✓ Virtual environment created${NC}"
else
    echo -e "${YELLOW}Virtual environment already exists${NC}"
fi

# Activate virtual environment
source venv/bin/activate

# Upgrade pip
echo ""
echo "Upgrading pip..."
pip install --upgrade pip --quiet

# Install Python dependencies
echo ""
echo "Installing Python dependencies..."
pip install -r requirements.txt --quiet
echo -e "${GREEN}✓ Python dependencies installed${NC}"

# Check for required tools
echo ""
echo "Checking for required tools..."

check_tool() {
    if command -v $1 &> /dev/null; then
        echo -e "${GREEN}✓ $1${NC}"
        return 0
    else
        echo -e "${YELLOW}✗ $1 not found${NC}"
        return 1
    fi
}

MISSING_TOOLS=false

if ! check_tool wkg; then
    echo "  Install with: cargo install wkg"
    MISSING_TOOLS=true
fi

if ! check_tool wac; then
    echo "  Install with: cargo install wac-cli"
    MISSING_TOOLS=true
fi

if ! check_tool wasmtime; then
    echo "  Install from: https://wasmtime.dev/"
    MISSING_TOOLS=true
fi

# Generate bindings
echo ""
echo "Generating Python bindings..."
componentize-py --wit-path wit --world tools-handler bindings . 2>/dev/null || true
echo -e "${GREEN}✓ Bindings generated${NC}"

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Setup complete!${NC}"
echo ""
echo "Next steps:"
echo "  1. Activate the virtual environment:"
echo "     source venv/bin/activate"
echo ""
echo "  2. Build the component:"
echo "     make build"
echo ""
echo "  3. Run the server:"
echo "     make run"
echo ""

if [ "$MISSING_TOOLS" = true ]; then
    echo -e "${YELLOW}Note: Some tools are missing. Please install them before building.${NC}"
fi

echo -e "${GREEN}========================================${NC}"