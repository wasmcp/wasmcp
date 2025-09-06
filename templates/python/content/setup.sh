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
                BINARY_URL="https://github.com/$GITHUB_REPO/releases/download/$VERSION/wac-cli-${ARCH_NAME}-unknown-linux-musl"
            elif [ "$OS_NAME" = "macos" ]; then
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

# Configure wkg for wasmcp namespace if needed
echo ""
echo "Checking wkg configuration..."
WKG_CONFIG="$HOME/.config/wasm-pkg/config.toml"

# Create config directory if it doesn't exist
mkdir -p "$(dirname "$WKG_CONFIG")"

# Check if config file exists, create if not
if [ ! -f "$WKG_CONFIG" ]; then
    echo -e "${YELLOW}No wkg config found. Creating new config file...${NC}"
    cat > "$WKG_CONFIG" << 'EOF'
[namespace_registries]
wasmcp = "ghcr.io"
EOF
    echo -e "${GREEN}✓ Created wkg config with wasmcp namespace${NC}"
else
    # Check if wasmcp is already configured
    if ! grep -q '^wasmcp\s*=' "$WKG_CONFIG" 2>/dev/null; then
        echo -e "${YELLOW}The 'wasmcp' namespace is not configured in wkg.${NC}"
        echo ""
        echo "Would you like to add it automatically? [Y/n]"
        read -r RESPONSE
        RESPONSE=${RESPONSE:-Y}
        
        if [[ "$RESPONSE" =~ ^[Yy]$ ]]; then
            # Check if [namespace_registries] section exists
            if ! grep -q '^\[namespace_registries\]' "$WKG_CONFIG"; then
                # Add the entire section
                echo "" >> "$WKG_CONFIG"
                echo "[namespace_registries]" >> "$WKG_CONFIG"
                echo 'wasmcp = "ghcr.io"' >> "$WKG_CONFIG"
            else
                # Add just the wasmcp line after [namespace_registries]
                sed -i '/^\[namespace_registries\]/a wasmcp = "ghcr.io"' "$WKG_CONFIG"
            fi
            echo -e "${GREEN}✓ Added wasmcp namespace to wkg config${NC}"
        else
            echo "Please configure the wasmcp namespace manually by editing:"
            echo "  $WKG_CONFIG"
            echo ""
            echo "Add this line under [namespace_registries]:"
            echo '  wasmcp = "ghcr.io"'
            exit 1
        fi
    else
        echo -e "${GREEN}✓ wasmcp namespace already configured${NC}"
    fi
fi

# Fetch WIT dependencies
echo ""
echo "Fetching WIT dependencies..."
wkg wit fetch
echo -e "${GREEN}✓ WIT dependencies fetched${NC}"

# Generate bindings
echo ""
echo "Generating Python bindings..."
componentize-py --wit-path wit --world {{project-name | kebab_case}} bindings . 2>/dev/null || true
echo -e "${GREEN}✓ Bindings generated${NC}"

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Setup complete!${NC}"
echo ""
echo "Next steps:"
echo "  1. Activate the virtual environment (optional, for Python development):"
echo "     source venv/bin/activate"
echo ""
echo "  2. Build the component:"
echo "     make build"
echo ""
echo "  3. Run the server:"
echo "     make serve"
echo ""

# Check for runtime (wasmtime or spin)
if ! command -v wasmtime &> /dev/null && ! command -v spin &> /dev/null; then
    echo -e "${YELLOW}Note: No compatible runtime detected for serving.${NC}"
    echo "  Install wasmtime with:"
    echo "    curl https://wasmtime.dev/install.sh -sSf | bash"
fi

echo -e "${GREEN}========================================${NC}"