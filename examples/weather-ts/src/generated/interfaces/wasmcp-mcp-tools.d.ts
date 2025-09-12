/** @module Interface wasmcp:mcp/tools@0.2.0-alpha.27 **/
/**
 * List available tools
 */
export function listTools(request: ListToolsRequest): ListToolsResult;
/**
 * Execute a tool
 */
export function callTool(request: CallToolRequest, context: AuthContext | undefined): CallToolResult;
export type McpError = import('./wasmcp-mcp-mcp-types.js').McpError;
export type AuthContext = import('./wasmcp-mcp-authorization-types.js').AuthContext;
export type ListToolsRequest = import('./wasmcp-mcp-tools-types.js').ListToolsRequest;
export type ListToolsResult = import('./wasmcp-mcp-tools-types.js').ListToolsResult;
export type CallToolRequest = import('./wasmcp-mcp-tools-types.js').CallToolRequest;
export type CallToolResult = import('./wasmcp-mcp-tools-types.js').CallToolResult;
