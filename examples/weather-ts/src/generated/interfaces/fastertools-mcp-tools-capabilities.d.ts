/** @module Interface fastertools:mcp/tools-capabilities@0.4.1 **/
/**
 * List available tools
 */
export function handleListTools(request: ListToolsRequest): ListToolsResponse;
/**
 * Execute a tool
 */
export function handleCallTool(request: CallToolRequest): ToolResult;
export type McpError = import('./fastertools-mcp-types.js').McpError;
export type ListToolsRequest = import('./fastertools-mcp-tool-types.js').ListToolsRequest;
export type ListToolsResponse = import('./fastertools-mcp-tool-types.js').ListToolsResponse;
export type CallToolRequest = import('./fastertools-mcp-tool-types.js').CallToolRequest;
export type ToolResult = import('./fastertools-mcp-tool-types.js').ToolResult;
