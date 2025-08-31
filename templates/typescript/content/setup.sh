#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Setting up TypeScript MCP Handler...${NC}"
echo ""

# Check Node.js version
echo "Checking Node.js version..."
if ! command -v node &> /dev/null; then
    echo -e "${RED}Node.js is not installed. Please install Node.js 20 or later.${NC}"
    exit 1
fi

NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
REQUIRED_VERSION="20"

if [ "$NODE_VERSION" -lt "$REQUIRED_VERSION" ]; then
    echo -e "${RED}Node.js v$NODE_VERSION found, but Node.js v$REQUIRED_VERSION or later is required.${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Node.js $(node -v)${NC}"

# Check npm
echo ""
echo "Checking npm..."
if ! command -v npm &> /dev/null; then
    echo -e "${RED}npm is not installed.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ npm $(npm -v)${NC}"

# Install npm dependencies
echo ""
echo "Installing npm dependencies..."
npm install --silent
echo -e "${GREEN}✓ npm dependencies installed${NC}"

# Function to detect OS and architecture
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"
    
    case "$OS" in
        Linux*)     OS_NAME="linux";;
        Darwin*)    OS_NAME="macos";;
        *)          OS_NAME="unknown";;
    esac
    
    case "$ARCH" in
        x86_64)     ARCH_NAME="x86_64";;
        aarch64|arm64) ARCH_NAME="aarch64";;
        *)          ARCH_NAME="unknown";;
    esac
}

# Function to install tool from GitHub releases
install_tool() {
    local TOOL_NAME=$1
    local GITHUB_REPO=$2
    local VERSION=$3
    local BINARY_PATTERN=$4
    
    echo -e "${YELLOW}$TOOL_NAME is not installed.${NC}"
    echo ""
    
    detect_platform
    
    if [ "$OS_NAME" = "unknown" ] || [ "$ARCH_NAME" = "unknown" ]; then
        echo -e "${RED}Could not detect platform. Please install $TOOL_NAME manually:${NC}"
        echo "  cargo install $5"
        return 1
    fi
    
    # Construct download URL based on tool and platform
    case "$TOOL_NAME" in
        wkg)
            if [ "$OS_NAME" = "linux" ]; then
                BINARY_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/wkg-${ARCH_NAME}-unknown-linux-gnu"
            elif [ "$OS_NAME" = "macos" ]; then
                BINARY_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/wkg-${ARCH_NAME}-apple-darwin"
            fi
            ;;
        wac)
            if [ "$OS_NAME" = "linux" ]; then
                BINARY_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/wac-cli-${ARCH_NAME}-unknown-linux-gnu"
            elif [ "$OS_NAME" = "macos" ]; then
                BINARY_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/wac-cli-${ARCH_NAME}-apple-darwin"
            fi
            ;;
        *)
            echo -e "${RED}Unknown tool: $TOOL_NAME${NC}"
            return 1
            ;;
    esac
    
    echo "Would you like to install $TOOL_NAME automatically? (y/n)"
    read -r response
    
    if [[ "$response" != "y" ]]; then
        echo -e "${YELLOW}Please install $TOOL_NAME manually and run setup again.${NC}"
        echo "  Using cargo: cargo install $5"
        return 1
    fi
    
    echo "Downloading $TOOL_NAME from:"
    echo "  $BINARY_URL"
    echo ""
    
    # Download and install
    TEMP_FILE="/tmp/$TOOL_NAME-download"
    if curl -L -o "$TEMP_FILE" "$BINARY_URL"; then
        chmod +x "$TEMP_FILE"
        
        # Try to move to /usr/local/bin (may require sudo)
        if mv "$TEMP_FILE" "/usr/local/bin/$TOOL_NAME" 2>/dev/null; then
            echo -e "${GREEN}✓ $TOOL_NAME installed to /usr/local/bin${NC}"
        elif sudo mv "$TEMP_FILE" "/usr/local/bin/$TOOL_NAME" 2>/dev/null; then
            echo -e "${GREEN}✓ $TOOL_NAME installed to /usr/local/bin (with sudo)${NC}"
        else
            # Fall back to local bin directory
            mkdir -p "$HOME/.local/bin"
            mv "$TEMP_FILE" "$HOME/.local/bin/$TOOL_NAME"
            echo -e "${GREEN}✓ $TOOL_NAME installed to ~/.local/bin${NC}"
            echo -e "${YELLOW}  Make sure ~/.local/bin is in your PATH${NC}"
        fi
    else
        echo -e "${RED}Failed to download $TOOL_NAME${NC}"
        echo "Please install manually:"
        echo "  cargo install $5"
        return 1
    fi
}

# Check for wkg
echo ""
echo "Checking for wkg..."
if command -v wkg &> /dev/null; then
    echo -e "${GREEN}✓ wkg $(wkg --version 2>&1 | head -n1)${NC}"
else
    install_tool "wkg" "bytecodealliance/wasm-pkg-tools" "v0.6.0" "wkg" "wasm-pkg-tools"
fi

# Check for wac
echo ""
echo "Checking for wac..."
if command -v wac &> /dev/null; then
    echo -e "${GREEN}✓ wac $(wac --version 2>&1 | head -n1)${NC}"
else
    install_tool "wac" "bytecodealliance/wac" "v0.8.0" "wac-cli" "wac-cli"
fi

# Setup WIT dependencies
echo ""
echo "Setting up WIT dependencies..."
if [ ! -f "wit/deps/mcp/mcp.wit" ]; then
    echo "Fetching MCP WIT files..."
    wkg wit fetch
    echo -e "${GREEN}✓ WIT dependencies fetched${NC}"
else
    echo -e "${GREEN}✓ WIT dependencies already present${NC}"
fi

# Check for optional tools
echo ""
echo "Checking optional tools..."

# Check for wasmtime
if command -v wasmtime &> /dev/null; then
    echo -e "${GREEN}✓ wasmtime $(wasmtime --version 2>&1 | head -n1)${NC}"
else
    echo -e "${YELLOW}⚠ wasmtime not found (optional)${NC}"
    echo "  Install: curl https://wasmtime.dev/install.sh -sSf | bash"
fi

# Check for spin
if command -v spin &> /dev/null; then
    echo -e "${GREEN}✓ spin $(spin --version 2>&1 | head -n1)${NC}"
else
    echo -e "${YELLOW}⚠ spin not found (optional)${NC}"
    echo "  Install: curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash"
fi

echo ""
echo -e "${GREEN}✨ Setup complete!${NC}"
echo ""
echo "Next steps:"
echo "  1. Build the component: make build"
echo "  2. Run the server: make run"
echo ""
echo "Available commands:"
echo "  make help     - Show all available commands"
echo "  make build    - Build the MCP server component"
echo "  make run      - Build and run the server"
echo "  make clean    - Clean build artifacts"
echo "  make typecheck - Run TypeScript type checking"