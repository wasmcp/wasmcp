# GitHub Actions Workflows

This directory contains CI/CD workflows for the wasmcp project.

## Release Workflows

### WIT Package Releases

WIT packages use a reusable workflow pattern:

- **`reusable-release-wit.yml`** - Reusable workflow for publishing WIT packages
- **`release-wit-protocol.yml`** - Release protocol WIT package
- **`release-wit-server.yml`** - Release server WIT package

**Usage:** Go to Actions → Select specific WIT workflow → Run workflow with version

### Component Releases

Framework components also use a reusable workflow pattern:

- **`reusable-release-component.yml`** - Reusable workflow for building and publishing components
- **Individual component workflows:**
  - `release-http-transport.yml`
  - `release-stdio-transport.yml`
  - `release-http-notifications.yml`
  - `release-method-not-found.yml`
  - `release-tools-middleware.yml`
  - `release-resources-middleware.yml`
- **`release-all-components.yml`** - Convenience dispatcher to release all components at once

**Usage:**

- **For individual component releases/hotfixes:** Go to Actions → Select specific component workflow → Run workflow with version
- **For coordinated releases of all components:** Go to Actions → Release All Components → Run workflow with version

### Why Individual Workflows?

The new individual workflow pattern (vs. the old monolithic approach) provides:

1. **Independent versioning** - Components can have different versions
2. **Hotfix capability** - Fix and release one component without touching others
3. **Reduced blast radius** - Failures isolated to single components
4. **Clear intent** - Each workflow is self-documenting
5. **Scalability** - Adding new components = adding new files (no edits to existing workflows)
6. **Parallel releases** - Can release multiple components simultaneously

## CI Workflows

- **`ci.yml`** - Continuous integration (test, lint, build)
- **`security.yml`** - Security scanning (dependency audit, SBOM generation)

## Deprecated Workflows

- **`release-components.yml.deprecated`** - Old monolithic component release workflow (kept for reference)
