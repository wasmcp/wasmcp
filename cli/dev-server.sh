#!/bin/bash
# Development server management script for wasmcp mcp serve
#
# Usage:
#   ./dev-server.sh [--local-resources <PATH>] start   - Build and start server
#   ./dev-server.sh stop                                - Stop server
#   ./dev-server.sh [--local-resources <PATH>] restart  - Rebuild and restart server
#   ./dev-server.sh status                              - Check if server is running
#   ./dev-server.sh logs [-f]                           - Show recent logs
#   ./dev-server.sh clean                               - Stop server and clean logs
#
# Flags:
#   --local-resources <PATH>  - Override GitHub resource fetching with local filesystem

set -e

# Configuration
PORT=8085
BINARY_PATH="./target/aarch64-apple-darwin/release/wasmcp"
WASMCP_SUPPORT_DIR="$HOME/Library/Application Support/wasmcp"
PID_FILE="$WASMCP_SUPPORT_DIR/dev-server.pid"
LOG_FILE="$WASMCP_SUPPORT_DIR/dev-server.log"
LOCAL_RESOURCES=""  # Optional path to local resources (set via --local-resources flag)

# Ensure support directory exists
mkdir -p "$WASMCP_SUPPORT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if server is running
is_running() {
    if [ -f "$PID_FILE" ]; then
        PID=$(cat "$PID_FILE")
        if ps -p "$PID" > /dev/null 2>&1; then
            return 0  # Running
        else
            # PID file exists but process is dead
            rm -f "$PID_FILE"
            return 1  # Not running
        fi
    fi
    return 1  # Not running
}

# Get server PID
get_pid() {
    if [ -f "$PID_FILE" ]; then
        cat "$PID_FILE"
    fi
}

# Build the CLI
build() {
    log_info "Building wasmcp CLI (release mode)..."
    if cargo build --release --target aarch64-apple-darwin; then
        log_success "Build completed"
        return 0
    else
        log_error "Build failed"
        return 1
    fi
}

# Start the server
start() {
    if is_running; then
        log_warn "Server is already running (PID: $(get_pid))"
        log_info "Use './dev-server.sh stop' to stop it first"
        exit 1
    fi

    # Build if binary doesn't exist
    if [ ! -f "$BINARY_PATH" ]; then
        log_warn "Binary not found, building..."
        build || exit 1
    fi

    log_info "Starting wasmcp MCP server on port $PORT..."

    # Build command with optional local resources
    CMD="$BINARY_PATH mcp serve --port $PORT -v"
    if [ -n "$LOCAL_RESOURCES" ]; then
        log_info "Using local resources from: $LOCAL_RESOURCES"
        CMD="$CMD --local-resources $LOCAL_RESOURCES"
    fi

    # Start server in background with verbose logging
    nohup $CMD > "$LOG_FILE" 2>&1 &

    # Save PID
    echo $! > "$PID_FILE"

    # Wait a moment and check if it started
    sleep 1

    if is_running; then
        log_success "Server started (PID: $(get_pid))"
        log_info "Logs: tail -f $LOG_FILE"
        log_info "Config: /Users/coreyryan/data/mashh/wasmcp/.agent/mcp/dev-config.json"
        log_info "Test: claude --print --mcp-config /Users/coreyryan/data/mashh/wasmcp/.agent/mcp/dev-config.json -- \"test\""
    else
        log_error "Server failed to start"
        log_info "Check logs: cat $LOG_FILE"
        rm -f "$PID_FILE"
        exit 1
    fi
}

# Stop the server
stop() {
    if ! is_running; then
        log_warn "Server is not running"
        # Clean up stale PID file
        rm -f "$PID_FILE"

        # Check for any wasmcp mcp serve processes
        log_info "Checking for orphaned processes..."
        ORPHANED=$(pgrep -f "wasmcp mcp serve" || true)
        if [ -n "$ORPHANED" ]; then
            log_warn "Found orphaned wasmcp process(es): $ORPHANED"
            log_info "Kill with: kill $ORPHANED"
        fi
        exit 0
    fi

    PID=$(get_pid)
    log_info "Stopping server (PID: $PID)..."

    # Try graceful shutdown first
    kill "$PID" 2>/dev/null || true

    # Wait up to 5 seconds for graceful shutdown
    for i in {1..5}; do
        if ! ps -p "$PID" > /dev/null 2>&1; then
            break
        fi
        sleep 1
    done

    # Force kill if still running
    if ps -p "$PID" > /dev/null 2>&1; then
        log_warn "Graceful shutdown failed, forcing..."
        kill -9 "$PID" 2>/dev/null || true
    fi

    rm -f "$PID_FILE"
    log_success "Server stopped"
}

# Restart the server
restart() {
    log_info "Restarting server..."
    stop
    sleep 1
    build || exit 1
    start
}

# Show server status
status() {
    if is_running; then
        PID=$(get_pid)
        log_success "Server is running (PID: $PID)"

        # Show process info
        echo ""
        ps -p "$PID" -o pid,ppid,command,%cpu,%mem,etime

        # Check if port is listening
        echo ""
        if lsof -i ":$PORT" > /dev/null 2>&1; then
            log_success "Port $PORT is listening"
            lsof -i ":$PORT"
        else
            log_warn "Port $PORT is not listening"
        fi

        # Test MCP server response with proper handshake
        echo ""
        log_info "Testing MCP server response..."

        # First, send initialize request
        INIT_RESPONSE=$(curl -s -X POST "http://127.0.0.1:$PORT/mcp" \
            -H "Content-Type: application/json" \
            -H "Accept: application/json, text/event-stream" \
            -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"dev-status","version":"1.0.0"}}}' \
            --max-time 2 2>&1 || echo "error")

        if echo "$INIT_RESPONSE" | grep -q "\"result\""; then
            log_success "MCP server responding correctly"

            # Count capabilities
            TOOLS=$(echo "$INIT_RESPONSE" | grep -o '"tools"' | wc -l | tr -d ' ')
            RESOURCES=$(echo "$INIT_RESPONSE" | grep -o '"resources"' | wc -l | tr -d ' ')
            PROMPTS=$(echo "$INIT_RESPONSE" | grep -o '"prompts"' | wc -l | tr -d ' ')

            echo "  Server initialized successfully"
            if [ "$TOOLS" -gt 0 ]; then
                echo "  ✓ Tools capability"
            fi
            if [ "$RESOURCES" -gt 0 ]; then
                echo "  ✓ Resources capability"
            fi
            if [ "$PROMPTS" -gt 0 ]; then
                echo "  ✓ Prompts capability"
            fi
        elif echo "$INIT_RESPONSE" | grep -q "error"; then
            log_warn "MCP server not responding (connection failed)"
        else
            log_warn "MCP server returned unexpected response"
            echo "  Response: ${INIT_RESPONSE:0:200}"
        fi

        # Show recent log lines
        echo ""
        log_info "Recent logs (last 10 lines):"
        if [ -f "$LOG_FILE" ]; then
            tail -10 "$LOG_FILE"
        else
            log_warn "No log file found"
        fi
    else
        log_warn "Server is not running"

        # Check for orphaned processes
        ORPHANED=$(pgrep -f "wasmcp mcp serve" || true)
        if [ -n "$ORPHANED" ]; then
            log_warn "Found orphaned wasmcp process(es): $ORPHANED"
            log_info "Kill with: kill $ORPHANED"
        fi

        # Check if port is still in use
        if lsof -i ":$PORT" > /dev/null 2>&1; then
            log_warn "Port $PORT is in use by another process:"
            lsof -i ":$PORT"
        fi
    fi
}

# Show logs
logs() {
    if [ ! -f "$LOG_FILE" ]; then
        log_warn "No log file found at $LOG_FILE"
        exit 0
    fi

    if [ "$1" == "-f" ] || [ "$1" == "--follow" ]; then
        log_info "Following logs (Ctrl+C to stop)..."
        tail -f "$LOG_FILE"
    else
        log_info "Recent logs (last 50 lines):"
        tail -50 "$LOG_FILE"
    fi
}

# Clean up
clean() {
    log_info "Cleaning up..."
    stop 2>/dev/null || true

    if [ -f "$LOG_FILE" ]; then
        rm -f "$LOG_FILE"
        log_success "Removed log file"
    fi

    if [ -f "$PID_FILE" ]; then
        rm -f "$PID_FILE"
        log_success "Removed PID file"
    fi

    log_success "Clean complete"
}

# Rebuild without restarting
rebuild() {
    log_info "Rebuilding (server will continue running)..."
    build
    log_info "Rebuild complete. Use './dev-server.sh restart' to restart with new binary"
}

# Parse arguments
COMMAND=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --local-resources)
            LOCAL_RESOURCES="$2"
            shift 2
            ;;
        start|stop|restart|status|logs|clean|rebuild)
            COMMAND="$1"
            shift
            break
            ;;
        *)
            COMMAND="$1"
            shift
            break
            ;;
    esac
done

# Main command handler
case "${COMMAND:-}" in
    start)
        start
        ;;
    stop)
        stop
        ;;
    restart)
        restart
        ;;
    status)
        status
        ;;
    logs)
        logs "$@"
        ;;
    clean)
        clean
        ;;
    rebuild)
        rebuild
        ;;
    *)
        echo "wasmcp MCP Development Server Manager"
        echo ""
        echo "Usage: $0 [flags] <command>"
        echo ""
        echo "Flags:"
        echo "  --local-resources <PATH>  - Override GitHub fetching with local filesystem"
        echo "                              (provide absolute path to repository root)"
        echo ""
        echo "Commands:"
        echo "  start    - Build and start the server"
        echo "  stop     - Stop the server"
        echo "  restart  - Rebuild and restart the server"
        echo "  status   - Show server status and recent logs"
        echo "  logs     - Show logs (use -f to follow)"
        echo "  rebuild  - Rebuild without restarting"
        echo "  clean    - Stop server and remove logs"
        echo ""
        echo "Examples:"
        echo "  $0 start                                      # Start server"
        echo "  $0 --local-resources /path/to/wasmcp restart  # Restart with local resources"
        echo "  $0 logs -f                                    # Follow logs"
        echo ""
        exit 1
        ;;
esac
