#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Setting up Rust MCP Provider...${NC}"
echo ""

# Check Rust and cargo
echo "Checking Rust version..."
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Cargo is not installed. Please install Rust.${NC}"
    echo "  Install from: https://rustup.rs"
    exit 1
fi

RUST_VERSION=$(rustc --version | cut -d' ' -f2)
echo -e "${GREEN}✓ Rust $RUST_VERSION${NC}"
echo -e "${GREEN}✓ cargo $(cargo --version | cut -d' ' -f2)${NC}"

# Check cargo-component
echo ""
echo "Checking for cargo-component..."
if ! command -v cargo-component &> /dev/null; then
    echo -e "${YELLOW}cargo-component is not installed.${NC}"
    echo ""
    echo "Would you like to install cargo-component? [Y/n]"
    read -r RESPONSE
    RESPONSE=${RESPONSE:-Y}
    
    if [[ "$RESPONSE" =~ ^[Yy]$ ]]; then
        echo "Installing cargo-component..."
        if cargo install --locked cargo-component; then
            echo -e "${GREEN}✓ cargo-component installed${NC}"
        else
            echo -e "${RED}Failed to install cargo-component${NC}"
            echo "Please install manually with: cargo install --locked cargo-component"
            exit 1
        fi
    else
        echo "Please install cargo-component manually:"
        echo "  cargo install --locked cargo-component"
        exit 1
    fi
else
    echo -e "${GREEN}✓ cargo-component${NC}"
fi

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
            else
                BINARY_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/wkg-${ARCH_NAME}-apple-darwin"
            fi
            ;;
        wac)
            if [ "$OS_NAME" = "linux" ]; then
                BINARY_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/wac-cli-${ARCH_NAME}-unknown-linux-musl"
            else
                BINARY_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/wac-cli-${ARCH_NAME}-apple-darwin"
            fi
            ;;
    esac
    
    echo "Would you like to install $TOOL_NAME automatically? [Y/n]"
    read -r RESPONSE
    RESPONSE=${RESPONSE:-Y}
    
    if [[ "$RESPONSE" =~ ^[Yy]$ ]]; then
        echo "Downloading $TOOL_NAME from:"
        echo "  $BINARY_URL"
        
        # Create local bin directory if it doesn't exist
        mkdir -p "$HOME/.local/bin"
        
        # Download and install
        if curl -L "$BINARY_URL" -o "$HOME/.local/bin/$TOOL_NAME"; then
            chmod +x "$HOME/.local/bin/$TOOL_NAME"
            echo -e "${GREEN}✓ $TOOL_NAME installed to ~/.local/bin/$TOOL_NAME${NC}"
            
            # Check if ~/.local/bin is in PATH
            if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
                echo ""
                echo -e "${YELLOW}Add ~/.local/bin to your PATH:${NC}"
                echo '  export PATH="$HOME/.local/bin:$PATH"'
                echo ""
                # Add to PATH for current script execution
                export PATH="$HOME/.local/bin:$PATH"
            fi
            return 0
        else
            echo -e "${RED}Failed to download $TOOL_NAME${NC}"
            echo "Please install manually with: cargo install $5"
            return 1
        fi
    else
        echo "Please install $TOOL_NAME manually:"
        echo "  cargo install $5"
        echo "Or download from: https://github.com/$GITHUB_REPO/releases"
        return 1
    fi
}

# Check for required tools
echo ""
echo "Checking for required tools..."

MISSING_TOOLS=false

# Check and potentially install wkg
if ! command -v wkg &> /dev/null; then
    if ! install_tool "wkg" "bytecodealliance/wasm-pkg-tools" "v0.11.0" "wkg" "wkg"; then
        MISSING_TOOLS=true
    fi
else
    echo -e "${GREEN}✓ wkg${NC}"
fi

# Check and potentially install wac
if ! command -v wac &> /dev/null; then
    if ! install_tool "wac" "bytecodealliance/wac" "v0.8.0" "wac-cli" "wac-cli"; then
        MISSING_TOOLS=true
    fi
else
    echo -e "${GREEN}✓ wac${NC}"
fi

# Exit if tools are missing
if [ "$MISSING_TOOLS" = true ]; then
    echo ""
    echo -e "${RED}Required tools are missing. Please install them and run setup again.${NC}"
    exit 1
fi

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Setup complete!${NC}"
echo ""
echo "Next steps:"
echo "  1. Build the component:"
echo "     make build"
echo ""
echo "  2. Run the server:"
echo "     make serve"
echo ""

# Check for runtime (wasmtime or spin)
if ! command -v wasmtime &> /dev/null && ! command -v spin &> /dev/null; then
    echo -e "${YELLOW}Note: No compatible runtime detected for serving.${NC}"
    echo "  Install wasmtime with:"
    echo "    curl https://wasmtime.dev/install.sh -sSf | bash"
fi

echo -e "${GREEN}========================================${NC}"