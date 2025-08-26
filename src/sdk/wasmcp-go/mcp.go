// Package mcp provides the Go SDK for building MCP (Model Context Protocol) handlers
// in WebAssembly components, following the same patterns as the Spin Go SDK.
package mcp

import (
	"encoding/json"
	"fmt"

	"github.com/fastertools/wasmcp/src/sdk/wasmcp-go/wasmcp/mcp/handler"
	"go.bytecodealliance.org/cm"
	
	// Enable WASI HTTP support for standard net/http package
	_ "github.com/ydnar/wasi-http-go/wasihttp"
)

// ToolFunc is the function signature for tool implementations.
// It receives JSON arguments and returns a string result or error.
type ToolFunc func(args json.RawMessage) (string, error)

// ResourceFunc is the function signature for resource implementations.
// It returns the resource content as a string or error.
type ResourceFunc func() (string, error)

// PromptFunc is the function signature for prompt implementations.
// It receives JSON arguments and returns prompt messages or error.
type PromptFunc func(args json.RawMessage) ([]PromptMessage, error)

// PromptMessage represents a message in a prompt conversation.
type PromptMessage struct {
	Role    string `json:"role"`    // "user" or "assistant"
	Content string `json:"content"` // Message content
}

// PromptArgument defines an argument for a prompt.
type PromptArgument struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	Required    bool   `json:"required"`
}

// toolDef holds the definition and handler for a tool.
type toolDef struct {
	name        string
	description string
	schema      json.RawMessage
	handler     ToolFunc
}

// resourceDef holds the definition and handler for a resource.
type resourceDef struct {
	uri         string
	name        string
	description string
	mimeType    string
	handler     ResourceFunc
}

// promptDef holds the definition and handler for a prompt.
type promptDef struct {
	name        string
	description string
	arguments   []PromptArgument
	handler     PromptFunc
}

// Handler collects all MCP components (tools, resources, prompts).
type Handler struct {
	tools     map[string]*toolDef
	resources map[string]*resourceDef
	prompts   map[string]*promptDef
}

// The global handler instance that will be set via Handle().
// This follows the same pattern as Spin's global handler.
var globalHandler = &Handler{
	tools:     make(map[string]*toolDef),
	resources: make(map[string]*resourceDef),
	prompts:   make(map[string]*promptDef),
}

// Handle sets up the MCP handler. It must be called in an init() function.
// This follows the same pattern as spinhttp.Handle().
//
// Example:
//
//	func init() {
//	    mcp.Handle(func(h *mcp.Handler) {
//	        h.Tool("echo", "Echo a message", echoSchema, echoHandler)
//	        h.Resource("config://app", "App config", "text/plain", configHandler)
//	        h.Prompt("greeting", "Generate greeting", greetingArgs, greetingHandler)
//	    })
//	}
func Handle(fn func(*Handler)) {
	fn(globalHandler)
	
	// Set up the generated exports
	setupExports()
}

// setupExports sets up the wit-bindgen-go generated exports
func setupExports() {
	handler.Exports.ListTools = func() cm.List[handler.Tool] {
		tools := make([]handler.Tool, 0, len(globalHandler.tools))
		for _, tool := range globalHandler.tools {
			tools = append(tools, handler.Tool{
				Name:        tool.name,
				Description: tool.description,
				InputSchema: string(tool.schema),
			})
		}
		if len(tools) == 0 {
			return cm.NewList[handler.Tool](nil, 0)
		}
		return cm.NewList(&tools[0], len(tools))
	}

	handler.Exports.CallTool = func(name string, arguments string) handler.ToolResult {
		tool, ok := globalHandler.tools[name]
		if !ok {
			return handler.ToolResultError(handler.Error{
				Code:    -32601,
				Message: fmt.Sprintf("Tool not found: %s", name),
				Data:    cm.None[string](),
			})
		}

		result, err := tool.handler(json.RawMessage(arguments))
		if err != nil {
			return handler.ToolResultError(handler.Error{
				Code:    -32603,
				Message: err.Error(),
				Data:    cm.None[string](),
			})
		}

		return handler.ToolResultText(result)
	}

	handler.Exports.ListResources = func() cm.List[handler.ResourceInfo] {
		resources := make([]handler.ResourceInfo, 0, len(globalHandler.resources))
		for _, resource := range globalHandler.resources {
			desc := cm.None[string]()
			if resource.description != "" {
				desc = cm.Some(resource.description)
			}
			mime := cm.None[string]()
			if resource.mimeType != "" {
				mime = cm.Some(resource.mimeType)
			}
			
			resources = append(resources, handler.ResourceInfo{
				URI:         resource.uri,
				Name:        resource.name,
				Description: desc,
				MIMEType:    mime,
			})
		}
		if len(resources) == 0 {
			return cm.NewList[handler.ResourceInfo](nil, 0)
		}
		return cm.NewList(&resources[0], len(resources))
	}

	handler.Exports.ReadResource = func(uri string) handler.ResourceResult {
		resource, ok := globalHandler.resources[uri]
		if !ok {
			return handler.ResourceResultError(handler.Error{
				Code:    -32002,
				Message: fmt.Sprintf("Resource not found: %s", uri),
				Data:    cm.None[string](),
			})
		}

		content, err := resource.handler()
		if err != nil {
			return handler.ResourceResultError(handler.Error{
				Code:    -32603,
				Message: err.Error(),
				Data:    cm.None[string](),
			})
		}

		mime := cm.None[string]()
		if resource.mimeType != "" {
			mime = cm.Some(resource.mimeType)
		}

		return handler.ResourceResultContents(handler.ResourceContents{
			URI:      resource.uri,
			MIMEType: mime,
			Text:     cm.Some(content),
			Blob:     cm.None[cm.List[uint8]](),
		})
	}

	handler.Exports.ListPrompts = func() cm.List[handler.Prompt] {
		prompts := make([]handler.Prompt, 0, len(globalHandler.prompts))
		for _, prompt := range globalHandler.prompts {
			args := make([]handler.PromptArgument, len(prompt.arguments))
			for i, arg := range prompt.arguments {
				desc := cm.None[string]()
				if arg.Description != "" {
					desc = cm.Some(arg.Description)
				}
				
				args[i] = handler.PromptArgument{
					Name:        arg.Name,
					Description: desc,
					Required:    arg.Required,
				}
			}

			desc := cm.None[string]()
			if prompt.description != "" {
				desc = cm.Some(prompt.description)
			}

			argsPtr := (*handler.PromptArgument)(nil)
			if len(args) > 0 {
				argsPtr = &args[0]
			}
			
			prompts = append(prompts, handler.Prompt{
				Name:        prompt.name,
				Description: desc,
				Arguments:   cm.NewList(argsPtr, len(args)),
			})
		}
		if len(prompts) == 0 {
			return cm.NewList[handler.Prompt](nil, 0)
		}
		return cm.NewList(&prompts[0], len(prompts))
	}

	handler.Exports.GetPrompt = func(name string, arguments string) handler.PromptResult {
		prompt, ok := globalHandler.prompts[name]
		if !ok {
			return handler.PromptResultError(handler.Error{
				Code:    -32002,
				Message: fmt.Sprintf("Prompt not found: %s", name),
				Data:    cm.None[string](),
			})
		}

		messages, err := prompt.handler(json.RawMessage(arguments))
		if err != nil {
			return handler.PromptResultError(handler.Error{
				Code:    -32603,
				Message: err.Error(),
				Data:    cm.None[string](),
			})
		}

		mcpMessages := make([]handler.PromptMessage, len(messages))
		for i, msg := range messages {
			mcpMessages[i] = handler.PromptMessage{
				Role:    msg.Role,
				Content: msg.Content,
			}
		}

		msgsPtr := (*handler.PromptMessage)(nil)
		if len(mcpMessages) > 0 {
			msgsPtr = &mcpMessages[0]
		}
		return handler.PromptResultMessages(cm.NewList(msgsPtr, len(mcpMessages)))
	}
}

// Tool registers a tool with the handler.
//
// Parameters:
//   - name: The tool's unique identifier
//   - description: Human-readable description of what the tool does
//   - schema: JSON schema for the tool's input parameters (use json.RawMessage(`{...}`))
//   - fn: The function to execute when the tool is called
func (h *Handler) Tool(name, description string, schema json.RawMessage, fn ToolFunc) {
	h.tools[name] = &toolDef{
		name:        name,
		description: description,
		schema:      schema,
		handler:     fn,
	}
}

// Resource registers a resource with the handler.
//
// Parameters:
//   - uri: The resource's unique URI identifier
//   - name: Human-readable name for the resource
//   - description: Optional description (pass empty string if not needed)
//   - mimeType: Optional MIME type (pass empty string if not needed)
//   - fn: The function to execute when the resource is read
func (h *Handler) Resource(uri, name, description, mimeType string, fn ResourceFunc) {
	h.resources[uri] = &resourceDef{
		uri:         uri,
		name:        name,
		description: description,
		mimeType:    mimeType,
		handler:     fn,
	}
}

// Prompt registers a prompt with the handler.
//
// Parameters:
//   - name: The prompt's unique identifier
//   - description: Optional description (pass empty string if not needed)
//   - arguments: List of arguments the prompt accepts
//   - fn: The function to execute when the prompt is resolved
func (h *Handler) Prompt(name, description string, arguments []PromptArgument, fn PromptFunc) {
	h.prompts[name] = &promptDef{
		name:        name,
		description: description,
		arguments:   arguments,
		handler:     fn,
	}
}

// Schema creates a json.RawMessage from a JSON string.
// This is a helper function for defining tool schemas inline.
//
// Example:
//
//	mcp.Schema(`{
//	    "type": "object",
//	    "properties": {
//	        "message": {"type": "string"}
//	    }
//	}`)
func Schema(s string) json.RawMessage {
	return json.RawMessage(s)
}