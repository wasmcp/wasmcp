/** @module Interface wasmcp:mcp/mcp-types@0.2.0-alpha.27 **/
/**
 * Role in a conversation
 * # Variants
 * 
 * ## `"user"`
 * 
 * ## `"assistant"`
 */
export type Role = 'user' | 'assistant';
/**
 * JSON value as a string
 * This is a JSON-encoded value that should be parsed/validated by implementations
 */
export type JsonValue = string;
/**
 * JSON object as a string
 * This is a JSON-encoded object that should be parsed/validated by implementations
 */
export type JsonObject = string;
/**
 * JSON Schema as a string
 * This is a JSON Schema document that defines structure and validation rules
 */
export type JsonSchema = string;
/**
 * Metadata fields for extensibility
 * Key-value pairs where values are JSON-encoded strings
 */
export type MetaFields = Array<[string, string]>;
/**
 * Icon specification for visual representation
 */
export interface Icon {
  /**
   * A standard URI pointing to an icon resource
   */
  src: string,
  /**
   * Optional override if the server's MIME type is missing or generic
   */
  mimeType?: string,
  /**
   * Size specification (e.g., "48x48", "any" for SVG, or "48x48 96x96")
   */
  sizes?: string,
}
/**
 * Annotations provide hints to clients about how to handle data
 */
export interface Annotations {
  /**
   * Who this data is intended for
   */
  audience?: Array<Role>,
  /**
   * Priority from 0.0 (least) to 1.0 (most important)
   * Implementations SHOULD validate this is within [0.0, 1.0] range
   */
  priority?: number,
  /**
   * ISO 8601 timestamp of last modification
   * Format: YYYY-MM-DDTHH:mm:ss[.sss]Z or ±HH:MM offset
   */
  lastModified?: string,
}
/**
 * Text content with optional annotations
 */
export interface TextContent {
  text: string,
  /**
   * Optional protocol-level metadata for this content block
   */
  meta?: JsonObject,
  annotations?: Annotations,
}
/**
 * Image content as base64-encoded string
 */
export interface ImageContent {
  /**
   * Base64-encoded image data
   */
  data: string,
  /**
   * MIME type (e.g., "image/png", "image/jpeg")
   */
  mimeType: string,
  /**
   * Optional protocol-level metadata for this content block
   */
  meta?: JsonObject,
  annotations?: Annotations,
}
/**
 * Audio content as base64-encoded string
 */
export interface AudioContent {
  /**
   * Base64-encoded audio data
   */
  data: string,
  /**
   * MIME type (e.g., "audio/wav", "audio/mp3")
   */
  mimeType: string,
  annotations?: Annotations,
}
/**
 * Raw resource representation (for resource links)
 */
export interface RawResource {
  /**
   * URI representing the resource location
   */
  uri: string,
  /**
   * Name of the resource
   */
  name: string,
  /**
   * Human-readable title of the resource
   */
  title?: string,
  /**
   * Optional description of the resource
   */
  description?: string,
  /**
   * MIME type of the resource content
   */
  mimeType?: string,
  /**
   * Size in bytes (before encoding), if known
   */
  size?: number,
  /**
   * Optional list of icons for the resource
   */
  icons?: Array<Icon>,
}
export interface TextResourceContents {
  uri: string,
  mimeType?: string,
  text: string,
  meta?: JsonObject,
}
export interface BlobResourceContents {
  uri: string,
  mimeType?: string,
  /**
   * Base64-encoded binary data
   */
  blob: string,
  meta?: JsonObject,
}
/**
 * Resource contents can be either text or binary
 */
export type ResourceContents = ResourceContentsText | ResourceContentsBlob;
export interface ResourceContentsText {
  tag: 'text',
  val: TextResourceContents,
}
export interface ResourceContentsBlob {
  tag: 'blob',
  val: BlobResourceContents,
}
/**
 * Embedded resource content
 */
export interface EmbeddedResource {
  /**
   * Optional protocol-level metadata for this content block
   */
  meta?: JsonObject,
  /**
   * The actual resource contents
   */
  resource: ResourceContents,
  annotations?: Annotations,
}
/**
 * Content block types that can be included in messages
 */
export type ContentBlock = ContentBlockText | ContentBlockImage | ContentBlockAudio | ContentBlockResource | ContentBlockResourceLink;
export interface ContentBlockText {
  tag: 'text',
  val: TextContent,
}
export interface ContentBlockImage {
  tag: 'image',
  val: ImageContent,
}
export interface ContentBlockAudio {
  tag: 'audio',
  val: AudioContent,
}
export interface ContentBlockResource {
  tag: 'resource',
  val: EmbeddedResource,
}
export interface ContentBlockResourceLink {
  tag: 'resource-link',
  val: RawResource,
}
/**
 * Standard JSON-RPC and MCP error codes
 */
export type ErrorCode = ErrorCodeParseError | ErrorCodeInvalidRequest | ErrorCodeMethodNotFound | ErrorCodeInvalidParams | ErrorCodeInternalError | ErrorCodeResourceNotFound | ErrorCodeToolNotFound | ErrorCodePromptNotFound | ErrorCodeUnauthorized | ErrorCodeRateLimited | ErrorCodeTimeout | ErrorCodeCancelled | ErrorCodeCustomCode;
/**
 * JSON-RPC standard errors
 */
export interface ErrorCodeParseError {
  tag: 'parse-error',
}
/**
 * -32700
 */
export interface ErrorCodeInvalidRequest {
  tag: 'invalid-request',
}
/**
 * -32600
 */
export interface ErrorCodeMethodNotFound {
  tag: 'method-not-found',
}
/**
 * -32601
 */
export interface ErrorCodeInvalidParams {
  tag: 'invalid-params',
}
/**
 * -32602
 */
export interface ErrorCodeInternalError {
  tag: 'internal-error',
}
/**
 * -32603
 * MCP-specific errors
 */
export interface ErrorCodeResourceNotFound {
  tag: 'resource-not-found',
}
export interface ErrorCodeToolNotFound {
  tag: 'tool-not-found',
}
export interface ErrorCodePromptNotFound {
  tag: 'prompt-not-found',
}
export interface ErrorCodeUnauthorized {
  tag: 'unauthorized',
}
export interface ErrorCodeRateLimited {
  tag: 'rate-limited',
}
export interface ErrorCodeTimeout {
  tag: 'timeout',
}
export interface ErrorCodeCancelled {
  tag: 'cancelled',
}
/**
 * Custom error with specific code
 */
export interface ErrorCodeCustomCode {
  tag: 'custom-code',
  val: number,
}
/**
 * Standard error structure
 */
export interface McpError {
  code: ErrorCode,
  message: string,
  /**
   * Additional error context (JSON-encoded)
   */
  data?: string,
}
/**
 * Progress token for tracking long-running operations
 */
export type ProgressToken = string;
/**
 * JSON-RPC request ID
 * Can be either a string or number in JSON-RPC
 */
export type RequestId = RequestIdStr | RequestIdNum;
export interface RequestIdStr {
  tag: 'str',
  val: string,
}
export interface RequestIdNum {
  tag: 'num',
  val: bigint,
}
/**
 * Message role for LLM interactions
 * # Variants
 * 
 * ## `"user"`
 * 
 * ## `"assistant"`
 * 
 * ## `"system"`
 */
export type MessageRole = 'user' | 'assistant' | 'system';
/**
 * Hint for model selection
 */
export interface ModelHint {
  /**
   * Name pattern to match (e.g., "claude", "gpt-4")
   */
  name?: string,
}
/**
 * Model selection preferences for LLM sampling
 */
export interface ModelPreferences {
  /**
   * Hints for model selection
   */
  hints?: Array<ModelHint>,
  /**
   * Priority for cost optimization (0.0-1.0)
   */
  costPriority?: number,
  /**
   * Priority for speed (0.0-1.0)
   */
  speedPriority?: number,
  /**
   * Priority for intelligence/capability (0.0-1.0)
   */
  intelligencePriority?: number,
}
