/** @module Interface wasmcp:mcp/tools-capabilities@0.1.0 **/
/**
 * List available tools
 */
export function handleListTools(request: ListToolsRequest): ListToolsResponse;
/**
 * Execute a tool
 */
export function handleCallTool(request: CallToolRequest): ToolResult;
export type McpError = import('./wasmcp-mcp-types.js').McpError;
export type ListToolsRequest = import('./wasmcp-mcp-tool-types.js').ListToolsRequest;
export type ListToolsResponse = import('./wasmcp-mcp-tool-types.js').ListToolsResponse;
export type CallToolRequest = import('./wasmcp-mcp-tool-types.js').CallToolRequest;
export type ToolResult = import('./wasmcp-mcp-tool-types.js').ToolResult;
