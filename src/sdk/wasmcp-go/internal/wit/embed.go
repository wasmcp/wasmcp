// Package wit embeds all WIT interface definitions internally.
// Users never interact with these files directly.
package wit

import _ "embed"

// All WIT definitions are embedded at compile time.
// These are used by the code generator but never exposed to users.

//go:embed world.wit
var worldWIT string

//go:embed mcp.wit  
var mcpWIT string

// GetWorld returns the world WIT definition
func GetWorld() string {
	return worldWIT
}

// GetMCP returns the MCP interface WIT definition
func GetMCP() string {
	return mcpWIT
}