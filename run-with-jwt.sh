#!/bin/bash

# Run MCP server with JWT configuration using wasmtime
# Usage: ./run-with-jwt.sh <issuer> <audience> [jwks_uri]
# Example: ./run-with-jwt.sh https://dev-abc123.auth0.com/ https://my-api

ISSUER=${1:-"https://your-auth.auth0.com/"}
AUDIENCE=${2:-"https://your-api"}
JWKS_URI=${3:-"${ISSUER}.well-known/jwks.json"}

echo "Starting MCP server with JWT configuration:"
echo "  Issuer: $ISSUER"
echo "  Audience: $AUDIENCE"
echo "  JWKS URI: $JWKS_URI"
echo ""

# Run with wasmtime serve, passing WASI config variables
wasmtime serve \
  -Scli \
  -Sconfig \
  -Sconfig-var="jwt.expected_issuer=$ISSUER" \
  -Sconfig-var="jwt.expected_audience=$AUDIENCE" \
  -Sconfig-var="jwt.jwks_uri=$JWKS_URI" \
  -Sconfig-var="oauth.resource_url=http://localhost:8080" \
  -Sconfig-var="oauth.auth_server=$ISSUER" \
  -Sconfig-var="oauth.auth_issuer=$ISSUER" \
  test-auth-server.wasm