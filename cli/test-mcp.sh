#!/bin/bash
# Helper script for testing wasmcp MCP server with headless Claude
#
# Usage:
#   ./test-mcp.sh "your query here"
#   ./test-mcp.sh --config /path/to/config.json "your query"
#   ./test-mcp.sh --prompt "custom prompt"

set -e

# Default config
DEFAULT_CONFIG="/Users/coreyryan/data/mashh/wasmcp/.agent/mcp/dev-config.json"
CONFIG="$DEFAULT_CONFIG"
PROMPT=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --config)
            CONFIG="$2"
            shift 2
            ;;
        --prompt)
            PROMPT="$2"
            shift 2
            ;;
        *)
            # If no --prompt was provided, treat remaining args as the query
            if [ -z "$PROMPT" ]; then
                PROMPT="$*"
            fi
            break
            ;;
    esac
done

# Check if prompt/query provided
if [ -z "$PROMPT" ]; then
    echo "Usage: $0 [--config /path/to/config.json] [--prompt \"query\" | \"query\"]"
    echo ""
    echo "Examples:"
    echo "  $0 \"List all resources\""
    echo "  $0 \"Read wasmcp://resources/getting-started\""
    echo "  $0 --config custom.json \"test query\""
    echo "  $0 --prompt \"Read wasmcp://resources/getting-started\""
    exit 1
fi

# Verify config exists
if [ ! -f "$CONFIG" ]; then
    echo "Error: Config file not found: $CONFIG"
    exit 1
fi

# Change to /tmp to avoid claude inferring local file paths from current directory
cd /tmp

# Run headless Claude with the prompt as the query
claude --mcp-config "$CONFIG" --print "$PROMPT"
