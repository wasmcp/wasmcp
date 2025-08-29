/** @module Interface fastertools:mcp/tools@0.1.8 **/
/**
 * Tool operations
 * List available tools with optional pagination
 */
export function listTools(request: ListToolsRequest): ListToolsResponse;
/**
 * Execute a tool with the provided arguments
 */
export function callTool(request: CallToolRequest): ToolResult;
export type ContentBlock = import('./fastertools-mcp-types.js').ContentBlock;
export type JsonValue = import('./fastertools-mcp-types.js').JsonValue;
export type JsonSchema = import('./fastertools-mcp-types.js').JsonSchema;
export type McpError = import('./fastertools-mcp-types.js').McpError;
export type BaseMetadata = import('./fastertools-mcp-types.js').BaseMetadata;
export type MetaFields = import('./fastertools-mcp-types.js').MetaFields;
export type Cursor = import('./fastertools-mcp-types.js').Cursor;
export type ProgressToken = import('./fastertools-mcp-types.js').ProgressToken;
/**
 * Behavioral hints about tool operations
 */
export interface ToolAnnotations {
  /**
   * Human-readable title for display
   */
  title?: string,
  /**
   * Tool does not modify environment
   */
  readOnlyHint?: boolean,
  /**
   * Tool may perform destructive updates (meaningful when not read-only)
   */
  destructiveHint?: boolean,
  /**
   * Repeated calls with same args have no additional effect
   */
  idempotentHint?: boolean,
  /**
   * Tool interacts with external entities
   */
  openWorldHint?: boolean,
}
/**
 * Tool definition with metadata and schema
 */
export interface Tool {
  /**
   * Base metadata (name and optional title)
   */
  base: BaseMetadata,
  /**
   * Human-readable description of what the tool does
   */
  description?: string,
  /**
   * JSON Schema for input parameters
   */
  inputSchema: JsonSchema,
  /**
   * Optional schema for structured output
   */
  outputSchema?: JsonSchema,
  /**
   * Behavioral hints for clients
   */
  annotations?: ToolAnnotations,
  /**
   * Extension metadata
   */
  meta?: MetaFields,
}
/**
 * Result from executing a tool
 */
export interface ToolResult {
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
   * If true, content should contain error details
   */
  isError?: boolean,
  /**
   * Extension metadata
   */
  meta?: MetaFields,
}
/**
 * Request to list available tools
 */
export interface ListToolsRequest {
  /**
   * Pagination cursor from previous response
   */
  cursor?: Cursor,
  /**
   * Optional progress tracking token
   */
  progressToken?: ProgressToken,
  /**
   * Extension metadata
   */
  meta?: MetaFields,
}
/**
 * Response with list of available tools
 */
export interface ListToolsResponse {
  /**
   * Available tools
   */
  tools: Array<Tool>,
  /**
   * Cursor for next page if more tools exist
   */
  nextCursor?: Cursor,
  /**
   * Extension metadata
   */
  meta?: MetaFields,
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
   * Arguments as JSON object
   */
  arguments?: JsonValue,
  /**
   * Optional progress tracking token
   */
  progressToken?: ProgressToken,
  /**
   * Extension metadata
   */
  meta?: MetaFields,
}
