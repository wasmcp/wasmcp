/** @module Interface fastertools:mcp/tool-handler@0.1.8 **/
/**
 * List available tools
 */
export function handleListTools(request: ListToolsRequest): ListToolsResponse;
/**
 * Execute a tool
 */
export function handleCallTool(request: CallToolRequest): ToolResult;
export type McpError = import('./fastertools-mcp-types.js').McpError;
export type ListToolsRequest = import('./fastertools-mcp-tools.js').ListToolsRequest;
export type ListToolsResponse = import('./fastertools-mcp-tools.js').ListToolsResponse;
export type CallToolRequest = import('./fastertools-mcp-tools.js').CallToolRequest;
export type ToolResult = import('./fastertools-mcp-tools.js').ToolResult;
