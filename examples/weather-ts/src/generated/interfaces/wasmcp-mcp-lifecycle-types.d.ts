/** @module Interface wasmcp:mcp/lifecycle-types@0.2.0-alpha.27 **/
export type MetaFields = import('./wasmcp-mcp-mcp-types.js').MetaFields;
export type JsonObject = import('./wasmcp-mcp-mcp-types.js').JsonObject;
export type Icon = import('./wasmcp-mcp-mcp-types.js').Icon;
/**
 * Protocol version string
 * Common values: "2024-11-05", "2025-03-26", "2025-06-18"
 */
export type ProtocolVersion = string;
/**
 * Information about an MCP implementation
 */
export interface Implementation {
  /**
   * Implementation name (e.g., "weather-server")
   */
  name: string,
  /**
   * Optional human-readable title
   */
  title?: string,
  /**
   * Implementation version (e.g., "1.0.0")
   */
  version: string,
  /**
   * Optional website URL for more information
   */
  websiteUrl?: string,
  /**
   * Optional list of icons for the implementation
   */
  icons?: Array<Icon>,
}
/**
 * Root listing capability details
 */
export interface RootsCapability {
  /**
   * Server will notify when roots list changes
   */
  listChanged?: boolean,
}
/**
 * Prompts capability details
 */
export interface PromptsCapability {
  /**
   * Server will notify when prompts list changes
   */
  listChanged?: boolean,
}
/**
 * Resources capability details
 */
export interface ResourcesCapability {
  /**
   * Server supports resource subscriptions
   */
  subscribe?: boolean,
  /**
   * Server will notify when resource list changes
   */
  listChanged?: boolean,
}
/**
 * Tools capability details
 */
export interface ToolsCapability {
  /**
   * Server will notify when tools list changes
   */
  listChanged?: boolean,
}
/**
 * Elicitation capability details
 */
export interface ElicitationCapability {
  /**
   * Whether the client supports JSON Schema validation
   */
  schemaValidation?: boolean,
}
/**
 * Capabilities that a client supports
 */
export interface ClientCapabilities {
  /**
   * Experimental/custom capabilities as JSON objects
   */
  experimental?: JsonObject,
  /**
   * Support for roots (directory access)
   */
  roots?: RootsCapability,
  /**
   * Support for LLM sampling (empty object when enabled)
   */
  sampling?: JsonObject,
  /**
   * Support for user elicitation
   */
  elicitation?: ElicitationCapability,
}
/**
 * Capabilities that a server provides
 */
export interface ServerCapabilities {
  /**
   * Experimental/custom capabilities as JSON objects
   */
  experimental?: JsonObject,
  /**
   * Server can send log messages (empty object when enabled)
   */
  logging?: JsonObject,
  /**
   * Server supports argument autocompletion (empty object when enabled)
   */
  completions?: JsonObject,
  /**
   * Server offers prompts
   */
  prompts?: PromptsCapability,
  /**
   * Server offers resources
   */
  resources?: ResourcesCapability,
  /**
   * Server offers tools
   */
  tools?: ToolsCapability,
}
/**
 * Initialize request sent by client on connection
 */
export interface InitializeRequest {
  /**
   * Protocol version the client supports
   */
  protocolVersion: ProtocolVersion,
  /**
   * Client's capabilities
   */
  capabilities: ClientCapabilities,
  /**
   * Client implementation details
   */
  clientInfo: Implementation,
}
/**
 * Initialize response from server
 */
export interface InitializeResult {
  /**
   * Protocol version the server supports
   */
  protocolVersion: ProtocolVersion,
  /**
   * Server's capabilities
   */
  capabilities: ServerCapabilities,
  /**
   * Server implementation details
   */
  serverInfo: Implementation,
  /**
   * Optional instructions for using the server
   */
  instructions?: string,
}
