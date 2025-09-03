package main

import (
	"context"
	"encoding/json"
	"fmt"

	corecapabilities "weather_go/internal/fastertools/mcp/core-capabilities"
	sessiontypes "weather_go/internal/fastertools/mcp/session-types"
	"weather_go/internal/fastertools/mcp/tools"
	toolscapabilities "weather_go/internal/fastertools/mcp/tools-capabilities"
	"weather_go/internal/fastertools/mcp/types"

	"go.bytecodealliance.org/cm"
)

// Server wraps the MCP server functionality
type Server struct {
	implementation *Implementation
	tools          map[string]*registeredTool
}

// Implementation contains server metadata
type Implementation struct {
	Name    string
	Version string
}

// ServerOptions contains optional server configuration
type ServerOptions struct {
	Instructions string
}

// Tool represents an MCP tool definition
type Tool struct {
	Name        string
	Description string
	InputSchema Schema
}

// Schema represents a JSON schema as a string (for TinyGo compatibility)
type Schema string

// registeredTool stores a tool definition with its handler
type registeredTool struct {
	definition *Tool
	handler    ToolHandler
}

// ToolHandler is the function signature for tool handlers
type ToolHandler func(context.Context, *CallToolRequest) (*CallToolResult, error)

// CallToolRequest represents an incoming tool call
type CallToolRequest struct {
	Name      string
	Arguments string // JSON string
}

// CallToolResult represents the result of a tool call
type CallToolResult struct {
	Content []Content
	IsError bool
}

// Content represents output content
type Content interface {
	isContent()
}

// TextContent represents text output
type TextContent struct {
	Text string
}

func (TextContent) isContent() {}

// McpError represents an MCP protocol error
type McpError struct {
	Code    ErrorCode
	Message string
	Data    interface{}
}

func (e *McpError) Error() string {
	return fmt.Sprintf("MCP error %d: %s", e.Code, e.Message)
}

// ErrorCode represents MCP error codes
type ErrorCode int

const (
	ErrorCodeInvalidParams ErrorCode = -32602
	ErrorCodeToolNotFound  ErrorCode = -32601
	ErrorCodeInternal      ErrorCode = -32603
)

// NewServer creates a new MCP server
func NewServer(impl *Implementation, options *ServerOptions) *Server {
	return &Server{
		implementation: impl,
		tools:          make(map[string]*registeredTool),
	}
}

// AddTool registers a tool with an untyped handler
func (s *Server) AddTool(tool *Tool, handler ToolHandler) {
	s.tools[tool.Name] = &registeredTool{
		definition: tool,
		handler:    handler,
	}
}

// AddTool registers a tool with a typed handler
// This function provides type safety and automatic JSON unmarshaling
func AddTool[T any](s *Server, tool *Tool, handler func(context.Context, T) (*CallToolResult, error)) {
	s.tools[tool.Name] = &registeredTool{
		definition: tool,
		handler: func(ctx context.Context, req *CallToolRequest) (*CallToolResult, error) {
			var args T
			if req.Arguments != "" {
				if err := json.Unmarshal([]byte(req.Arguments), &args); err != nil {
					return ErrorResult(fmt.Sprintf("Invalid arguments: %v", err)), nil
				}
			}
			return handler(ctx, args)
		},
	}
}

// Run initializes the Wasm exports and starts the server
func (s *Server) Run(ctx context.Context, transport interface{}) {
	// Wire up the core capability exports
	corecapabilities.Exports.HandleInitialize = s.handleInitialize
	corecapabilities.Exports.HandleInitialized = s.handleInitialized
	corecapabilities.Exports.HandlePing = s.handlePing
	corecapabilities.Exports.HandleShutdown = s.handleShutdown
	// Note: GetAuthConfig is set in main.go
	
	// Wire up the tools capability exports
	toolscapabilities.Exports.HandleListTools = s.handleListTools
	toolscapabilities.Exports.HandleCallTool = s.handleCallTool
}

// handleListTools implements the list tools handler
func (s *Server) handleListTools(request tools.ListToolsRequest) cm.Result[toolscapabilities.ListToolsResponseShape, tools.ListToolsResponse, types.McpError] {
	toolsList := make([]tools.Tool, 0, len(s.tools))

	for _, rt := range s.tools {
		toolsList = append(toolsList, tools.Tool{
			Base: types.BaseMetadata{
				Name:  rt.definition.Name,
				Title: cm.Some(rt.definition.Name),
			},
			Description:  cm.Some(rt.definition.Description),
			InputSchema:  types.JSONSchema(rt.definition.InputSchema),
			OutputSchema: cm.None[types.JSONSchema](),
			Annotations:  cm.None[tools.ToolAnnotations](),
			Meta:         cm.None[types.MetaFields](),
		})
	}

	response := tools.ListToolsResponse{
		Tools:      cm.ToList(toolsList),
		NextCursor: cm.None[types.Cursor](),
		Meta:       cm.None[types.MetaFields](),
	}

	return cm.OK[cm.Result[toolscapabilities.ListToolsResponseShape, tools.ListToolsResponse, types.McpError]](response)
}

// handleCallTool implements the call tool handler
func (s *Server) handleCallTool(request tools.CallToolRequest) cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError] {
	rt, exists := s.tools[request.Name]
	if !exists {
		return mcpErrorResult(ErrorCodeToolNotFound, fmt.Sprintf("Unknown tool: %s", request.Name))
	}

	// Convert request to our internal format
	var arguments string
	if request.Arguments.Some() != nil {
		arguments = string(*request.Arguments.Some())
	}
	callReq := &CallToolRequest{
		Name:      request.Name,
		Arguments: arguments,
	}

	// Call the handler
	result, err := rt.handler(context.Background(), callReq)
	if err != nil {
		// Check if it's an MCP error
		if mcpErr, ok := err.(*McpError); ok {
			return mcpErrorResult(mcpErr.Code, mcpErr.Message)
		}
		// Otherwise treat as internal error
		return mcpErrorResult(ErrorCodeInternal, err.Error())
	}

	// Convert our result to WIT types
	return convertToWitResult(result)
}

// Helper functions for creating results

// TextResult creates a successful text result
func TextResult(text string) *CallToolResult {
	return &CallToolResult{
		Content: []Content{&TextContent{Text: text}},
		IsError: false,
	}
}

// ErrorResult creates an error result (visible to LLM)
func ErrorResult(message string) *CallToolResult {
	return &CallToolResult{
		Content: []Content{&TextContent{Text: message}},
		IsError: true,
	}
}

// MultiTextResult creates a result with multiple text blocks
func MultiTextResult(texts ...string) *CallToolResult {
	content := make([]Content, len(texts))
	for i, text := range texts {
		content[i] = &TextContent{Text: text}
	}
	return &CallToolResult{
		Content: content,
		IsError: false,
	}
}

// Internal helper functions

func convertToWitResult(result *CallToolResult) cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError] {
	contentBlocks := make([]types.ContentBlock, len(result.Content))

	for i, c := range result.Content {
		if tc, ok := c.(*TextContent); ok {
			contentBlocks[i] = types.ContentBlockText(types.TextContent{
				Text:        tc.Text,
				Annotations: cm.None[types.Annotations](),
				Meta:        cm.None[types.MetaFields](),
			})
		}
	}

	witResult := tools.ToolResult{
		Content:           cm.ToList(contentBlocks),
		StructuredContent: cm.None[types.JSONValue](),
		IsError:           cm.Some(result.IsError),
		Meta:              cm.None[types.MetaFields](),
	}

	return cm.OK[cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError]](witResult)
}

func mcpErrorResult(code ErrorCode, message string) cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError] {
	var witCode types.ErrorCode
	switch code {
	case ErrorCodeInvalidParams:
		witCode = types.ErrorCodeInvalidParams()
	case ErrorCodeToolNotFound:
		witCode = types.ErrorCodeToolNotFound()
	default:
		witCode = types.ErrorCodeInternalError()
	}

	return cm.Err[cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError]](types.McpError{
		Code:    witCode,
		Message: message,
		Data:    cm.None[string](),
	})
}

// Core capability handlers

// handleInitialize handles the initialize request
func (s *Server) handleInitialize(request sessiontypes.InitializeRequest) cm.Result[corecapabilities.InitializeResponseShape, sessiontypes.InitializeResponse, types.McpError] {
	response := sessiontypes.InitializeResponse{
		ProtocolVersion: sessiontypes.ProtocolVersionV20250618,
		Capabilities: sessiontypes.ServerCapabilities{
			Experimental: cm.None[types.MetaFields](),
			Logging:      cm.None[bool](),
			Completions:  cm.None[bool](),
			Prompts:      cm.None[sessiontypes.PromptsCapability](),
			Resources:    cm.None[sessiontypes.ResourcesCapability](),
			Tools:        cm.Some(sessiontypes.ToolsCapability{}),
		},
		ServerInfo: sessiontypes.ImplementationInfo{
			Name:    s.implementation.Name,
			Version: s.implementation.Version,
			Title:   cm.None[string](),
		},
		Instructions: cm.None[string](),
		Meta:         cm.None[types.MetaFields](),
	}
	
	return cm.OK[cm.Result[corecapabilities.InitializeResponseShape, sessiontypes.InitializeResponse, types.McpError]](response)
}

// handleInitialized handles the initialized notification
func (s *Server) handleInitialized() cm.Result[types.McpError, struct{}, types.McpError] {
	return cm.OK[cm.Result[types.McpError, struct{}, types.McpError]](struct{}{})
}

// handlePing handles the ping request
func (s *Server) handlePing() cm.Result[types.McpError, struct{}, types.McpError] {
	return cm.OK[cm.Result[types.McpError, struct{}, types.McpError]](struct{}{})
}

// handleShutdown handles the shutdown request
func (s *Server) handleShutdown() cm.Result[types.McpError, struct{}, types.McpError] {
	return cm.OK[cm.Result[types.McpError, struct{}, types.McpError]](struct{}{})
}
