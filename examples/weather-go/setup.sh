#!/bin/bash
set -euo pipefail

echo "🚀 Setting up Go MCP Weather Example"
echo "=================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check for TinyGo
check_tinygo() {
    if command -v tinygo &> /dev/null; then
        TINYGO_VERSION=$(tinygo version | head -n1)
        echo -e "${GREEN}✓${NC} TinyGo installed: $TINYGO_VERSION"
        
        # Check version is >= 0.34.0
        VERSION=$(tinygo version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
        MAJOR=$(echo $VERSION | cut -d. -f1)
        MINOR=$(echo $VERSION | cut -d. -f2)
        
        if [ "$MAJOR" -eq 0 ] && [ "$MINOR" -lt 34 ]; then
            echo -e "${YELLOW}⚠${NC} TinyGo version $VERSION is older than recommended 0.34.0"
            echo "   Please update TinyGo for best compatibility"
        fi
    else
        echo -e "${RED}✗${NC} TinyGo not found"
        echo ""
        echo "Please install TinyGo:"
        echo "  macOS:    brew install tinygo"
        echo "  Linux:    wget https://github.com/tinygo-org/tinygo/releases/download/v0.34.0/tinygo_0.34.0_amd64.deb && sudo dpkg -i tinygo_0.34.0_amd64.deb"
        echo "  Windows:  scoop install tinygo"
        echo ""
        echo "Or visit: https://tinygo.org/getting-started/"
        return 1
    fi
}

# Check for wasm-tools
check_wasm_tools() {
    if command -v wasm-tools &> /dev/null; then
        WASM_TOOLS_VERSION=$(wasm-tools -V)
        echo -e "${GREEN}✓${NC} wasm-tools installed: $WASM_TOOLS_VERSION"
    else
        echo -e "${RED}✗${NC} wasm-tools not found"
        echo ""
        echo "Installing wasm-tools..."
        if command -v cargo &> /dev/null; then
            cargo install --locked wasm-tools@1.235.0
        else
            echo "Please install wasm-tools from: https://github.com/bytecodealliance/wasm-tools/releases"
            return 1
        fi
    fi
}

# Check for wkg
check_wkg() {
    if command -v wkg &> /dev/null; then
        echo -e "${GREEN}✓${NC} wkg installed"
    else
        echo -e "${RED}✗${NC} wkg not found"
        echo ""
        echo "Installing wkg..."
        if command -v cargo &> /dev/null; then
            cargo install --locked wkg
        else
            echo "Please install wkg from: https://github.com/bytecodealliance/wasm-pkg-tools/releases"
            return 1
        fi
    fi
}

# Check for wac
check_wac() {
    if command -v wac &> /dev/null; then
        echo -e "${GREEN}✓${NC} wac installed"
    else
        echo -e "${RED}✗${NC} wac not found"
        echo ""
        echo "Installing wac..."
        if command -v cargo &> /dev/null; then
            cargo install --locked wac-cli
        else
            echo "Please install wac from: https://github.com/bytecodealliance/wac/releases"
            return 1
        fi
    fi
}

# Check Go version (need 1.24+ for go tool support)
check_go() {
    if command -v go &> /dev/null; then
        GO_VERSION=$(go version | grep -oE '[0-9]+\.[0-9]+' | head -1)
        echo -e "${GREEN}✓${NC} Go installed: $(go version)"
        
        MAJOR=$(echo $GO_VERSION | cut -d. -f1)
        MINOR=$(echo $GO_VERSION | cut -d. -f2)
        
        if [ "$MAJOR" -eq 1 ] && [ "$MINOR" -lt 24 ]; then
            echo -e "${YELLOW}⚠${NC} Go $GO_VERSION is older than recommended 1.24"
            echo "   The 'go tool' directive requires Go 1.24+"
            echo "   You can still use wit-bindgen-go directly"
        fi
    else
        echo -e "${RED}✗${NC} Go not found"
        echo "Please install Go from: https://go.dev/dl/"
        return 1
    fi
}

# Check runtime (wasmtime or spin)
check_runtime() {
    local has_runtime=false
    
    if command -v wasmtime &> /dev/null; then
        echo -e "${GREEN}✓${NC} wasmtime installed: $(wasmtime --version)"
        has_runtime=true
    fi
    
    if command -v spin &> /dev/null; then
        echo -e "${GREEN}✓${NC} spin installed: $(spin --version)"
        has_runtime=true
    fi
    
    if [ "$has_runtime" = false ]; then
        echo -e "${YELLOW}⚠${NC} No WebAssembly runtime found"
        echo ""
        echo "Install at least one runtime:"
        echo "  wasmtime: curl https://wasmtime.dev/install.sh -sSf | bash"
        echo "  spin:     curl -fsSL https://developer.fermyon.com/downloads/install.sh | bash"
    fi
}

# Main setup flow
main() {
    echo "Checking dependencies..."
    echo ""
    
    local all_good=true
    
    check_go || all_good=false
    check_tinygo || all_good=false
    check_wasm_tools || all_good=false
    check_wkg || all_good=false
    check_wac || all_good=false
    check_runtime
    
    echo ""
    echo "=================================="
    
    if [ "$all_good" = true ]; then
        echo -e "${GREEN}✅ All required tools are installed!${NC}"
        echo ""
        
        # Install Go dependencies
        echo "Installing Go dependencies..."
        go mod download
        go mod tidy
        
        # Set up WIT dependencies
        echo "Setting up WIT dependencies..."
        if [ ! -f "wkg.lock" ]; then
            wkg wit init || true
        fi
        
        # Configure WIT namespace if needed
        if [ ! -f "wkg.toml" ]; then
            echo "Creating wkg.toml configuration..."
            cat > wkg.toml << 'EOF'
version = "0.1.0"

[dependencies]
"fastertools:mcp" = "0.1.10"
"wasi:cli" = "0.2.0"
"wasi:http" = "0.2.0"
EOF
        fi
        
        echo ""
        echo "🎉 Setup complete!"
        echo ""
        echo "Next steps:"
        echo "  1. Build the component:  make build"
        echo "  2. Run the server:       make run"
        echo "  3. Test the tools:       make test-tools"
    else
        echo -e "${RED}❌ Some dependencies are missing${NC}"
        echo "Please install the missing tools and run setup again"
        exit 1
    fi
}

main "$@"