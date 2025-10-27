# MCP Server Daemon Management

The `wasmcp` CLI includes a built-in MCP server that can run as a background daemon, making it easy to keep the server running continuously without an active terminal session.

## Quick Start

```bash
# Start server as background daemon
wasmcp mcp start

# Check if it's running
wasmcp mcp status

# View logs
wasmcp mcp logs

# Stop the server
wasmcp mcp stop
```

## Commands

### `wasmcp mcp serve`

Run the MCP server in foreground mode (attached to current terminal).

```bash
# HTTP server on default port 8085
wasmcp mcp serve

# Custom port
wasmcp mcp serve --port 9000

# Stdio transport
wasmcp mcp serve --stdio

# Verbose logging
wasmcp mcp serve -v

# Use local filesystem instead of GitHub for resources
wasmcp mcp serve --local-resources /path/to/wasmcp
```

**When to use:** Development, debugging, or when you want to see output in real-time.

**Stopping:** Press Ctrl+C to stop.

---

### `wasmcp mcp start`

Start the MCP server as a background daemon.

```bash
# Start with default settings (port 8085)
wasmcp mcp start

# Start on custom port
wasmcp mcp start --port 9000

# Start with verbose logging
wasmcp mcp start -v

# Start with local resources
wasmcp mcp start --local-resources /path/to/wasmcp
```

**Behavior:**
- Starts server in background
- Saves configuration flags for later `restart` commands
- Returns immediately (doesn't block terminal)
- Creates PID file at `~/.local/state/wasmcp/mcp-server.pid` (Linux) or `~/Library/Application Support/wasmcp/mcp-server.pid` (macOS)
- Logs to `~/.local/state/wasmcp/mcp-server.log` (Linux) or `~/Library/Application Support/wasmcp/mcp-server.log` (macOS)

**Conflict detection:**
If a server is already running, `start` will fail with an error. Use `restart` to stop and start with new settings.

---

### `wasmcp mcp stop`

Stop the background daemon gracefully.

```bash
wasmcp mcp stop
```

**Behavior:**
- Sends SIGTERM to the daemon process
- Waits up to 10 seconds for graceful shutdown
- Falls back to SIGKILL if process doesn't stop
- Removes PID file after successful shutdown

---

### `wasmcp mcp restart`

Stop the existing daemon and start a new one, merging configuration flags.

```bash
# Restart with same settings as before
wasmcp mcp restart

# Restart on different port (merges with saved flags)
wasmcp mcp restart --port 9000

# Note: Cannot disable verbose via restart - use stop + start instead
wasmcp mcp stop
wasmcp mcp start --port 8085  # Starts without verbose
```

**Flag Merging Behavior:**
- New flags override saved flags
- Omitted flags use previously saved values
- **Note:** Verbose flag uses OR logic - once enabled, it cannot be disabled via restart (use `stop` then `start` to turn it off)
- Example: If started with `--port 8085 -v`, then restarted with `--port 9000`, the server runs on port 9000 with verbose still enabled

**Difference from `start`:**
- `start` - Completely replaces all saved flags
- `restart` - Merges new flags with saved flags

---

### `wasmcp mcp status`

Check if the daemon is running and perform a health check.

```bash
wasmcp mcp status
```

**Output:**
```
✓ Server running (PID: 12345)
  Port: 8085
  Health: OK (wasmcp-mcp-server 0.4.3 - 5 tools, 18 resources)
```

**Health Check:**
The status command performs a full MCP protocol handshake:
1. Connects to the server via HTTP
2. Sends `initialize` request
3. Calls `tools/list` and `resources/list`
4. Validates responses and counts available tools/resources

**Possible States:**
- `✓ Server running` - Daemon is alive and responding to MCP requests
- `✗ Server stopped` - No daemon running
- `⚠ Server stopped (stale PID file detected)` - PID file exists but process is dead (automatically cleaned up)
- `Health: FAILED (...)` - Process is running but MCP protocol is not responding correctly

---

### `wasmcp mcp logs`

View daemon logs.

```bash
# Show all logs
wasmcp mcp logs

# Follow logs in real-time (like tail -f)
wasmcp mcp logs -f
```

**Log Contents:**
- Server startup messages
- MCP requests and responses (when verbose enabled)
- Errors and warnings
- Tool execution results

**Log Rotation:**
Logs are appended indefinitely. Use `wasmcp mcp clean` to clear old logs.

---

### `wasmcp mcp clean`

Remove all daemon state files.

```bash
wasmcp mcp clean
```

**Removes:**
- PID file (`mcp-server.pid`)
- Log file (`mcp-server.log`)
- Saved flags file (`mcp-server.flags`)

**Warning:** This does NOT stop a running server. Stop the server first with `wasmcp mcp stop`, or it will leave orphaned processes.

## State Directory Locations

wasmcp follows XDG Base Directory specification for state files:

**Linux/Unix:**
- `$XDG_STATE_HOME/wasmcp/` (if `XDG_STATE_HOME` is set)
- `~/.local/state/wasmcp/` (default)

**macOS:**
- `~/Library/Application Support/wasmcp/`

**Files:**
- `mcp-server.pid` - Process ID of running daemon
- `mcp-server.log` - Daemon logs
- `mcp-server.flags` - Saved configuration flags (JSON)

## Common Workflows

### Development Workflow

```bash
# Start daemon for development
wasmcp mcp start -v

# Check it's working
wasmcp mcp status

# Follow logs while testing
wasmcp mcp logs -f

# Make changes to wasmcp source...

# Restart with new binary
wasmcp mcp restart

# Stop when done
wasmcp mcp stop
```

### Production Deployment

```bash
# Start on specific port without verbose logging
wasmcp mcp start --port 8085

# Verify it's healthy
wasmcp mcp status

# Check logs occasionally
wasmcp mcp logs | tail -100

# Restart after updates
wasmcp mcp restart
```

### Testing Local Resource Changes

```bash
# Start with local resources for development
wasmcp mcp start --local-resources /path/to/wasmcp/repo -v

# Edit documentation files locally...
# Changes are immediately reflected (no restart needed for resource content)

# When done, restart without local resources
wasmcp mcp restart
```

### Troubleshooting

**Server won't start:**
```bash
# Check if already running
wasmcp mcp status

# If stale PID detected, clean it
wasmcp mcp clean

# Try starting again
wasmcp mcp start
```

**Port conflict:**
```bash
# Check what's using the port
lsof -i :8085

# Start on different port
wasmcp mcp start --port 9000
```

**Server not responding:**
```bash
# Check logs
wasmcp mcp logs

# Check health
wasmcp mcp status

# If stuck, force restart
wasmcp mcp stop
wasmcp mcp start
```

**Orphaned processes:**
```bash
# If clean removed PID file but process still running
ps aux | grep wasmcp

# Kill manually
kill <PID>

# Clean up state
wasmcp mcp clean
```

## Platform Notes

### macOS

The daemon uses process spawning (not fork) to avoid issues with the Objective-C runtime. This is handled automatically.

### Linux

The daemon uses traditional forking via the `daemonize` crate for better session management.

### Windows

Daemon mode is not yet supported on Windows. Use `wasmcp mcp serve` (foreground mode) instead.

## See Also

- [CLI README](../cli/README.md) - Full CLI documentation
- [MCP Resources](../cli/README.md#available-resources) - Available MCP resources
- [MCP Tools](../cli/README.md#available-tools) - Available MCP tools
