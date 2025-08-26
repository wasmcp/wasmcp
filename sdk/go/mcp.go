// Package mcp provides the Go SDK for building MCP (Model Context Protocol) handlers
// in WebAssembly components using pure wasip2.
package mcp

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/fastertools/wasmcp/src/sdk/go/wasmcp/mcp/handler"
	"go.bytecodealliance.org/cm"
	
	// Enable WASI HTTP support for standard net/http package
	_ "github.com/ydnar/wasi-http-go/wasihttp"
)

// Server represents an MCP server instance.
// In WASM components, there's only one global server.
type Server struct {
	tools     map[string]*toolDef
	resources map[string]*resourceDef
	prompts   map[string]*promptDef
}

// Tool represents an MCP tool definition.
type Tool struct {
	Name        string          `json:"name"`
	Description string          `json:"description"`
	InputSchema json.RawMessage `json:"inputSchema,omitempty"`
}

// Resource represents an MCP resource definition.
type Resource struct {
	URI         string `json:"uri"`
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	MimeType    string `json:"mimeType,omitempty"`
}

// Prompt represents an MCP prompt definition.
type Prompt struct {
	Name        string           `json:"name"`
	Description string           `json:"description,omitempty"`
	Arguments   []PromptArgument `json:"arguments,omitempty"`
}

// PromptArgument defines an argument for a prompt.
type PromptArgument struct {
	Name        string `json:"name"`
	Description string `json:"description,omitempty"`
	Required    bool   `json:"required"`
}

// CallToolResult represents the result of calling a tool.
type CallToolResult struct {
	Content []Content `json:"content"`
	IsError bool      `json:"isError,omitempty"`
}

// Content represents content in a tool result.
type Content interface {
	isContent()
}

// TextContent represents text content.
type TextContent struct {
	Text string `json:"text"`
}

func (*TextContent) isContent() {}

// PromptMessage represents a message in a prompt conversation.
type PromptMessage struct {
	Role    string `json:"role"`    // "user" or "assistant"
	Content string `json:"content"`
}

// Implementation defines the server implementation details.
type Implementation struct {
	Name    string `json:"name"`
	Version string `json:"version"`
}

// toolDef holds the definition and handler for a tool.
type toolDef struct {
	tool    *Tool
	handler any // Can be various function signatures
}

// resourceDef holds the definition and handler for a resource.
type resourceDef struct {
	resource *Resource
	handler  any
}

// promptDef holds the definition and handler for a prompt.
type promptDef struct {
	prompt  *Prompt
	handler any
}

// The global server instance
var globalServer = &Server{
	tools:     make(map[string]*toolDef),
	resources: make(map[string]*resourceDef),
	prompts:   make(map[string]*promptDef),
}

// NewServer creates a new MCP server.
// In WASM components, this returns the global server instance.
func NewServer(impl *Implementation, options any) *Server {
	// In WASM, we ignore impl and options as they're handled by the gateway
	return globalServer
}

// AddTool adds a tool to the server with a generic typed handler.
// The handler is automatically wrapped to unmarshal JSON arguments into the In type.
//
// The handler should have the signature:
//   func(ctx context.Context, args In) (*CallToolResult, error)
//
// For tools that return simple text, you can return a CallToolResult with TextContent.
// The In type should match your tool's input schema.
//
// Since TinyGo has limited reflection, you must manually provide the input schema.
// Use the Schema() helper to create schemas from JSON strings.
func AddTool[In any](server *Server, tool *Tool, handler func(context.Context, In) (*CallToolResult, error)) {
	wrappedHandler := func(ctx context.Context, raw json.RawMessage) (*CallToolResult, error) {
		var args In
		if raw != nil && len(raw) > 0 && string(raw) != "{}" {
			if err := json.Unmarshal(raw, &args); err != nil {
				return nil, fmt.Errorf("invalid arguments: %w", err)
			}
		}
		return handler(ctx, args)
	}
	
	server.tools[tool.Name] = &toolDef{
		tool:    tool,
		handler: wrappedHandler,
	}
}

// AddResource adds a resource to the server.
// Resources are read-only and return text or binary content.
func (s *Server) AddResource(resource *Resource, handler func(context.Context) (string, error)) {
	s.resources[resource.URI] = &resourceDef{
		resource: resource,
		handler:  handler,
	}
}

// AddPrompt adds a prompt to the server with a generic typed handler.
// The handler is automatically wrapped to unmarshal JSON arguments into the In type.
func AddPrompt[In any](server *Server, prompt *Prompt, handler func(context.Context, In) ([]PromptMessage, error)) {
	wrappedHandler := func(ctx context.Context, raw json.RawMessage) ([]PromptMessage, error) {
		var args In
		if raw != nil && len(raw) > 0 && string(raw) != "{}" {
			if err := json.Unmarshal(raw, &args); err != nil {
				return nil, fmt.Errorf("invalid arguments: %w", err)
			}
		}
		return handler(ctx, args)
	}
	
	server.prompts[prompt.Name] = &promptDef{
		prompt:  prompt,
		handler: wrappedHandler,
	}
}

// Run initializes the WASM exports. Call this in init().
func (s *Server) Run(ctx context.Context, transport any) error {
	setupExports()
	return nil
}


// callTypedHandler invokes handlers with the appropriate signature
func callTypedHandler(handler any, ctx context.Context, arguments string) (any, error) {
	// For tools - handlers that work with json.RawMessage and return CallToolResult
	if fn, ok := handler.(func(context.Context, json.RawMessage) (*CallToolResult, error)); ok {
		return fn(ctx, json.RawMessage(arguments))
	}
	
	// For resources - handlers that return string
	if fn, ok := handler.(func(context.Context) (string, error)); ok {
		return fn(ctx)
	}
	
	// For resources - handlers that return []byte
	if fn, ok := handler.(func(context.Context) ([]byte, error)); ok {
		return fn(ctx)
	}
	
	// For prompts - handlers that work with json.RawMessage and return []PromptMessage
	if fn, ok := handler.(func(context.Context, json.RawMessage) ([]PromptMessage, error)); ok {
		return fn(ctx, json.RawMessage(arguments))
	}
	
	return nil, fmt.Errorf("unsupported handler signature")
}

// setupExports sets up the wit-bindgen-go generated exports
func setupExports() {
	handler.Exports.ListTools = func() cm.List[handler.Tool] {
		tools := make([]handler.Tool, 0, len(globalServer.tools))
		for _, tool := range globalServer.tools {
			inputSchema := "{}"
			if tool.tool.InputSchema != nil {
				inputSchema = string(tool.tool.InputSchema)
			}
			tools = append(tools, handler.Tool{
				Name:        tool.tool.Name,
				Description: tool.tool.Description,
				InputSchema: inputSchema,
			})
		}
		if len(tools) == 0 {
			return cm.NewList[handler.Tool](nil, 0)
		}
		return cm.NewList(&tools[0], len(tools))
	}

	handler.Exports.CallTool = func(name string, arguments string) handler.ToolResult {
		tool, ok := globalServer.tools[name]
		if !ok {
			return handler.ToolResultError(handler.Error{
				Code:    -32601,
				Message: fmt.Sprintf("Tool not found: %s", name),
				Data:    cm.None[string](),
			})
		}

		ctx := context.Background()
		
		// Use reflection to call the handler
		result, err := callTypedHandler(tool.handler, ctx, arguments)
		if err != nil {
			return handler.ToolResultError(handler.Error{
				Code:    -32603,
				Message: err.Error(),
				Data:    cm.None[string](),
			})
		}
		
		// Convert result to text based on type
		switch v := result.(type) {
		case string:
			return handler.ToolResultText(v)
		case *CallToolResult:
			if v != nil && len(v.Content) > 0 {
				if textContent, ok := v.Content[0].(*TextContent); ok {
					return handler.ToolResultText(textContent.Text)
				}
			}
			return handler.ToolResultText("")
		default:
			return handler.ToolResultError(handler.Error{
				Code:    -32603,
				Message: fmt.Sprintf("Unsupported return type: %T", result),
				Data:    cm.None[string](),
			})
		}
	}

	handler.Exports.ListResources = func() cm.List[handler.ResourceInfo] {
		resources := make([]handler.ResourceInfo, 0, len(globalServer.resources))
		for _, resource := range globalServer.resources {
			desc := cm.None[string]()
			if resource.resource.Description != "" {
				desc = cm.Some(resource.resource.Description)
			}
			mime := cm.None[string]()
			if resource.resource.MimeType != "" {
				mime = cm.Some(resource.resource.MimeType)
			}
			
			resources = append(resources, handler.ResourceInfo{
				URI:         resource.resource.URI,
				Name:        resource.resource.Name,
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
		resource, ok := globalServer.resources[uri]
		if !ok {
			return handler.ResourceResultError(handler.Error{
				Code:    -32002,
				Message: fmt.Sprintf("Resource not found: %s", uri),
				Data:    cm.None[string](),
			})
		}

		ctx := context.Background()
		
		// Use reflection to call the handler
		result, err := callTypedHandler(resource.handler, ctx, "")
		if err != nil {
			return handler.ResourceResultError(handler.Error{
				Code:    -32603,
				Message: err.Error(),
				Data:    cm.None[string](),
			})
		}
		
		mime := cm.None[string]()
		if resource.resource.MimeType != "" {
			mime = cm.Some(resource.resource.MimeType)
		}

		// Handle different return types
		switch v := result.(type) {
		case string:
			return handler.ResourceResultContents(handler.ResourceContents{
				URI:      resource.resource.URI,
				MIMEType: mime,
				Text:     cm.Some(v),
				Blob:     cm.None[cm.List[uint8]](),
			})
		case []byte:
			blobList := cm.NewList(&v[0], len(v))
			return handler.ResourceResultContents(handler.ResourceContents{
				URI:      resource.resource.URI,
				MIMEType: mime,
				Text:     cm.None[string](),
				Blob:     cm.Some(blobList),
			})
		default:
			return handler.ResourceResultError(handler.Error{
				Code:    -32603,
				Message: fmt.Sprintf("Unsupported return type: %T", result),
				Data:    cm.None[string](),
			})
		}
	}

	handler.Exports.ListPrompts = func() cm.List[handler.Prompt] {
		prompts := make([]handler.Prompt, 0, len(globalServer.prompts))
		for _, prompt := range globalServer.prompts {
			args := make([]handler.PromptArgument, len(prompt.prompt.Arguments))
			for i, arg := range prompt.prompt.Arguments {
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
			if prompt.prompt.Description != "" {
				desc = cm.Some(prompt.prompt.Description)
			}

			argsPtr := (*handler.PromptArgument)(nil)
			if len(args) > 0 {
				argsPtr = &args[0]
			}
			
			prompts = append(prompts, handler.Prompt{
				Name:        prompt.prompt.Name,
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
		prompt, ok := globalServer.prompts[name]
		if !ok {
			return handler.PromptResultError(handler.Error{
				Code:    -32002,
				Message: fmt.Sprintf("Prompt not found: %s", name),
				Data:    cm.None[string](),
			})
		}

		ctx := context.Background()
		
		// Use reflection to call the handler
		result, err := callTypedHandler(prompt.handler, ctx, arguments)
		if err != nil {
			return handler.PromptResultError(handler.Error{
				Code:    -32603,
				Message: err.Error(),
				Data:    cm.None[string](),
			})
		}
		
		// Convert result to prompt messages
		messages, ok := result.([]PromptMessage)
		if !ok {
			return handler.PromptResultError(handler.Error{
				Code:    -32603,
				Message: fmt.Sprintf("Handler must return []PromptMessage, got %T", result),
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

// Schema creates a json.RawMessage from a JSON string.
func Schema(s string) json.RawMessage {
	return json.RawMessage(s)
}

// ObjectSchema creates a JSON schema for an object with properties.
func ObjectSchema(properties map[string]any) json.RawMessage {
	schema := map[string]any{
		"type":       "object",
		"properties": properties,
	}
	b, _ := json.Marshal(schema)
	return b
}

// RequiredObjectSchema creates a JSON schema for an object with required properties.
func RequiredObjectSchema(properties map[string]any, required []string) json.RawMessage {
	schema := map[string]any{
		"type":       "object",
		"properties": properties,
		"required":   required,
	}
	b, _ := json.Marshal(schema)
	return b
}