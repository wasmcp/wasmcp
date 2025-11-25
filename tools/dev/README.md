# wasmcp Development Tools

Local development toolkit for wasmcp. Replaces fragile shell scripts with a robust Rust-based CLI.

**Location**: `tools/dev/` - Development infrastructure, not production code.

## Building

```bash
# From tools/dev directory
make build

# Or from workspace root (specify your platform target)
cargo build --release --target aarch64-apple-darwin --manifest-path tools/dev/Cargo.toml

# Run directly
cargo run --release --target aarch64-apple-darwin --manifest-path tools/dev/Cargo.toml -- --help
```

The binary will be at `tools/dev/target/<your-platform>/release/dev-tools`

**Note**: tools/dev is excluded from the workspace to avoid inheriting the `wasm32-wasip2` default target. You must specify `--target` explicitly or configure your own build wrapper.

## Installation (Optional)

```bash
# Install as 'wasmcp-dev' command
make install
```

## Quick Access

Configure your own alias or wrapper to avoid typing the full cargo command. Example approaches:

```bash
# Shell alias (add to ~/.bashrc or ~/.zshrc)
alias dt='cargo run --release --target aarch64-apple-darwin --manifest-path tools/dev/Cargo.toml --'

# Then use:
dt --help
dt deps status
dt build
```

## Usage

### Managing WIT Dependencies

**Problem**: During local development, you want `deps.toml` files to point to local WIT directories instead of remote URLs.

**Solution**: Use the `deps` subcommand to dynamically patch all deps.toml files in the workspace.

```bash
# Show current status of all deps.toml files
<your-command> deps status

# Patch all deps.toml to use local paths (default: ./wit-local)
<your-command> deps local

# Patch to use custom local path
<your-command> deps local --path /path/to/local/wit

# Restore original deps.toml from backups
<your-command> deps restore
```

(Replace `<your-command>` with your configured alias or the full cargo run command)

**How it works**:
1. Finds all `wit/deps.toml` files in workspace
2. Creates `.toml.backup` files before patching
3. Converts URL dependencies to local `{ path = "..." }` dependencies
4. Can restore originals at any time

### Building Components

Build all workspace components in the correct order:

```bash
# Build everything (CLI, crates, examples)
<your-command> build

# Build only specific components
<your-command> build --only session-store,transport
```

This replaces the manual process from the shell script:
- Kills processes on port 3000
- **Nukes existing `wit/deps` directories** before running `wit-deps update` (ensures clean state)
- Runs `wit-deps update` for each component
- Builds with `make work` or `cargo build --release --target wasm32-wasip2`

### Composing Components

The `compose` command is a pass-through to `wasmcp compose server` with convenience features:

```bash
# Compose specific components
<your-command> compose \
  ./examples/counter-middleware/target/wasm32-wasip2/release/counter_middleware.wasm \
  ./examples/calculator-rs/target/wasm32-wasip2/release/calculator.wasm

# Add local overrides for all core components (transport, server-io, kv-store, etc.)
<your-command> compose \
  ./examples/counter-middleware/target/wasm32-wasip2/release/counter_middleware.wasm \
  ./examples/calculator-rs/target/wasm32-wasip2/release/calculator.wasm \
  --local-overrides

# Custom output path and force overwrite
<your-command> compose \
  ./examples/counter-middleware/target/wasm32-wasip2/release/counter_middleware.wasm \
  -o .agent/sse-spin.wasm --force --local-overrides

# Pass through any wasmcp compose arguments after --
<your-command> compose \
  ./examples/counter-middleware/target/wasm32-wasip2/release/counter_middleware.wasm \
  --local-overrides \
  -- --runtime wasmtime --override-custom-component ./custom.wasm
```

The `--local-overrides` flag automatically adds all local override flags:
- `--override-transport ./target/wasm32-wasip2/release/transport.wasm`
- `--override-server-io ./target/wasm32-wasip2/release/server_io.wasm`
- `--override-kv-store ./target/wasm32-wasip2/release/kv_store-d2.wasm`
- `--override-session-store ./target/wasm32-wasip2/release/session_store.wasm`
- And all other middleware overrides

### Running Composed Components

```bash
# Run with Spin
<your-command> run spin

# Run with Wasmtime (not yet implemented)
<your-command> run wasmtime

# Custom wasm path
<your-command> run spin --wasm /tmp/my-composed.wasm
```

## Full Workflow Example

Complete development workflow:

```bash
# 1. Patch deps.toml to use local WIT (if needed)
<your-command> deps local

# 2. Build all components
<your-command> build

# 3. Compose with local overrides
<your-command> compose \
  ./examples/counter-middleware/target/wasm32-wasip2/release/counter_middleware.wasm \
  ./examples/calculator-rs/target/wasm32-wasip2/release/calculator.wasm \
  -o .agent/sse-spin.wasm --force --local-overrides

# 4. Run with Spin
<your-command> run spin
```

## Extending

### Adding New Components

Edit `tools/dev/src/build.rs` and add to the `get_components()` function:

```rust
Component::new("my-component", "crates/my-component", false),
```

Set `has_makefile` to `true` if the component uses `make work`, `false` if it uses `cargo build`.

### Adding Runtime Support

Edit `tools/dev/src/compose.rs` and implement the runtime function (e.g., `run_with_wasmtime`).

## Architecture

- `main.rs` - CLI structure and argument parsing
- `deps.rs` - deps.toml patching logic (local/restore/status)
- `build.rs` - Component build orchestration
- `compose.rs` - Component composition and runtime execution
