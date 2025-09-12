/** @module Interface wasmcp:mcp/tools-types@0.2.0 **/
export type ContentBlock = import('./wasmcp-mcp-mcp-types.js').ContentBlock;
export type JsonValue = import('./wasmcp-mcp-mcp-types.js').JsonValue;
export type JsonObject = import('./wasmcp-mcp-mcp-types.js').JsonObject;
export type Icon = import('./wasmcp-mcp-mcp-types.js').Icon;
export type AuthContext = import('./wasmcp-mcp-authorization-types.js').AuthContext;
/**
 * Behavioral hints about tool operations
 */
export interface ToolAnnotations {
  /**
   * Human-readable title for display
   */
  title?: string,
  /**
   * Tool does not modify environment (default: false)
   */
  readOnlyHint?: boolean,
  /**
   * Tool may perform destructive updates (default: true)
   */
  destructiveHint?: boolean,
  /**
   * Repeated calls with same args have no additional effect (default: false)
   */
  idempotentHint?: boolean,
  /**
   * Tool interacts with external entities (default: true)
   */
  openWorldHint?: boolean,
}
/**
 * Tool definition with metadata and schema
 */
export interface Tool {
  /**
   * The name of the tool
   */
  name: string,
  /**
   * A human-readable title for the tool
   */
  title?: string,
  /**
   * Human-readable description of what the tool does
   */
  description?: string,
  /**
   * JSON Schema object for input parameters
   */
  inputSchema: JsonObject,
  /**
   * Optional JSON Schema object for structured output
   */
  outputSchema?: JsonObject,
  /**
   * Behavioral hints for clients
   */
  annotations?: ToolAnnotations,
  /**
   * Optional list of icons for the tool
   */
  icons?: Array<Icon>,
}
/**
 * Request to execute a tool
 */
export interface CallToolRequest {
  /**
   * Name of the tool to execute
   */
  name: string,
  /**
   * Arguments as JSON object (must match the tool's input schema)
   */
  arguments?: JsonObject,
}
/**
 * Result from executing a tool
 */
export interface CallToolResult {
  /**
   * Unstructured content blocks (text, images, etc.)
   */
  content: Array<ContentBlock>,
  /**
   * Optional structured JSON output
   */
  structuredContent?: JsonValue,
  /**
   * Whether the tool execution resulted in an error
   */
  isError?: boolean,
  /**
   * Optional metadata
   */
  meta?: JsonObject,
}
/**
 * Request to list available tools
 */
export interface ListToolsRequest {
  /**
   * Pagination cursor from previous response
   */
  cursor?: string,
}
/**
 * Response with list of available tools
 */
export interface ListToolsResult {
  /**
   * Available tools
   */
  tools: Array<Tool>,
  /**
   * Cursor for next page if more tools exist
   */
  nextCursor?: string,
}
