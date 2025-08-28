#!/bin/bash
# Build script for common server variants

set -e

echo "Building server variants..."

# Tools-only variant
echo "Building server-tools..."
cp wit-variants/server-tools.wit wit/world.wit
cargo component build --features "tools" --no-default-features --release
mv target/wasm32-wasip1/release/wasmcp_server.wasm target/wasmcp-server-tools.wasm
echo "  ✓ server-tools built"

# Resources-only variant
echo "Building server-resources..."
cp wit-variants/server-resources.wit wit/world.wit
cargo component build --features "resources" --no-default-features --release
mv target/wasm32-wasip1/release/wasmcp_server.wasm target/wasmcp-server-resources.wasm
echo "  ✓ server-resources built"

# Prompts-only variant
echo "Building server-prompts..."
cp wit-variants/server-prompts.wit wit/world.wit
cargo component build --features "prompts" --no-default-features --release
mv target/wasm32-wasip1/release/wasmcp_server.wasm target/wasmcp-server-prompts.wasm
echo "  ✓ server-prompts built"

# Basic variant (tools + resources)
echo "Building server-basic..."
cp wit-variants/server-basic.wit wit/world.wit
cargo component build --features "tools,resources" --no-default-features --release
mv target/wasm32-wasip1/release/wasmcp_server.wasm target/wasmcp-server-basic.wasm
echo "  ✓ server-basic built"

# Standard variant (tools + resources + prompts)
echo "Building server-standard..."
cp wit-variants/server-standard.wit wit/world.wit
cargo component build --features "tools,resources,prompts" --no-default-features --release
mv target/wasm32-wasip1/release/wasmcp_server.wasm target/wasmcp-server-standard.wasm
echo "  ✓ server-standard built"

# Full variant (all features except sse for now)
echo "Building server-full..."
cp wit-variants/server-full.wit wit/world.wit
cargo component build --features "tools,resources,prompts" --no-default-features --release
mv target/wasm32-wasip1/release/wasmcp_server.wasm target/wasmcp-server-full.wasm
echo "  ✓ server-full built"

echo ""
echo "All server variants built successfully!"
echo "Output files:"
ls -lh target/*.wasm