#!/bin/bash
set -euo pipefail

# Verify all version files are synchronized
# Usage: ./scripts/verify-versions.sh [expected-version]
#
# If expected-version is provided, verifies all files match that version.
# If not provided, checks that all version files are consistent with each other.

EXPECTED_VERSION="${1:-}"
ERRORS=0

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_error() {
    echo -e "${RED}❌ $1${NC}"
    ERRORS=$((ERRORS + 1))
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Extract versions from files
WIT_VERSION=$(grep -oP 'package wasmcp:mcp@\K[^;]+' wit/world.wit)
CLI_VERSION=$(grep -oP '^version = "\K[^"]+' cli/Cargo.toml)
MAIN_VERSION=$(grep -oP 'default_value = "\K[^"]+' cli/src/main.rs | head -1)
SCAFFOLD_VERSION=$(grep -oP 'v\K[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?' cli/src/scaffold.rs | head -1)

echo "Version Check Report"
echo "===================="
echo ""
echo "Found versions:"
echo "  WIT package (wit/world.wit):              $WIT_VERSION"
echo "  CLI (cli/Cargo.toml):                     $CLI_VERSION"
echo "  CLI main default (cli/src/main.rs):       $MAIN_VERSION"
echo "  CLI scaffold template (cli/src/scaffold.rs): $SCAFFOLD_VERSION"
echo ""

# Check component Cargo.toml files
echo "Component versions:"
COMPONENT_VERSIONS=()
for cargo_file in crates/*/Cargo.toml; do
    if grep -q "wasmcp:mcp" "$cargo_file"; then
        component_version=$(grep -oP '@\K[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?' "$cargo_file" | head -1)
        component_name=$(basename $(dirname "$cargo_file"))
        echo "  $component_name: $component_version"
        COMPONENT_VERSIONS+=("$component_version")
    fi
done
echo ""

# Check Makefile version
MAKEFILE_VERSION=$(grep -oP 'wasmcp:[a-z-]+@\K[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?' crates/Makefile | head -1)
echo "  Makefile (crates/Makefile):            $MAKEFILE_VERSION"
echo ""

# If expected version provided, check all match it
if [ -n "$EXPECTED_VERSION" ]; then
    echo "Verifying all versions match: $EXPECTED_VERSION"
    echo "================================================"
    echo ""

    [ "$WIT_VERSION" = "$EXPECTED_VERSION" ] && log_success "WIT version matches" || log_error "WIT version is $WIT_VERSION, expected $EXPECTED_VERSION"
    [ "$CLI_VERSION" = "$EXPECTED_VERSION" ] && log_success "CLI version matches" || log_error "CLI version is $CLI_VERSION, expected $EXPECTED_VERSION"
    [ "$MAIN_VERSION" = "$EXPECTED_VERSION" ] && log_success "Main default version matches" || log_error "Main default is $MAIN_VERSION, expected $EXPECTED_VERSION"
    [ "$SCAFFOLD_VERSION" = "$EXPECTED_VERSION" ] && log_success "Scaffold template version matches" || log_error "Scaffold template is $SCAFFOLD_VERSION, expected $EXPECTED_VERSION"
    [ "$MAKEFILE_VERSION" = "$EXPECTED_VERSION" ] && log_success "Makefile version matches" || log_error "Makefile version is $MAKEFILE_VERSION, expected $EXPECTED_VERSION"

    for component_version in "${COMPONENT_VERSIONS[@]}"; do
        if [ "$component_version" != "$EXPECTED_VERSION" ]; then
            log_error "Component version is $component_version, expected $EXPECTED_VERSION"
        fi
    done

    # Check all component versions are the same
    UNIQUE_COMPONENT_VERSIONS=$(printf '%s\n' "${COMPONENT_VERSIONS[@]}" | sort -u | wc -l)
    if [ "$UNIQUE_COMPONENT_VERSIONS" -eq 1 ]; then
        log_success "All component versions consistent"
    else
        log_error "Component versions are inconsistent"
    fi

else
    # Check all versions are consistent with each other
    echo "Verifying version consistency"
    echo "============================="
    echo ""

    REFERENCE_VERSION="$WIT_VERSION"
    echo "Using WIT version as reference: $REFERENCE_VERSION"
    echo ""

    [ "$CLI_VERSION" = "$REFERENCE_VERSION" ] && log_success "CLI version matches WIT" || log_warning "CLI version ($CLI_VERSION) differs from WIT ($REFERENCE_VERSION)"
    [ "$MAIN_VERSION" = "$REFERENCE_VERSION" ] && log_success "Main default matches WIT" || log_warning "Main default ($MAIN_VERSION) differs from WIT ($REFERENCE_VERSION)"
    [ "$SCAFFOLD_VERSION" = "$REFERENCE_VERSION" ] && log_success "Scaffold template matches WIT" || log_warning "Scaffold template ($SCAFFOLD_VERSION) differs from WIT ($REFERENCE_VERSION)"
    [ "$MAKEFILE_VERSION" = "$REFERENCE_VERSION" ] && log_success "Makefile version matches WIT" || log_warning "Makefile version ($MAKEFILE_VERSION) differs from WIT ($REFERENCE_VERSION)"

    # Check all component versions match reference
    MISMATCHED=0
    for component_version in "${COMPONENT_VERSIONS[@]}"; do
        if [ "$component_version" != "$REFERENCE_VERSION" ]; then
            MISMATCHED=$((MISMATCHED + 1))
        fi
    done

    if [ "$MISMATCHED" -eq 0 ]; then
        log_success "All component versions match WIT"
    else
        log_warning "$MISMATCHED component(s) have different versions from WIT"
    fi

    # Check all component versions are at least consistent with each other
    UNIQUE_COMPONENT_VERSIONS=$(printf '%s\n' "${COMPONENT_VERSIONS[@]}" | sort -u | wc -l)
    if [ "$UNIQUE_COMPONENT_VERSIONS" -eq 1 ]; then
        log_success "All component versions are consistent with each other"
    else
        log_error "Component versions are inconsistent: found $UNIQUE_COMPONENT_VERSIONS different versions"
    fi
fi

echo ""
echo "Summary"
echo "======="
if [ "$ERRORS" -eq 0 ]; then
    log_success "All version checks passed!"
    exit 0
else
    log_error "Found $ERRORS error(s)"
    echo ""
    echo "To fix version mismatches, run:"
    echo "  ./scripts/bump-version.sh <desired-version>"
    exit 1
fi
