# Release Guide

This document describes the release process for wasmcp, which consists of three **completely independent** release workflows:

1. **WIT Package** - Interface definitions (`wasmcp:mcp@version`)
2. **Components** - 8 framework WebAssembly components
3. **CLI** - Native binary tool for scaffolding and composition

## Release Architecture

Each workflow creates its own GitHub release with its own tag:

```
┌─────────────────┐
│  WIT Package    │  → Tag: wit-v{version}
│  (Interface)    │  → Release: "WIT v{version}"
└────────┬────────┘
         │ publishes to ghcr.io/wasmcp/mcp
         │
         │ Components workflow requires this to exist
         │
         ▼
┌─────────────────┐
│  Components     │  → Tag: components-v{version}
│  (8 wasm files) │  → Release: "Components v{version}"
└─────────────────┘

┌──────────────┐
│     CLI      │  → Tag: cli-v{version}
│  (4 platforms)│  → Release: "CLI v{version}"
└──────────────┘
```

### Dependencies

- **Components** workflow requires WIT package to be published first
- **CLI** workflow is completely independent
- Each workflow creates its own separate GitHub release

## Workflows

### 1. Release WIT Package (`.github/workflows/release-wit.yml`)

**Triggers:** Manual
**Publishes:** `wasmcp:mcp@{version}` to `ghcr.io/wasmcp/mcp`
**Creates:** Tag `wit-v{version}` + GitHub release "WIT v{version}"

**When to run:**
- Breaking changes to WIT interfaces
- New capabilities added to interfaces
- Before releasing components with interface changes

### 2. Release Components (`.github/workflows/release-components.yml`)

**Triggers:** Manual
**Requires:** WIT package must exist at specified version
**Publishes:** 8 components to `ghcr.io/wasmcp/*`
**Creates:** Tag `components-v{version}` + GitHub release "Components v{version}"

**What it builds:**
- `wasmcp:request@{version}`
- `wasmcp:error-writer@{version}`
- `wasmcp:initialize-writer@{version}`
- `wasmcp:tools-writer@{version}`
- `wasmcp:resources-writer@{version}`
- `wasmcp:http-transport@{version}`
- `wasmcp:stdio-transport@{version}`
- `wasmcp:initialize-handler@{version}`

### 3. Release CLI (`.github/workflows/release-cli.yml`)

**Triggers:** Manual (completely independent)
**Creates:** Tag `cli-v{version}` + GitHub release "CLI v{version}"

**Platforms:**
- Linux x86_64
- Linux ARM64
- macOS x86_64
- macOS Apple Silicon (ARM64)

## Release Scenarios

### Full Release (WIT + Components + CLI)

Use this for major/minor versions or when all artifacts need updating.

```bash
# 1. Bump all versions
./scripts/bump-version.sh 0.4.0 --validate

# 2. Review changes
git diff

# 3. Commit version bump
git add -A
git commit -m "chore: bump version to 0.4.0"
git push

# 4. Release WIT package (FIRST)
# GitHub UI: Actions → Release WIT Package
# Input: version = 0.4.0
# Creates: Tag wit-v0.4.0 + Release "WIT v0.4.0"
# Wait for completion (~2-3 minutes)

# 5. Release Components and CLI (can run in parallel, independent)
# GitHub UI: Actions → Release Components
# Input: version = 0.4.0, wit_version = 0.4.0 (optional)
# Creates: Tag components-v0.4.0 + Release "Components v0.4.0"
#
# GitHub UI: Actions → Release CLI
# Input: version = 0.4.0
# Creates: Tag cli-v0.4.0 + Release "CLI v0.4.0"
#
# These create SEPARATE releases, not one combined release
# Combined time: ~12-15 minutes

# 6. Post-release tasks
# - Make packages public at https://github.com/orgs/wasmcp/packages
# - Update release notes for each release:
#   - https://github.com/wasmcp/wasmcp/releases/tag/wit-v0.4.0
#   - https://github.com/wasmcp/wasmcp/releases/tag/components-v0.4.0
#   - https://github.com/wasmcp/wasmcp/releases/tag/cli-v0.4.0
```

### CLI-Only Hotfix

Use this for CLI bug fixes that don't require component or WIT changes.

```bash
# 1. Fix the bug in cli/
git commit -m "fix: CLI scaffold template generation"

# 2. Bump version (or use same version)
./scripts/bump-version.sh 0.3.0-alpha.61
git add cli/Cargo.toml cli/src/main.rs
git commit -m "chore: bump CLI to 0.3.0-alpha.61"
git push

# 3. Release CLI only
# GitHub UI: Actions → Release CLI
# Input: version = 0.3.0-alpha.61
# Creates: Tag cli-v0.3.0-alpha.61 + Release "CLI v0.3.0-alpha.61"
# Time: ~8-10 minutes
```

### Component-Only Update

Use this for component implementation fixes without interface changes.

```bash
# 1. Fix component logic
git commit crates/tools-writer/src/lib.rs -m "fix: tool response JSON formatting"

# 2. Bump version (or use same version)
./scripts/bump-version.sh 0.3.0-alpha.61
git add -A && git commit -m "chore: bump version to 0.3.0-alpha.61"
git push

# 3. Release components only
# GitHub UI: Actions → Release Components
# Input: version = 0.3.0-alpha.61, wit_version = 0.3.0-alpha.60
# Creates: Tag components-v0.3.0-alpha.61 + Release "Components v0.3.0-alpha.61"
# Time: ~10-12 minutes
#
# Note: wit_version can reference older WIT if no interface changes
```

### Pre-release (Alpha/Beta)

```bash
# Follow same process as Full Release, but with pre-release version
./scripts/bump-version.sh 0.4.0-alpha.1 --validate

# GitHub release will automatically be marked as "pre-release"
# due to version containing a hyphen
```

## Version Management

### Checking Version Consistency

```bash
# Check all version files are in sync
./scripts/verify-versions.sh

# Check against specific version
./scripts/verify-versions.sh 0.3.0-alpha.60
```

### Version Locations

The following files contain version numbers and must stay synchronized:

1. **WIT Package** - `wit/world.wit`
   ```wit
   package wasmcp:mcp@0.3.0-alpha.60;
   ```

2. **CLI** - `cli/Cargo.toml`
   ```toml
   version = "0.3.0-alpha.60"
   ```

3. **CLI Defaults** - `cli/src/compose.rs` and `cli/src/scaffold.rs`
   ```rust
   default_value = "0.3.0-alpha.60"
   ```

4. **Components** - `crates/*/Cargo.toml` (8 files)
   ```toml
   target = "wasmcp:mcp/error-writer@0.3.0-alpha.60"
   ```

5. **Makefile** - `crates/Makefile`
   ```makefile
   wkg publish --package 'wasmcp:request@0.3.0-alpha.60' ...
   ```

### Automated Version Bumping

```bash
# Bump all versions
./scripts/bump-version.sh 0.4.0

# Dry run (preview changes)
./scripts/bump-version.sh 0.4.0 --dry-run

# Bump and validate
./scripts/bump-version.sh 0.4.0 --validate
```

## Troubleshooting

### "WIT package not found" Error

**Problem:** Component release fails with "WIT package wasmcp:mcp@X.Y.Z not found"

**Solution:**
```bash
# 1. Check if WIT package exists
wkg get wasmcp:mcp@0.3.0-alpha.60

# 2. If not found, release WIT package first
# Go to Actions → Release WIT Package → Run workflow

# 3. Retry component release
```

### Version Mismatch

**Problem:** `verify-versions.sh` reports mismatches

**Solution:**
```bash
# Re-run bump script
./scripts/bump-version.sh <correct-version>

# Verify
./scripts/verify-versions.sh <correct-version>
```

### Release Already Exists

**Problem:** GitHub release creation fails because tag exists

**Solution:** Each workflow creates its own release with a unique tag:
- WIT: `wit-v{version}`
- Components: `components-v{version}`
- CLI: `cli-v{version}`

If you run the same workflow twice with the same version, it will fail. Delete the release/tag first or use a new version.

### Failed Cross-compilation (ARM64)

**Problem:** Linux ARM64 CLI build fails

**Solution:**
```bash
# The workflow installs gcc-aarch64-linux-gnu automatically
# If running locally:
sudo apt-get install gcc-aarch64-linux-gnu
```

## CI/CD Integration

All workflows run on `workflow_dispatch` (manual trigger only). Each workflow is completely independent:
- **WIT**: Manually triggered when interface changes
- **Components**: Manually triggered (requires WIT to exist)
- **CLI**: Manually triggered (fully independent)

This gives you full control over what to release and when.

## Security

### Supply Chain

All releases include:
- **SBOM** files (CycloneDX format) for components and CLI
- **SHA256 checksums** for all artifacts
- Signed commits (if configured)

### Verification

Users can verify downloads:
```bash
# Verify CLI download
curl -fsSL https://github.com/wasmcp/wasmcp/releases/download/v0.3.0/wasmcp-x86_64-unknown-linux-gnu.sha256 -o wasmcp.sha256
sha256sum -c wasmcp.sha256

# Verify components
curl -fsSL https://github.com/wasmcp/wasmcp/releases/download/v0.3.0/checksums-components.txt -o checksums.txt
sha256sum -c checksums.txt
```

## Release Checklist

### Pre-Release
- [ ] All tests passing (`make test` or CI)
- [ ] Version bumped: `./scripts/bump-version.sh X.Y.Z --validate`
- [ ] Changes committed and pushed
- [ ] CHANGELOG updated (if maintaining one)

### Release (run workflows in order)
- [ ] WIT package released → Tag `wit-vX.Y.Z` created
- [ ] Components released → Tag `components-vX.Y.Z` created
- [ ] CLI released → Tag `cli-vX.Y.Z` created

Each creates its own independent GitHub release.

### Post-Release
- [ ] Packages made public at https://github.com/orgs/wasmcp/packages
- [ ] Release notes updated for each release (3 separate releases)
- [ ] CLI installation tested
- [ ] Templates tested: `wasmcp new test --type tools --language rust && cd test && make compose`
- [ ] Announce release

## Metrics

### Typical Release Times

| Scenario | Time | CI Minutes |
|----------|------|------------|
| Full release (WIT + Components + CLI) | ~15 min | ~45 min |
| WIT only | ~2 min | ~2 min |
| Components only | ~12 min | ~25 min |
| CLI only | ~10 min | ~18 min |
| CLI hotfix (single platform) | ~3 min | ~3 min |

### Cost Optimization

To minimize CI costs:
- **Selective releases**: Only release what changed
- **Parallel execution**: Run Components + CLI simultaneously
- **Skip unchanged platforms**: Modify CLI workflow to build only needed platforms

## Future Improvements

1. **Automated Changelog**: Use `git-cliff` or similar
2. **Release Notes Template**: Pull request descriptions → release notes
3. **Notification Integration**: Slack/Discord webhooks on release
4. **Homebrew Formula**: Auto-update `wasmcp.rb` on release
5. **Windows Support**: Add Windows CLI builds
6. **Release Verification**: Automated smoke tests post-release
