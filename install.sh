#!/usr/bin/env bash
# install.sh - wasmcp CLI installer
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/wasmcp/wasmcp/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/wasmcp/wasmcp/main/install.sh | bash -s -- --version 0.4.4
#
# Environment variables:
#   WASMCP_HOME - Installation directory (default: $HOME/.wasmcp)
#   WASMCP_VERSION - Version to install (default: latest)
#
# Options:
#   --version <version>  Install specific version (e.g., 0.4.4)
#   --help               Show this help message

set -euo pipefail

# Configuration
readonly GITHUB_REPO="wasmcp/wasmcp"
readonly WASMCP_HOME="${WASMCP_HOME:-$HOME/.wasmcp}"
readonly WASMCP_BIN="$WASMCP_HOME/bin"

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[0;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Track if we're in a terminal for color output
if [[ -t 1 ]]; then
    readonly USE_COLOR=true
else
    readonly USE_COLOR=false
fi

#######################################
# Print colored message
# Arguments:
#   $1 - Color code
#   $2 - Message
#######################################
print_color() {
    local color="$1"
    local message="$2"
    if [[ "$USE_COLOR" == "true" ]]; then
        echo -e "${color}${message}${NC}"
    else
        echo "$message"
    fi
}

#######################################
# Print error message and exit
# Arguments:
#   $1 - Error message
#######################################
error() {
    print_color "$RED" "ERROR: $1" >&2
    exit 1
}

#######################################
# Print warning message
# Arguments:
#   $1 - Warning message
#######################################
warn() {
    print_color "$YELLOW" "WARNING: $1" >&2
}

#######################################
# Print info message
# Arguments:
#   $1 - Info message
#######################################
info() {
    print_color "$BLUE" "$1"
}

#######################################
# Print success message
# Arguments:
#   $1 - Success message
#######################################
success() {
    print_color "$GREEN" "$1"
}

#######################################
# Show help message
#######################################
show_help() {
    cat << EOF
wasmcp installer

Usage:
  curl -fsSL https://raw.githubusercontent.com/wasmcp/wasmcp/main/install.sh | bash
  curl -fsSL https://raw.githubusercontent.com/wasmcp/wasmcp/main/install.sh | bash -s -- --version 0.4.4

Options:
  --version <version>  Install specific version (e.g., 0.4.4)
  --help               Show this help message

Environment variables:
  WASMCP_HOME          Installation directory (default: \$HOME/.wasmcp)
  WASMCP_VERSION       Version to install (default: latest)

Examples:
  # Install latest version
  curl -fsSL https://raw.githubusercontent.com/wasmcp/wasmcp/main/install.sh | bash

  # Install specific version
  curl -fsSL https://raw.githubusercontent.com/wasmcp/wasmcp/main/install.sh | bash -s -- --version 0.4.4

  # Custom install location
  WASMCP_HOME=\$HOME/tools curl -fsSL https://raw.githubusercontent.com/wasmcp/wasmcp/main/install.sh | bash
EOF
}

#######################################
# Detect operating system
# Returns:
#   OS identifier for release artifacts
#######################################
detect_os() {
    local os
    os="$(uname -s)"

    case "$os" in
        Linux*)
            echo "unknown-linux-gnu"
            ;;
        Darwin*)
            echo "apple-darwin"
            ;;
        *)
            error "Unsupported operating system: $os"
            ;;
    esac
}

#######################################
# Detect CPU architecture
# Returns:
#   Architecture identifier for release artifacts
#######################################
detect_arch() {
    local arch
    arch="$(uname -m)"

    case "$arch" in
        x86_64)
            echo "x86_64"
            ;;
        arm64)
            # macOS uses arm64, but releases use aarch64
            echo "aarch64"
            ;;
        aarch64)
            echo "aarch64"
            ;;
        *)
            error "Unsupported architecture: $arch"
            ;;
    esac
}

#######################################
# Check if required commands exist
# Arguments:
#   $@ - Commands to check
#######################################
check_commands() {
    local missing=()
    for cmd in "$@"; do
        if ! command -v "$cmd" &> /dev/null; then
            missing+=("$cmd")
        fi
    done

    if [[ ${#missing[@]} -gt 0 ]]; then
        error "Required commands not found: ${missing[*]}"
    fi
}

#######################################
# Get latest release version from GitHub
# Returns:
#   Version string (e.g., "0.4.4")
#######################################
get_latest_version() {
    local version

    info "Fetching latest version..." >&2

    # Use GitHub API to get all releases and filter for CLI releases
    # This is necessary because the repo contains multiple packages with different tag prefixes
    version=$(curl -fsSL "https://api.github.com/repos/$GITHUB_REPO/releases?per_page=100" \
        | grep '"tag_name"' \
        | grep '"cli-v' \
        | sed -E 's/.*"cli-v([^"]+)".*/\1/' \
        | head -1)

    if [[ -z "$version" ]]; then
        error "Failed to fetch latest CLI version from GitHub API"
    fi

    # Validate version format
    if ! [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$ ]]; then
        error "Invalid version format: $version"
    fi

    echo "$version"
}

#######################################
# Download file with checksum verification
# Arguments:
#   $1 - URL to download
#   $2 - Output path
#   $3 - Expected SHA256 checksum (optional)
#######################################
download_file() {
    local url="$1"
    local output="$2"
    local expected_checksum="${3:-}"

    info "Downloading: $url"

    if ! curl -fsSL "$url" -o "$output"; then
        error "Failed to download: $url"
    fi

    if [[ -n "$expected_checksum" ]]; then
        local actual_checksum
        actual_checksum=$(sha256sum "$output" | cut -d' ' -f1)

        if [[ "$actual_checksum" != "$expected_checksum" ]]; then
            error "Checksum verification failed!\nExpected: $expected_checksum\nActual:   $actual_checksum"
        fi

        success "✓ Checksum verified"
    fi
}

#######################################
# Download and install wasmcp binary
# Arguments:
#   $1 - Version to install
#   $2 - Target triple (e.g., x86_64-unknown-linux-gnu)
#######################################
download_and_install() {
    local version="$1"
    local target="$2"
    local temp_dir

    temp_dir=$(mktemp -d)

    local base_url="https://github.com/$GITHUB_REPO/releases/download/cli-v$version"
    local artifact_name="wasmcp-$target"
    local tarball="$artifact_name.tar.gz"
    local checksum_file="$artifact_name.sha256"

    info "Installing wasmcp v$version for $target..."

    # Download checksum file
    download_file \
        "$base_url/$checksum_file" \
        "$temp_dir/$checksum_file"

    # Extract expected checksum
    local expected_checksum
    expected_checksum=$(cut -d' ' -f1 "$temp_dir/$checksum_file")

    # Download tarball with checksum verification
    download_file \
        "$base_url/$tarball" \
        "$temp_dir/$tarball" \
        "$expected_checksum"

    # Extract tarball
    info "Extracting binary..."
    tar -xzf "$temp_dir/$tarball" -C "$temp_dir"

    # Verify binary exists
    if [[ ! -f "$temp_dir/wasmcp" ]]; then
        rm -rf "$temp_dir"
        error "Binary not found in tarball"
    fi

    # Create installation directory
    mkdir -p "$WASMCP_BIN"

    # Install binary
    info "Installing to $WASMCP_BIN/wasmcp..."
    mv "$temp_dir/wasmcp" "$WASMCP_BIN/wasmcp"
    chmod +x "$WASMCP_BIN/wasmcp"

    # Clean up temp directory
    rm -rf "$temp_dir"

    success "✓ Binary installed"
}

#######################################
# Detect shell configuration file
# Returns:
#   Path to shell config file
#######################################
detect_shell_profile() {
    # Detect shell type
    local shell_name
    shell_name=$(basename "${SHELL:-sh}")

    case "$shell_name" in
        bash)
            # macOS uses .bash_profile, Linux uses .bashrc
            if [[ "$OSTYPE" == "darwin"* ]] && [[ -f "$HOME/.bash_profile" ]]; then
                echo "$HOME/.bash_profile"
            elif [[ -f "$HOME/.bashrc" ]]; then
                echo "$HOME/.bashrc"
            else
                echo "$HOME/.profile"
            fi
            ;;
        zsh)
            echo "$HOME/.zshrc"
            ;;
        fish)
            echo "$HOME/.config/fish/config.fish"
            ;;
        *)
            # Fallback to .profile
            echo "$HOME/.profile"
            ;;
    esac
}

#######################################
# Generate shell configuration snippet
# Arguments:
#   $1 - Shell name
# Returns:
#   Configuration snippet for the shell
#######################################
generate_shell_config() {
    local shell_name="$1"

    case "$shell_name" in
        fish)
            cat << EOF

# wasmcp
set -gx WASMCP_HOME "$WASMCP_HOME"
fish_add_path "$WASMCP_BIN"
EOF
            ;;
        *)
            cat << EOF

# wasmcp
export WASMCP_HOME="$WASMCP_HOME"
export PATH="\$WASMCP_HOME/bin:\$PATH"
EOF
            ;;
    esac
}

#######################################
# Configure PATH in shell profile
#######################################
configure_path() {
    local profile
    profile=$(detect_shell_profile)

    # Check if already configured
    if [[ -f "$profile" ]] && grep -q "WASMCP_HOME" "$profile"; then
        info "PATH already configured in $profile"
        return 0
    fi

    # Detect shell type for config generation
    local shell_name
    shell_name=$(basename "${SHELL:-sh}")

    # Create profile directory if it doesn't exist (for fish)
    if [[ "$shell_name" == "fish" ]]; then
        mkdir -p "$(dirname "$profile")"
    fi

    info "Configuring PATH in $profile..."

    # Append configuration to profile
    generate_shell_config "$shell_name" >> "$profile"

    success "✓ PATH configured"
}

#######################################
# Show post-installation instructions
# Arguments:
#   $1 - Installed version
#######################################
show_completion_message() {
    local version="$1"
    local profile
    profile=$(detect_shell_profile)

    echo ""
    success "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    success "  wasmcp v$version installed successfully!"
    success "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""
    info "Installation location: $WASMCP_BIN/wasmcp"
    info "Configuration file:    $profile"
    echo ""
    info "To use wasmcp in your current shell, run:"
    echo ""
    print_color "$GREEN" "    source $profile"
    echo ""
    info "Or simply restart your terminal."
    echo ""
    info "Quick start:"
    echo ""
    echo "    wasmcp --version"
    echo "    wasmcp new my-tools --language rust"
    echo ""
    info "Documentation: https://github.com/$GITHUB_REPO"
    echo ""
}

#######################################
# Parse command line arguments
# Arguments:
#   $@ - Command line arguments
# Sets:
#   VERSION - Version to install
#######################################
parse_args() {
    VERSION="${WASMCP_VERSION:-latest}"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --version)
                if [[ -z "${2:-}" ]]; then
                    error "--version requires a version argument"
                fi
                VERSION="$2"
                shift 2
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                error "Unknown option: $1\nUse --help for usage information"
                ;;
        esac
    done
}

#######################################
# Main installation function
#######################################
main() {
    # Parse arguments
    parse_args "$@"

    # Check required commands
    check_commands curl tar sha256sum grep sed

    # Detect system
    local arch os target
    arch=$(detect_arch)
    os=$(detect_os)
    target="$arch-$os"

    info "Detected: $target"

    # Resolve version
    if [[ "$VERSION" == "latest" ]]; then
        VERSION=$(get_latest_version)
    else
        # Validate version format
        if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$ ]]; then
            error "Invalid version format: $VERSION"
        fi
    fi

    info "Target version: v$VERSION"
    echo ""

    # Download and install
    download_and_install "$VERSION" "$target"

    # Configure PATH
    configure_path

    # Verify installation
    if "$WASMCP_BIN/wasmcp" --version &> /dev/null; then
        success "✓ Installation verified"
    else
        warn "Installation completed but 'wasmcp --version' failed"
        warn "You may need to restart your shell or source your profile"
    fi

    # Show completion message
    show_completion_message "$VERSION"
}

# Run main function with all arguments
main "$@"
