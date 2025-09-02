#!/bin/bash

# Run MCP server with JWT configuration
# Usage: ./run-with-config.sh [server.wasm] [jwt_issuer] [jwt_audience] [jwks_uri]

SERVER_WASM=${1:-test-auth-server.wasm}
JWT_ISSUER=${2:-https://your-auth.auth0.com/}
JWT_AUDIENCE=${3:-https://your-mcp-api}
JWKS_URI=${4:-${JWT_ISSUER}.well-known/jwks.json}
PORT=${PORT:-3000}

echo "Starting MCP server with OAuth configuration:"
echo "  JWT Issuer: $JWT_ISSUER"
echo "  JWT Audience: $JWT_AUDIENCE"
echo "  JWKS URI: $JWKS_URI"
echo "  Port: $PORT"
echo ""

# For Spin (doesn't support WASI config yet, so we use env vars)
echo "Running with Spin (no WASI config support)..."
JWT_ISSUER="$JWT_ISSUER" \
JWT_AUDIENCE="$JWT_AUDIENCE" \
JWT_JWKS_URI="$JWKS_URI" \
spin up --from "$SERVER_WASM" --listen "127.0.0.1:$PORT"