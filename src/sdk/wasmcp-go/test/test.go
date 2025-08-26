package main

import (
	"encoding/json"
	mcp "github.com/fastertools/wasmcp-go"
	_ "github.com/fastertools/wasmcp-go/wasmcp/mcp/handler" // Import for exports registration
)

func init() {
	mcp.Handle(func(h *mcp.Handler) {
		h.Tool("test", "Test tool", mcp.Schema(`{"type": "object"}`), testHandler)
	})
}

func testHandler(args json.RawMessage) (string, error) {
	return "test response", nil
}

func main() {}