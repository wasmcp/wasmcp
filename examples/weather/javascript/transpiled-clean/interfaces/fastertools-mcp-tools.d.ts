/** @module Interface fastertools:mcp/tools@0.1.9 **/
export type Cursor = import('./fastertools-mcp-types.js').Cursor;
export type ProgressToken = import('./fastertools-mcp-types.js').ProgressToken;
export type MetaFields = import('./fastertools-mcp-types.js').MetaFields;
export interface ListToolsRequest {
  cursor?: Cursor,
  progressToken?: ProgressToken,
  meta?: MetaFields,
}
export type BaseMetadata = import('./fastertools-mcp-types.js').BaseMetadata;
export type JsonSchema = import('./fastertools-mcp-types.js').JsonSchema;
export interface ToolAnnotations {
  title?: string,
  readOnlyHint?: boolean,
  destructiveHint?: boolean,
  idempotentHint?: boolean,
  openWorldHint?: boolean,
}
export interface Tool {
  base: BaseMetadata,
  description?: string,
  inputSchema: JsonSchema,
  outputSchema?: JsonSchema,
  annotations?: ToolAnnotations,
  meta?: MetaFields,
}
export interface ListToolsResponse {
  tools: Array<Tool>,
  nextCursor?: Cursor,
  meta?: MetaFields,
}
export type JsonValue = import('./fastertools-mcp-types.js').JsonValue;
export interface CallToolRequest {
  name: string,
  arguments?: JsonValue,
  progressToken?: ProgressToken,
  meta?: MetaFields,
}
export type ContentBlock = import('./fastertools-mcp-types.js').ContentBlock;
export interface ToolResult {
  content: Array<ContentBlock>,
  structuredContent?: JsonValue,
  isError?: boolean,
  meta?: MetaFields,
}
