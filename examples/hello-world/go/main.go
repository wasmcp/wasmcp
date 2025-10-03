//go:build !setup

package main

import (
	"encoding/json"
	"fmt"

	"github.com/go/gen/wasi/io/v0.2.3/streams"
	errorresult "github.com/go/gen/wasmcp/mcp/v0.3.0/error-result"
	incominghandler "github.com/go/gen/wasmcp/mcp/v0.3.0/incoming-handler"
	"github.com/go/gen/wasmcp/mcp/v0.3.0/request"
	toolscallcontent "github.com/go/gen/wasmcp/mcp/v0.3.0/tools-call-content"
	toolslistresult "github.com/go/gen/wasmcp/mcp/v0.3.0/tools-list-result"
	"github.com/go/gen/wasmcp/mcp/v0.3.0/types"
	"go.bytecodealliance.org/cm"
)

func init() {
	incominghandler.Exports.Handle = Handle
}

func Handle(req request.Request, output streams.OutputStream) {
	if !req.Needs(types.ServerCapabilitiesTools) {
		incominghandler.Handle(req, output)
		return
	}

	id := req.ID()
	params := req.Params()

	if !params.IsOK() {
		_ = errorresult.Write(id, output, *params.Err())
		return
	}

	p := params.OK()
	if toolsListParams := p.ToolsList(); toolsListParams != nil {
		handleToolsList(id, output)
	} else if toolsCallParams := p.ToolsCall(); toolsCallParams != nil {
		handleToolsCall(id, toolsCallParams.Name, toolsCallParams.Arguments, output)
	}
}

func handleToolsList(id types.ID, output streams.OutputStream) {
	tools := []toolslistresult.Tool{
		{
			Name: "echo",
			InputSchema: mustMarshalJSON(map[string]any{
				"type": "object",
				"properties": map[string]any{
					"message": map[string]any{
						"type":        "string",
						"description": "The message to echo",
					},
				},
				"required": []string{"message"},
			}),
			Options: cm.Some(toolslistresult.ToolOptions{
				Description: cm.Some[string]("Echo a message back"),
				Title:       cm.Some[string]("Echo"),
			}),
		},
	}

	_ = toolslistresult.Write(id, output, cm.ToList(tools), cm.None[toolslistresult.Options]())
}

func handleToolsCall(id types.ID, name string, arguments cm.Option[types.JSON], output streams.OutputStream) {
	switch name {
	case "echo":
		result, err := handleEcho(arguments)
		if err != nil {
			_ = toolscallcontent.WriteError(id, output, err.Error())
			return
		}
		_ = toolscallcontent.WriteText(id, output, result, cm.None[toolscallcontent.Options]())

	default:
		_ = toolscallcontent.WriteError(id, output, fmt.Sprintf("Unknown tool: %s", name))
	}
}

func handleEcho(arguments cm.Option[types.JSON]) (string, error) {
	type EchoArgs struct {
		Message string `json:"message"`
	}

	var args EchoArgs
	if err := parseArgs(arguments, &args); err != nil {
		return "", err
	}

	return fmt.Sprintf("Echo: %s", args.Message), nil
}

func parseArgs(arguments cm.Option[types.JSON], target any) error {
	argsStr := "{}"
	if argPtr := arguments.Some(); argPtr != nil {
		argsStr = string(*argPtr)
	}

	if err := json.Unmarshal([]byte(argsStr), target); err != nil {
		return fmt.Errorf("failed to parse arguments: %w", err)
	}

	return nil
}

func mustMarshalJSON(v any) string {
	data, err := json.Marshal(v)
	if err != nil {
		panic(fmt.Sprintf("failed to marshal JSON: %v", err))
	}
	return string(data)
}

func main() {}
