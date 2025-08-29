/** @module Interface fastertools:mcp/types@0.1.8 **/
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
 * Metadata fields for extensibility
 * Key-value pairs where values are JSON-encoded strings
 */
export type MetaFields = Array<[string, string]>;
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
 * Base metadata pattern used throughout the protocol
 */
export interface BaseMetadata {
  /**
   * Programmatic identifier
   */
  name: string,
  /**
   * Human-readable display name
   */
  title?: string,
}
/**
 * Text content with optional annotations
 */
export interface TextContent {
  text: string,
  annotations?: Annotations,
  meta?: MetaFields,
}
/**
 * Image content as binary data
 */
export interface ImageContent {
  /**
   * Base64-encoded image data
   */
  data: Uint8Array,
  /**
   * MIME type (e.g., "image/png", "image/jpeg")
   */
  mimeType: string,
  annotations?: Annotations,
  meta?: MetaFields,
}
/**
 * Audio content as binary data
 */
export interface AudioContent {
  /**
   * Base64-encoded audio data
   */
  data: Uint8Array,
  /**
   * MIME type (e.g., "audio/wav", "audio/mp3")
   */
  mimeType: string,
  annotations?: Annotations,
  meta?: MetaFields,
}
/**
 * Reference to a resource that the server can read
 * Resource links included in prompts or tool results may not appear in resources/list
 */
export interface ResourceLink {
  /**
   * URI of the resource
   */
  uri: string,
  /**
   * Programmatic identifier for the resource
   */
  name: string,
  /**
   * Human-readable display title (preferred for UI display)
   */
  title?: string,
  /**
   * Description of what this resource represents
   */
  description?: string,
  /**
   * MIME type of the resource, if known
   */
  mimeType?: string,
  /**
   * Size in bytes (before encoding), if known
   */
  size?: bigint,
  /**
   * Client hints for handling
   */
  annotations?: Annotations,
  /**
   * Extension metadata
   */
  meta?: MetaFields,
}
export interface TextResourceContents {
  uri: string,
  mimeType?: string,
  text: string,
  meta?: MetaFields,
}
export interface BlobResourceContents {
  uri: string,
  mimeType?: string,
  /**
   * Binary data
   */
  blob: Uint8Array,
  meta?: MetaFields,
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
 * The contents of a resource, embedded into a prompt or tool call result
 */
export interface EmbeddedResource {
  /**
   * The actual resource contents (text or binary)
   */
  contents: ResourceContents,
  /**
   * Client hints for handling
   */
  annotations?: Annotations,
  /**
   * Extension metadata
   */
  meta?: MetaFields,
}
/**
 * Content block types that can be included in messages
 */
export type ContentBlock = ContentBlockText | ContentBlockImage | ContentBlockAudio | ContentBlockResourceLink | ContentBlockEmbeddedResource;
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
export interface ContentBlockResourceLink {
  tag: 'resource-link',
  val: ResourceLink,
}
export interface ContentBlockEmbeddedResource {
  tag: 'embedded-resource',
  val: EmbeddedResource,
}
/**
 * JSON Schema representation
 * Kept as a string since JSON Schema is complex
 * and typically validated by specialized libraries
 */
export type JsonSchema = string;
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
 * Pagination cursor for list operations
 */
export type Cursor = string;
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
/**
 * Resource template with URI template support (RFC 6570)
 */
export interface ResourceTemplate {
  /**
   * URI template that can be expanded with variables
   */
  uriTemplate: string,
  /**
   * Identifier for the template
   */
  name: string,
  /**
   * Human-readable description
   */
  description?: string,
  /**
   * Expected MIME type of resources
   */
  mimeType?: string,
}
