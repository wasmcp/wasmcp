# Version Management Scripts

This directory contains scripts for managing versions across the ftl-components repository.

## Scripts

### sync-versions.sh
Synchronizes versions from `versions.toml` to all templates and documentation.

```bash
./scripts/sync-versions.sh
```

This script:
- Updates SDK versions in template dependencies
- Updates gateway component references in spin.toml files
- Ensures all versions are consistent across the repo

### bump-version.sh
Bumps the version for a specific component and syncs all references.

```bash
./scripts/bump-version.sh <component> <new-version>
```

Examples:
```bash
# Bump Rust SDK to 0.3.0
./scripts/bump-version.sh ftl-sdk-rust 0.3.0

# Bump HTTP component to 0.2.0
./scripts/bump-version.sh mcp-http-component 0.2.0

# Bump TypeScript SDK to 0.2.0
./scripts/bump-version.sh ftl-sdk-typescript 0.2.0

# Bump WIT interface version (only on breaking changes!)
./scripts/bump-version.sh wit 0.2.0
```

## Version Management Strategy

1. **versions.toml** is the single source of truth
2. Run `sync-versions.sh` to propagate versions everywhere
3. CI validates that versions are in sync
4. Use `bump-version.sh` to update versions before releases

## Release Process

1. Bump version: `./scripts/bump-version.sh ftl-sdk-rust 0.3.0`
2. Commit changes: `git commit -am "chore: bump ftl-sdk-rust to 0.3.0"`
3. Create tag: `git tag ftl-sdk-rust-v0.3.0`
4. Push: `git push origin main ftl-sdk-rust-v0.3.0`
5. GitHub Actions will handle the release

## Adding New Components

To add version management for a new component:

1. Add it to `versions.toml`
2. Update `sync-versions.sh` to handle the new component
3. Update `bump-version.sh` to support version bumping
4. Add any necessary CI steps