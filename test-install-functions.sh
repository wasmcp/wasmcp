#!/usr/bin/env bash
# Test install.sh functions without actually installing

set -euo pipefail

# Source only the detection functions
eval "$(sed -n '/^detect_os()/,/^}/p' install.sh)"
eval "$(sed -n '/^detect_arch()/,/^}/p' install.sh)"
eval "$(sed -n '/^error()/,/^}/p' install.sh)"

# Define colors for output
readonly GREEN='\033[0;32m'
readonly RED='\033[0;31m'
readonly NC='\033[0m'

success() {
    echo -e "${GREEN}✓ $1${NC}"
}

fail() {
    echo -e "${RED}✗ $1${NC}"
    exit 1
}

# Test OS detection
echo "Testing OS detection..."
os=$(detect_os)
echo "  Detected: $os"

case "$(uname -s)" in
    Linux*)
        [[ "$os" == "unknown-linux-gnu" ]] && success "OS detection correct" || fail "Expected 'unknown-linux-gnu', got '$os'"
        ;;
    Darwin*)
        [[ "$os" == "apple-darwin" ]] && success "OS detection correct" || fail "Expected 'apple-darwin', got '$os'"
        ;;
    *)
        fail "Unknown OS: $(uname -s)"
        ;;
esac

# Test architecture detection
echo "Testing architecture detection..."
arch=$(detect_arch)
echo "  Detected: $arch"

case "$(uname -m)" in
    x86_64)
        [[ "$arch" == "x86_64" ]] && success "Arch detection correct" || fail "Expected 'x86_64', got '$arch'"
        ;;
    arm64|aarch64)
        [[ "$arch" == "aarch64" ]] && success "Arch detection correct" || fail "Expected 'aarch64', got '$arch'"
        ;;
    *)
        fail "Unknown architecture: $(uname -m)"
        ;;
esac

# Show target triple
target="$arch-$os"
echo ""
success "Target triple: $target"
echo ""
echo "This matches the release artifact pattern: wasmcp-$target.tar.gz"
