#!/bin/bash

# Usage: ./run-server.sh <issuer> <audience> [jwks_uri] [port]
# Example: ./run-server.sh https://dev-abc123.auth0.com/ https://my-api

ISSUER=${1:-"https://your-auth.auth0.com/"}
AUDIENCE=${2:-"https://your-api"}
JWKS_URI=${3:-"${ISSUER}.well-known/jwks.json"}
PORT=${4:-3000}

# Create a temporary wasmtime config file
cat > /tmp/wasmtime-config.toml << EOF
[[wasi.config]]
"jwt.expected_issuer" = "$ISSUER"
"jwt.expected_audience" = "$AUDIENCE"
"jwt.jwks_uri" = "$JWKS_URI"

# OAuth discovery metadata (adjust as needed)
"oauth.resource_url" = "http://localhost:$PORT"
"oauth.auth_server" = "$ISSUER"
"oauth.auth_issuer" = "$ISSUER"
EOF

echo "Starting MCP server with:"
echo "  Issuer: $ISSUER"
echo "  Audience: $AUDIENCE"
echo "  JWKS URI: $JWKS_URI"
echo "  Port: $PORT"
echo ""

# Run with wasmtime
wasmtime run \
  --wasi config=/tmp/wasmtime-config.toml \
  --tcplisten 127.0.0.1:$PORT \
  test-auth-server.wasm