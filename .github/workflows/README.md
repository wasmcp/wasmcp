# GitHub Workflows

## CI Workflow

The CI workflow runs on:
- Every push to the `main` branch
- Every pull request targeting `main`

It performs:
1. Builds the mcp-http-component WebAssembly module
2. Builds the ftl-sdk-rust library
3. Builds the ftl-sdk-typescript package
4. Runs available tests
5. Uploads the built WASM artifact

## Release Workflow

The release workflow is triggered by pushing a version tag:

```bash
git tag v0.1.0
git push origin v0.1.0
```

It performs:
1. Builds the mcp-http-component in release mode
2. Extracts version from Cargo.toml
3. Publishes to GitHub Container Registry at:
   - `ghcr.io/bowlofarugula/mcp-http-component:<version>`
   - `ghcr.io/bowlofarugula/mcp-http-component:latest`
4. Publishes ftl-sdk-rust to crates.io
5. Publishes @fastertools/ftl-sdk to npm
6. Creates a GitHub release with the WASM artifact

## Required Secrets

Configure these secrets in your repository settings:

- `CRATES_IO_TOKEN`: API token for publishing to crates.io
  - Get from: https://crates.io/settings/tokens
  - Required for publishing the Rust SDK
  
- `NPM_TOKEN`: API token for publishing to npm
  - Get from: https://www.npmjs.com/settings/~/tokens
  - Required for publishing the TypeScript SDK

The `GITHUB_TOKEN` is automatically provided and used for GitHub Container Registry.

## Manual Release

To manually publish components:

### WebAssembly Component
```bash
cd src/mcp-http-component
cargo component build --release
wkg oci push ghcr.io/bowlofarugula/mcp-http-component:0.1.0 \
  target/wasm32-wasip1/release/mcp_http_component.wasm
```

### Rust SDK
```bash
cd src/ftl-sdk-rust
cargo publish
```

### TypeScript SDK
```bash
cd src/ftl-sdk-typescript
npm publish --access public
```