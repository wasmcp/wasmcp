/**
 * Strict type definitions for MCP (Model Context Protocol)
 */

// Base types
export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface Annotations {
  audience?: string[];
  priority?: number;
  lastModified?: string;
}

export interface BaseMetadata {
  name: string;
  title?: string;
}

export type MetaFields = Record<string, JsonValue>;

// Content types
export interface TextContent {
  text: string;
  annotations?: Annotations;
  meta?: MetaFields;
}

export interface ContentBlock {
  tag: 'text';
  val: TextContent;
}

// Tool-specific types
export interface ToolAnnotations {
  title?: string;
  readOnlyHint?: boolean;
  destructiveHint?: boolean;
  idempotentHint?: boolean;
  openWorldHint?: boolean;
}

export interface Tool {
  base: BaseMetadata;
  description?: string;
  inputSchema: string; // JSON Schema as string
  outputSchema?: string;
  annotations?: ToolAnnotations;
  meta?: MetaFields;
}

export interface ToolResult {
  content: ContentBlock[];
  structuredContent?: JsonValue;
  isError?: boolean;
  meta?: MetaFields;
}

// Request/Response types
export interface ListToolsRequest {
  cursor?: string;
  progressToken?: string;
  meta?: MetaFields;
}

export interface ListToolsResponse {
  tools: Tool[];
  nextCursor?: string;
  meta?: MetaFields;
}

export interface CallToolRequest {
  name: string;
  arguments?: JsonValue;
  progressToken?: string;
  meta?: MetaFields;
}

// Handler interface
export interface ToolsCapabilities {
  handleListTools: (request: ListToolsRequest) => ListToolsResponse;
  handleCallTool: (request: CallToolRequest) => Promise<ToolResult>;
}
