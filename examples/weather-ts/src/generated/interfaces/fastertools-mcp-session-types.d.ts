/** @module Interface fastertools:mcp/session-types@0.1.23 **/
export type MetaFields = import('./fastertools-mcp-types.js').MetaFields;
/**
 * Protocol versions supported by MCP
 * These correspond to official MCP specification versions
 * # Variants
 * 
 * ## `"v20250326"`
 * 
 * MCP 2025-03-26 specification
 * ## `"v20250618"`
 * 
 * MCP 2025-06-18 specification (latest)
 */
export type ProtocolVersion = 'v20250326' | 'v20250618';
/**
 * Information about an MCP implementation
 */
export interface ImplementationInfo {
  /**
   * Implementation name (e.g., "weather-server")
   */
  name: string,
  /**
   * Implementation version (e.g., "1.0.0")
   */
  version: string,
  /**
   * Optional human-readable title
   */
  title?: string,
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
 * Capabilities that a client supports
 */
export interface ClientCapabilities {
  /**
   * Experimental/custom capabilities
   */
  experimental?: MetaFields,
  /**
   * Support for roots (directory access)
   */
  roots?: RootsCapability,
  /**
   * Support for LLM sampling
   */
  sampling?: boolean,
  /**
   * Support for user elicitation
   */
  elicitation?: boolean,
}
/**
 * Capabilities that a server provides
 */
export interface ServerCapabilities {
  /**
   * Experimental/custom capabilities
   */
  experimental?: MetaFields,
  /**
   * Server can send log messages
   */
  logging?: boolean,
  /**
   * Server supports argument autocompletion
   */
  completions?: boolean,
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
  clientInfo: ImplementationInfo,
  /**
   * Optional metadata
   */
  meta?: MetaFields,
}
/**
 * Initialize response from server
 */
export interface InitializeResponse {
  /**
   * Protocol version the server will use
   */
  protocolVersion: ProtocolVersion,
  /**
   * Server's capabilities
   */
  capabilities: ServerCapabilities,
  /**
   * Server implementation details
   */
  serverInfo: ImplementationInfo,
  /**
   * Optional instructions for using the server
   */
  instructions?: string,
  /**
   * Optional metadata
   */
  meta?: MetaFields,
}
