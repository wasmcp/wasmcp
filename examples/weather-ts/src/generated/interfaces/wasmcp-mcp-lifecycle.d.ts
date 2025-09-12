/** @module Interface wasmcp:mcp/lifecycle@0.2.0-alpha.27 **/
/**
 * Handle session initialization
 * Implementations should declare their capabilities here
 */
export function initialize(request: InitializeRequest): InitializeResult;
/**
 * Handle initialization complete notification
 */
export function clientInitialized(): void;
/**
 * Handle shutdown request
 */
export function shutdown(): void;
export type McpError = import('./wasmcp-mcp-mcp-types.js').McpError;
export type InitializeRequest = import('./wasmcp-mcp-lifecycle-types.js').InitializeRequest;
export type InitializeResult = import('./wasmcp-mcp-lifecycle-types.js').InitializeResult;
