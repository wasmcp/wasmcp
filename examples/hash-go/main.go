//go:generate go run go.bytecodealliance.org/cmd/wit-bindgen-go@latest generate --world hash --out gen ./wasmcp:hash.wasm

package main

import (
	"crypto/md5"
	"crypto/sha1"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"

	"go.bytecodealliance.org/cm"
	"hash-go/gen/wasmcp/mcp/protocol"
	toolscapability "hash-go/gen/wasmcp/mcp/tools-capability"
)

func init() {
	// Export the tools-capability interface
	toolscapability.Exports.ListTools = listTools
	toolscapability.Exports.CallTool = callTool
}

// listTools returns all available hash tools
func listTools(request protocol.ListToolsRequest, client protocol.ClientContext) protocol.ListToolsResult {
	return protocol.ListToolsResult{
		Tools: cm.ToList([]protocol.Tool{
			createSHA256Tool(),
			createMD5Tool(),
			createSHA1Tool(),
		}),
		NextCursor: cm.None[protocol.Cursor](),
		Meta:       cm.None[protocol.Meta](),
	}
}

// callTool executes a hash tool by name
func callTool(request protocol.CallToolRequest, client protocol.ClientContext) cm.Option[protocol.CallToolResult] {
	switch request.Name {
	case "sha256":
		return cm.Some(executeSHA256(request))
	case "md5":
		return cm.Some(executeMD5(request))
	case "sha1":
		return cm.Some(executeSHA1(request))
	default:
		// Return None to indicate we don't handle this tool
		// Middleware will delegate to next capability
		return cm.None[protocol.CallToolResult]()
	}
}

// Tool definitions

func createSHA256Tool() protocol.Tool {
	return protocol.Tool{
		Name: "sha256",
		InputSchema: `{
			"type": "object",
			"properties": {
				"text": {
					"type": "string",
					"description": "Text to hash"
				}
			},
			"required": ["text"]
		}`,
		Options: cm.Some(protocol.ToolOptions{
			Meta:         cm.None[protocol.Meta](),
			Annotations:  cm.None[protocol.ToolAnnotations](),
			Description:  cm.Some[string]("Compute SHA-256 hash of text"),
			OutputSchema: cm.None[protocol.JSON](),
			Title:        cm.Some[string]("SHA-256 Hash"),
		}),
	}
}

func createMD5Tool() protocol.Tool {
	return protocol.Tool{
		Name: "md5",
		InputSchema: `{
			"type": "object",
			"properties": {
				"text": {
					"type": "string",
					"description": "Text to hash"
				}
			},
			"required": ["text"]
		}`,
		Options: cm.Some(protocol.ToolOptions{
			Meta:         cm.None[protocol.Meta](),
			Annotations:  cm.None[protocol.ToolAnnotations](),
			Description:  cm.Some[string]("Compute MD5 hash of text"),
			OutputSchema: cm.None[protocol.JSON](),
			Title:        cm.Some[string]("MD5 Hash"),
		}),
	}
}

func createSHA1Tool() protocol.Tool {
	return protocol.Tool{
		Name: "sha1",
		InputSchema: `{
			"type": "object",
			"properties": {
				"text": {
					"type": "string",
					"description": "Text to hash"
				}
			},
			"required": ["text"]
		}`,
		Options: cm.Some(protocol.ToolOptions{
			Meta:         cm.None[protocol.Meta](),
			Annotations:  cm.None[protocol.ToolAnnotations](),
			Description:  cm.Some[string]("Compute SHA-1 hash of text"),
			OutputSchema: cm.None[protocol.JSON](),
			Title:        cm.Some[string]("SHA-1 Hash"),
		}),
	}
}

// Tool execution

type HashInput struct {
	Text string `json:"text"`
}

func executeSHA256(request protocol.CallToolRequest) protocol.CallToolResult {
	input, err := parseHashInput(request.Arguments)
	if err != nil {
		return errorResult(err.Error())
	}

	hash := sha256.Sum256([]byte(input.Text))
	hexHash := hex.EncodeToString(hash[:])

	return textResult("SHA-256: " + hexHash)
}

func executeMD5(request protocol.CallToolRequest) protocol.CallToolResult {
	input, err := parseHashInput(request.Arguments)
	if err != nil {
		return errorResult(err.Error())
	}

	hash := md5.Sum([]byte(input.Text))
	hexHash := hex.EncodeToString(hash[:])

	return textResult("MD5: " + hexHash)
}

func executeSHA1(request protocol.CallToolRequest) protocol.CallToolResult {
	input, err := parseHashInput(request.Arguments)
	if err != nil {
		return errorResult(err.Error())
	}

	hash := sha1.Sum([]byte(input.Text))
	hexHash := hex.EncodeToString(hash[:])

	return textResult("SHA-1: " + hexHash)
}

// Helper functions

func parseHashInput(args cm.Option[protocol.JSON]) (*HashInput, error) {
	if args.None() {
		return nil, &ValidationError{Message: "Arguments required"}
	}
	argsJSON := *args.Some()
	var input HashInput
	if err := json.Unmarshal([]byte(argsJSON), &input); err != nil {
		return nil, &ValidationError{Message: "Invalid JSON: " + err.Error()}
	}

	if input.Text == "" {
		return nil, &ValidationError{Message: "Text is required"}
	}

	return &input, nil
}

func textResult(text string) protocol.CallToolResult {
	textData := protocol.TextDataText(text)

	textContent := protocol.TextContent{
		Text:    textData,
		Options: cm.None[protocol.ContentOptions](),
	}

	contentBlock := protocol.ContentBlockText(textContent)

	return protocol.CallToolResult{
		Meta:              cm.None[protocol.Meta](),
		Content:           cm.ToList([]protocol.ContentBlock{contentBlock}),
		IsError:           cm.Some(false),
		StructuredContent: cm.None[protocol.JSON](),
	}
}

func errorResult(message string) protocol.CallToolResult {
	textData := protocol.TextDataText(message)

	textContent := protocol.TextContent{
		Text:    textData,
		Options: cm.None[protocol.ContentOptions](),
	}

	contentBlock := protocol.ContentBlockText(textContent)

	return protocol.CallToolResult{
		Meta:              cm.None[protocol.Meta](),
		Content:           cm.ToList([]protocol.ContentBlock{contentBlock}),
		IsError:           cm.Some(true),
		StructuredContent: cm.None[protocol.JSON](),
	}
}

type ValidationError struct {
	Message string
}

func (e *ValidationError) Error() string {
	return e.Message
}

// main is required for the `wasip2` target, even if it isn't used.
func main() {}
