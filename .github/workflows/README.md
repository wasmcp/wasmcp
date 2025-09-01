# GitHub Workflows

## CI Workflow

The CI workflow runs on:
- Every push to the `main` branch
- Every pull request targeting `main`

It performs:
1. Builds the mcp-http-transport WebAssembly component
2. Runs available tests
3. Uploads the built Wasm artifact

## Release Workflow

The release workflow is triggered by pushing a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

It performs:
1. Builds the mcp-http-transport in release mode
2. Extracts version from Cargo.toml
3. Publishes to GitHub Container Registry at:
   - `ghcr.io/fastertools/mcp-http-transport:<version>`
   - `ghcr.io/fastertools/mcp-http-transport:latest`
4. Creates a GitHub release with the Wasm artifact

## Required Secrets

Configure these secrets in your repository settings:

The `GITHUB_TOKEN` is automatically provided and used for GitHub Container Registry.

## Manual Release

To manually publish components:

### WebAssembly Component
```bash
cd components/http-transport
cargo component build --release
wkg oci push ghcr.io/fastertools/mcp-http-transport:0.1.0 \
  target/wasm32-wasip1/release/mcp_transport_http.wasm
```