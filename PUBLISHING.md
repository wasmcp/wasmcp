# Publishing Components to Registry

This guide explains how to publish wasmcp components to the WebAssembly registry so they can be used by others.

## Prerequisites

- `wkg` CLI tool installed ([installation guide](https://github.com/bytecodealliance/wasm-pkg-tools))
- Registry account configured (run `wkg config` if needed)

## Publishing Core Components

### 1. WIT Package

The WIT package defines all interfaces and must be published first:

```bash
# From repository root
wkg wit build
wkg publish fastertools:mcp@0.1.11.wasm
```

### 2. HTTP Transport Component

The HTTP transport has two variants:

```bash
cd components/http-transport

# Tools-only variant (for regular MCP servers)
make publish-tools
# Publishes: fastertools:mcp-http-tools-server@0.1.0

# Auth-enabled variant (for OAuth-protected servers)  
make publish-auth
# Publishes: fastertools:mcp-http-tools-auth-transport@0.1.0
```

### 3. Authorization Component

```bash
cd components/authorization
make publish
# Publishes: fastertools:mcp-authorization@0.1.0
```

## Publishing Provider Examples

### Weather Provider (Python)

```bash
cd examples/weather-py
make publish
# Publishes: fastertools:weather-py-provider@0.1.0
```

### Weather Provider (Rust)

```bash
cd examples/weather-rs
cargo component build --release
wkg publish --package 'fastertools:weather-rs-provider@0.1.0' \
  target/wasm32-wasip1/release/weather_rs.wasm
```

### Weather Provider (Go)

```bash
cd examples/weather-go
make build
wkg publish --package 'fastertools:weather-go-provider@0.1.0' \
  weather-go-provider.wasm
```

## Using Published Components

Once components are published, they can be used in any project:

### Simple MCP Server

```makefile
# Fetch and compose from registry
TRANSPORT_PKG = fastertools:mcp-http-tools-server@0.1.0
PROVIDER_PKG = your-namespace:your-provider@1.0.0

build:
	wkg get $(TRANSPORT_PKG) -o transport.wasm
	wkg get $(PROVIDER_PKG) -o provider.wasm
	wac plug --plug provider.wasm transport.wasm -o mcp-server.wasm
```

### OAuth-Protected MCP Server

```makefile
# Compose three components for auth-enabled server
TRANSPORT_PKG = fastertools:mcp-http-tools-auth-transport@0.1.0
AUTH_PKG = fastertools:mcp-authorization@0.1.0
PROVIDER_PKG = your-namespace:your-provider@1.0.0

build:
	wkg get $(TRANSPORT_PKG) -o transport.wasm
	wkg get $(AUTH_PKG) -o auth.wasm
	wkg get $(PROVIDER_PKG) -o provider.wasm
	wac plug --plug auth.wasm transport.wasm -o transport-auth.wasm
	wac plug --plug provider.wasm transport-auth.wasm -o mcp-server.wasm
```

## Versioning

Follow semantic versioning:
- Breaking changes: Bump major version (2.0.0)
- New features: Bump minor version (1.1.0)
- Bug fixes: Bump patch version (1.0.1)

Update version in:
1. Component's `Cargo.toml` or equivalent
2. Publishing command or Makefile
3. Documentation

## Registry Management

### List Your Packages

```bash
wkg list --namespace fastertools
```

### Check Package Info

```bash
wkg info fastertools:mcp-authorization
```

### Deprecate Old Versions

```bash
wkg deprecate fastertools:mcp-authorization@0.0.1
```

## Troubleshooting

### "Package already exists"

The exact package@version combination already exists. Bump the version number.

### "WIT package not found"

Ensure the WIT package is published first:
```bash
wkg wit build && wkg publish fastertools:mcp@0.1.11.wasm
```

### "Component validation failed"

Check that your component implements the correct world:
```bash
wasm-tools component wit your-component.wasm
```

## Best Practices

1. **Test Before Publishing**: Always test your component locally first
2. **Document Changes**: Update README with any API changes
3. **Use Semantic Versioning**: Help users understand compatibility
4. **Publish from CI/CD**: Automate publishing for consistency
5. **Tag Releases**: Tag git commits that correspond to published versions

## Example CI/CD Workflow

```yaml
# .github/workflows/publish.yml
name: Publish Component

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Install wkg
        run: |
          curl -fsSL https://github.com/bytecodealliance/wasm-pkg-tools/releases/latest/download/wkg-linux-x64.tar.gz | tar xz
          sudo mv wkg /usr/local/bin/
      - name: Build component
        run: cargo component build --release
      - name: Publish to registry
        run: |
          wkg publish --package "fastertools:my-component@${GITHUB_REF#refs/tags/v}" \
            target/wasm32-wasip1/release/my_component.wasm
        env:
          WKG_REGISTRY_TOKEN: ${{ secrets.WKG_REGISTRY_TOKEN }}
```