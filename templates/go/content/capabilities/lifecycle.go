package capabilities

import (
	"go.bytecodealliance.org/cm"
	lifecycle "weather_go/internal/wasmcp/mcp/lifecycle"
	lifecycletypes "weather_go/internal/wasmcp/mcp/lifecycle-types"
	mcptypes "weather_go/internal/wasmcp/mcp/mcp-types"
)

// Initialize handles session initialization.
//
// Technical Note: We return a cm.Result instead of Go's idiomatic (value, error)
// because WebAssembly core modules can only return a single value. The Result type
// is defined in WIT as result<T, E> and represents either success (T) or error (E).
// The InitializeResultShape is wit-bindgen-go's internal storage type for the result.
func Initialize(request lifecycletypes.InitializeRequest) cm.Result[lifecycle.InitializeResultShape, lifecycletypes.InitializeResult, mcptypes.McpError] {
	result := lifecycletypes.InitializeResult{
		ProtocolVersion: lifecycletypes.ProtocolVersion("0.1.0"),
		Capabilities: lifecycletypes.ServerCapabilities{
			Tools: cm.Some(lifecycletypes.ToolsCapability{
				ListChanged: cm.None[bool](),
			}),
			Experimental: cm.None[mcptypes.JSONObject](),
			Logging:      cm.None[mcptypes.JSONObject](),
			Completions:  cm.None[mcptypes.JSONObject](),
			Prompts:      cm.None[lifecycletypes.PromptsCapability](),
			Resources:    cm.None[lifecycletypes.ResourcesCapability](),
		},
		ServerInfo: lifecycletypes.Implementation{
			Name:       "weather-go",
			Version:    "0.1.0",
			Title:      cm.Some("Weather Go Provider"),
			WebsiteURL: cm.None[string](),
			Icons:      cm.None[cm.List[mcptypes.Icon]](),
		},
		Instructions: cm.Some("A Go MCP server providing weather tools"),
	}
	// In standard Go, this would be: return result, nil
	// The SetOK method sets the success variant of the Result type
	var res cm.Result[lifecycle.InitializeResultShape, lifecycletypes.InitializeResult, mcptypes.McpError]
	res.SetOK(result)
	return res
}

// ClientInitialized handles client initialization notification
func ClientInitialized() cm.Result[mcptypes.McpError, struct{}, mcptypes.McpError] {
	// Nothing to do on client initialization
	var result cm.Result[mcptypes.McpError, struct{}, mcptypes.McpError]
	result.SetOK(struct{}{})
	return result
}

// Shutdown handles shutdown request
func Shutdown() cm.Result[mcptypes.McpError, struct{}, mcptypes.McpError] {
	// Clean shutdown
	var result cm.Result[mcptypes.McpError, struct{}, mcptypes.McpError]
	result.SetOK(struct{}{})
	return result
}