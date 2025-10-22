//go:generate go tool wit-bindgen-go generate --world calculator-go --out internal wit

// Package main implements a Calculator Tools MCP component
//
// A tools capability that provides basic calculator operations with notification support.
package main

import (
	"encoding/json"
	"fmt"
	"strconv"

	"go.bytecodealliance.org/cm"

	"github.com/wasmcp/wasmcp/examples/calculator-go/internal/wasmcp/protocol/mcp"
	servermessages "github.com/wasmcp/wasmcp/examples/calculator-go/internal/wasmcp/protocol/server-messages"
	"github.com/wasmcp/wasmcp/examples/calculator-go/internal/wasmcp/protocol/tools"
	"github.com/wasmcp/wasmcp/examples/calculator-go/internal/wasmcp/server/notifications"
	"github.com/wasmcp/wasmcp/examples/calculator-go/internal/wasi/io/streams"
)

func init() {
	tools.Exports.ListTools = listTools
	tools.Exports.CallTool = callTool
}

// listTools returns the list of available calculator tools
func listTools(
	ctx servermessages.Context,
	request mcp.ListToolsRequest,
	clientStream cm.Option[cm.Rep],
) cm.Result[tools.ErrorCodeShape, mcp.ListToolsResult, mcp.ErrorCode] {
	var result cm.Result[tools.ErrorCodeShape, mcp.ListToolsResult, mcp.ErrorCode]

	toolsSlice := []mcp.Tool{
		{
			Name: "add",
			InputSchema: `{
				"type": "object",
				"properties": {
					"a": {"type": "number", "description": "First number"},
					"b": {"type": "number", "description": "Second number"}
				},
				"required": ["a", "b"]
			}`,
			Options: cm.Some(mcp.ToolOptions{
				Title:       cm.Some[string]("Add"),
				Description: cm.Some[string]("Add two numbers together"),
			}),
		},
		{
			Name: "subtract",
			InputSchema: `{
				"type": "object",
				"properties": {
					"a": {"type": "number", "description": "Number to subtract from"},
					"b": {"type": "number", "description": "Number to subtract"}
				},
				"required": ["a", "b"]
			}`,
			Options: cm.None[mcp.ToolOptions](),
		},
		{
			Name: "factorial",
			InputSchema: `{
				"type": "object",
				"properties": {
					"n": {
						"type": "integer",
						"description": "Calculate factorial of this number",
						"minimum": 0,
						"maximum": 20
					}
				},
				"required": ["n"]
			}`,
			Options: cm.Some(mcp.ToolOptions{
				Title:       cm.Some[string]("Factorial"),
				Description: cm.Some[string]("Calculate factorial with progress updates"),
			}),
		},
	}

	result.SetOK(mcp.ListToolsResult{
		Tools: cm.ToList(toolsSlice),
	})
	return result
}

// callTool executes a tool call and returns the result
func callTool(
	ctx servermessages.Context,
	request mcp.CallToolRequest,
	clientStream cm.Option[cm.Rep],
) cm.Option[mcp.CallToolResult] {
	// Extract the actual output stream if present
	var outStream *streams.OutputStream
	if rep := clientStream.Some(); rep != nil {
		// Convert cm.Rep (uint32) to OutputStream (cm.Resource)
		streamHandle := cm.Reinterpret[streams.OutputStream](*rep)
		outStream = &streamHandle
	}

	switch request.Name {
	case "add":
		return cm.Some(executeOperation(request.Arguments, func(a, b float64) float64 {
			return a + b
		}))
	case "subtract":
		return cm.Some(executeOperation(request.Arguments, func(a, b float64) float64 {
			return a - b
		}))
	case "factorial":
		return cm.Some(executeFactorial(ctx, request, outStream))
	default:
		return cm.None[mcp.CallToolResult]() // We don't handle this tool
	}
}

// executeOperation is a helper for binary arithmetic operations
func executeOperation(arguments cm.Option[mcp.JSON], op func(float64, float64) float64) mcp.CallToolResult {
	a, b, err := parseArgs(arguments)
	if err != nil {
		return errorResult(err.Error())
	}

	result := op(a, b)
	return successResult(fmt.Sprintf("%v", result))
}

// parseArgs parses the JSON arguments to extract 'a' and 'b' parameters
func parseArgs(arguments cm.Option[mcp.JSON]) (float64, float64, error) {
	argsStrPtr := arguments.Some()
	if argsStrPtr == nil {
		return 0, 0, fmt.Errorf("missing arguments")
	}

	var args map[string]interface{}
	if err := json.Unmarshal([]byte(*argsStrPtr), &args); err != nil {
		return 0, 0, fmt.Errorf("invalid JSON arguments: %v", err)
	}

	a, ok := args["a"].(float64)
	if !ok {
		return 0, 0, fmt.Errorf("missing or invalid parameter 'a'")
	}

	b, ok := args["b"].(float64)
	if !ok {
		return 0, 0, fmt.Errorf("missing or invalid parameter 'b'")
	}

	return a, b, nil
}

// executeFactorial calculates factorial with progress notifications
func executeFactorial(
	ctx servermessages.Context,
	request mcp.CallToolRequest,
	clientStream *streams.OutputStream,
) mcp.CallToolResult {
	n, err := parseFactorialArg(request.Arguments)
	if err != nil {
		return errorResult(err.Error())
	}

	// Send initial progress notification if stream is available
	if clientStream != nil {
		msg := fmt.Sprintf("Starting factorial calculation for %d!", n)
		_ = notifications.Log(*clientStream, msg, mcp.LogLevelInfo, cm.Some[string]("factorial"))
	}

	// Calculate factorial with progress updates
	var result uint64 = 1
	for i := uint64(1); i <= n; i++ {
		// Check for overflow
		if result > 0 && i > 0 && result > (^uint64(0))/i {
			return errorResult(fmt.Sprintf("Integer overflow: %d! is too large", n))
		}
		result *= i

		// Send progress notification every few steps (to avoid overwhelming)
		if clientStream != nil && (i%3 == 0 || i == n) {
			var prevResult uint64
			if i > 0 {
				prevResult = result / i
			}
			msg := fmt.Sprintf("Computing: %d * %d = %d", i, prevResult, result)
			_ = notifications.Log(*clientStream, msg, mcp.LogLevelDebug, cm.Some[string]("factorial"))
		}
	}

	// Send completion notification
	if clientStream != nil {
		msg := fmt.Sprintf("Factorial calculation complete: %d! = %d", n, result)
		_ = notifications.Log(*clientStream, msg, mcp.LogLevelInfo, cm.Some[string]("factorial"))
	}

	return successResult(strconv.FormatUint(result, 10))
}

// parseFactorialArg parses the factorial argument
func parseFactorialArg(arguments cm.Option[mcp.JSON]) (uint64, error) {
	argsStrPtr := arguments.Some()
	if argsStrPtr == nil {
		return 0, fmt.Errorf("missing arguments")
	}

	var args map[string]interface{}
	if err := json.Unmarshal([]byte(*argsStrPtr), &args); err != nil {
		return 0, fmt.Errorf("invalid JSON arguments: %v", err)
	}

	nFloat, ok := args["n"].(float64)
	if !ok {
		return 0, fmt.Errorf("missing or invalid parameter 'n'")
	}

	n := uint64(nFloat)
	if n > 20 {
		return 0, fmt.Errorf("input too large: %d (maximum is 20)", n)
	}

	return n, nil
}

// successResult creates a successful tool result
func successResult(text string) mcp.CallToolResult {
	textData := mcp.TextDataText(text)
	textContent := mcp.TextContent{
		Text:    textData,
		Options: cm.None[mcp.ContentOptions](),
	}
	contentBlock := mcp.ContentBlockText(textContent)

	return mcp.CallToolResult{
		Content: cm.ToList([]mcp.ContentBlock{contentBlock}),
		IsError: cm.None[bool](),
	}
}

// errorResult creates an error tool result
func errorResult(message string) mcp.CallToolResult {
	textData := mcp.TextDataText(message)
	textContent := mcp.TextContent{
		Text:    textData,
		Options: cm.None[mcp.ContentOptions](),
	}
	contentBlock := mcp.ContentBlockText(textContent)

	return mcp.CallToolResult{
		Content: cm.ToList([]mcp.ContentBlock{contentBlock}),
		IsError: cm.Some(true),
	}
}

// main is required for the wasi target, even if it isn't used
func main() {}
