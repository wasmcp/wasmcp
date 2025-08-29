/** @module Interface fastertools:mcp/types@0.1.9 **/
export type ErrorCode = ErrorCodeParseError | ErrorCodeInvalidRequest | ErrorCodeMethodNotFound | ErrorCodeInvalidParams | ErrorCodeInternalError | ErrorCodeResourceNotFound | ErrorCodeToolNotFound | ErrorCodePromptNotFound | ErrorCodeUnauthorized | ErrorCodeRateLimited | ErrorCodeTimeout | ErrorCodeCancelled | ErrorCodeCustomCode;
export interface ErrorCodeParseError {
  tag: 'parse-error',
}
export interface ErrorCodeInvalidRequest {
  tag: 'invalid-request',
}
export interface ErrorCodeMethodNotFound {
  tag: 'method-not-found',
}
export interface ErrorCodeInvalidParams {
  tag: 'invalid-params',
}
export interface ErrorCodeInternalError {
  tag: 'internal-error',
}
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
export interface ErrorCodeCustomCode {
  tag: 'custom-code',
  val: number,
}
export interface McpError {
  code: ErrorCode,
  message: string,
  data?: string,
}
export type Cursor = string;
export type ProgressToken = string;
export type MetaFields = Array<[string, string]>;
export interface BaseMetadata {
  name: string,
  title?: string,
}
export type JsonSchema = string;
export type JsonValue = string;
/**
 * # Variants
 * 
 * ## `"user"`
 * 
 * ## `"assistant"`
 */
export type Role = 'user' | 'assistant';
export interface Annotations {
  audience?: Array<Role>,
  priority?: number,
  lastModified?: string,
}
export interface TextContent {
  text: string,
  annotations?: Annotations,
  meta?: MetaFields,
}
export interface ImageContent {
  data: Uint8Array,
  mimeType: string,
  annotations?: Annotations,
  meta?: MetaFields,
}
export interface AudioContent {
  data: Uint8Array,
  mimeType: string,
  annotations?: Annotations,
  meta?: MetaFields,
}
export interface ResourceLink {
  uri: string,
  name: string,
  title?: string,
  description?: string,
  mimeType?: string,
  size?: bigint,
  annotations?: Annotations,
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
  blob: Uint8Array,
  meta?: MetaFields,
}
export type ResourceContents = ResourceContentsText | ResourceContentsBlob;
export interface ResourceContentsText {
  tag: 'text',
  val: TextResourceContents,
}
export interface ResourceContentsBlob {
  tag: 'blob',
  val: BlobResourceContents,
}
export interface EmbeddedResource {
  contents: ResourceContents,
  annotations?: Annotations,
  meta?: MetaFields,
}
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
