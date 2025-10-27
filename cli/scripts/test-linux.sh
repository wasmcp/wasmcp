#!/bin/bash
# Test wasmcp CLI in Linux environment via Docker
#
# This script runs the same tests that GitHub Actions CI will run,
# allowing you to catch Linux-specific issues before pushing.
#
# Usage:
#   ./scripts/test-linux.sh           # Run all tests
#   ./scripts/test-linux.sh --fast    # Skip clippy/fmt checks
#   ./scripts/test-linux.sh --help    # Show help

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
RUST_VERSION="1.89"
DOCKER_IMAGE="rust:${RUST_VERSION}"

# Parse arguments
FAST_MODE=false
SHOW_HELP=false

for arg in "$@"; do
    case $arg in
        --fast)
            FAST_MODE=true
            shift
            ;;
        --help|-h)
            SHOW_HELP=true
            shift
            ;;
        *)
            echo -e "${RED}Unknown argument: $arg${NC}"
            SHOW_HELP=true
            shift
            ;;
    esac
done

if [ "$SHOW_HELP" = true ]; then
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Test wasmcp CLI in Linux environment via Docker"
    echo ""
    echo "Options:"
    echo "  --fast    Skip clippy and rustfmt checks (faster)"
    echo "  --help    Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0              # Run all tests (recommended before pushing)"
    echo "  $0 --fast       # Quick test run (build + tests only)"
    exit 0
fi

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}❌ Error: Docker is not running${NC}"
    echo "Please start Docker Desktop and try again"
    exit 1
fi

# Print header
echo -e "${BLUE}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║${NC}  Testing wasmcp CLI in Linux Environment (via Docker)  ${BLUE}║${NC}"
echo -e "${BLUE}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}Docker Image:${NC} $DOCKER_IMAGE"
echo -e "${YELLOW}Working Dir:${NC}  $(pwd)"
echo -e "${YELLOW}Fast Mode:${NC}    $FAST_MODE"
echo ""

# Create test script that runs inside container
TEST_SCRIPT="/tmp/wasmcp-test-$$.sh"
cat > "$TEST_SCRIPT" << 'SCRIPT_EOF'
#!/bin/bash
set -e

if [ "$SKIP_LINT" != "true" ]; then
    echo "==> Installing Rust tooling..."
    rustup component add rustfmt clippy
    echo "✓ Tooling installed"
    echo ""

    echo "==> Checking code formatting..."
    cargo fmt --check
    echo "✓ Format check passed"
    echo ""

    echo "==> Running clippy (linter)..."
    cargo clippy --all-targets -- -D warnings
    echo "✓ Clippy passed"
    echo ""
fi

echo "==> Building for Linux (x86_64)..."
cargo build --release
echo "✓ Build successful"
echo ""

echo "==> Running test suite..."
cargo test --lib
echo "✓ All tests passed"
SCRIPT_EOF

chmod +x "$TEST_SCRIPT"

# Run tests in Docker
echo -e "${YELLOW}Running tests in Linux container...${NC}"
echo ""

if [ "$FAST_MODE" = true ]; then
    docker run --rm \
        --platform linux/amd64 \
        -v "$(pwd)":/workspace \
        -v "$TEST_SCRIPT":/test-script.sh \
        -w /workspace \
        -e SKIP_LINT=true \
        "$DOCKER_IMAGE" \
        /test-script.sh
else
    docker run --rm \
        --platform linux/amd64 \
        -v "$(pwd)":/workspace \
        -v "$TEST_SCRIPT":/test-script.sh \
        -w /workspace \
        "$DOCKER_IMAGE" \
        /test-script.sh
fi

# Clean up
rm -f "$TEST_SCRIPT"

# Print success message
echo ""
echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║${NC}  ✅ All Linux tests passed! Safe to push to GitHub   ${GREEN}║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "  git add ."
echo "  git commit -m \"Your commit message\""
echo "  git push"
echo ""
