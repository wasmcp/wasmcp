#!/bin/bash
set -euo pipefail

# Safe version bump script for wasmcp
# Usage: ./scripts/bump-version.sh <new-version> [--dry-run]

NEW_VERSION="${1:-}"
DRY_RUN=false

if [[ "${2:-}" == "--dry-run" ]]; then
    DRY_RUN=true
fi

if [[ -z "$NEW_VERSION" ]]; then
    echo "Usage: $0 <new-version> [--dry-run]"
    echo "Example: $0 0.3.0"
    echo "Example: $0 0.3.0-alpha.48 --dry-run"
    exit 1
fi

# Validate version format (semver or semver-prerelease)
if ! echo "$NEW_VERSION" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.-]+)?$'; then
    echo "Error: Invalid version format: $NEW_VERSION"
    echo "Expected: X.Y.Z or X.Y.Z-prerelease"
    exit 1
fi

# Get current version from WIT package
CURRENT_VERSION=$(grep -oP 'package wasmcp:mcp@\K[^;]+' wit/world.wit)

echo "🔍 Current version: $CURRENT_VERSION"
echo "🎯 New version: $NEW_VERSION"

if [[ "$DRY_RUN" == true ]]; then
    echo "🔬 DRY RUN MODE - No files will be modified"
fi

echo ""
echo "Files to update:"

# List all files that will be updated
echo "  - wit/world.wit"
echo "  - cli/Cargo.toml"
echo "  - cli/src/compose.rs"
echo "  - cli/src/scaffold.rs"
find crates -name Cargo.toml | while read -r file; do
    if grep -q "wasmcp:mcp" "$file"; then
        echo "  - $file"
    fi
done

echo ""

if [[ "$DRY_RUN" == false ]]; then
    read -p "Proceed with version bump? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 0
    fi
fi

update_file() {
    local file=$1
    local pattern=$2
    local replacement=$3

    if [[ "$DRY_RUN" == true ]]; then
        echo "  [dry-run] Would update: $file"
    else
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' "$pattern" "$file"
        else
            sed -i "$pattern" "$file"
        fi
        echo "  ✅ Updated: $file"
    fi
}

echo ""
echo "Updating files..."

# Update WIT package version
update_file "wit/world.wit" \
    "s/package wasmcp:mcp@[^;]\\+;/package wasmcp:mcp@$NEW_VERSION;/" \
    "package wasmcp:mcp@$NEW_VERSION;"

# Update CLI version
update_file "cli/Cargo.toml" \
    "s/^version = \".*\"/version = \"$NEW_VERSION\"/" \
    "version = \"$NEW_VERSION\""

# Update CLI default version in compose.rs
update_file "cli/src/compose.rs" \
    "s/default_value = \"[^\"]*\"/default_value = \"$NEW_VERSION\"/" \
    "default_value = \"$NEW_VERSION\""

# Update CLI default version in scaffold.rs
update_file "cli/src/scaffold.rs" \
    "s/default_value = \"[^\"]*\"/default_value = \"$NEW_VERSION\"/" \
    "default_value = \"$NEW_VERSION\""

# Update all component Cargo.toml files
find crates -name Cargo.toml | while read -r file; do
    if grep -q "wasmcp:mcp" "$file"; then
        # Update target lines with version
        update_file "$file" \
            "s|@[0-9]\\+\\.[0-9]\\+\\.[0-9]\\+\\(-[a-zA-Z0-9.-]\\+\\)\\?\"|@$NEW_VERSION\"|g" \
            ""
    fi
done

echo ""
if [[ "$DRY_RUN" == true ]]; then
    echo "🔬 Dry run complete. No files were modified."
    echo "Run without --dry-run to apply changes."
else
    echo "✅ Version bumped from $CURRENT_VERSION to $NEW_VERSION"
    echo ""
    echo "Next steps:"
    echo "  1. Review changes: git diff"
    echo "  2. Test build: cd crates && make"
    echo "  3. Test CLI: cd cli && cargo test"
    echo "  4. Commit: git add -A && git commit -m 'chore: bump version to $NEW_VERSION'"
fi
