/**
 * Lifecycle capability implementation for {{project-name | kebab_case}} MCP server.
 * 
 * This module handles MCP session lifecycle: initialization, client ready, and shutdown.
 * In TypeScript/JavaScript with jco, we export functions that match the WIT interface
 * signatures. Unlike Python classes or Rust traits, jco uses plain function exports.
 */

import type {
  InitializeRequest,
  InitializeResult,
  ServerCapabilities,
  Implementation,
  ToolsCapability,
} from '../generated/interfaces/wasmcp-mcp-lifecycle-types.js';

/**
 * Initialize the MCP server.
 * 
 * TypeScript with jco handles Result types transparently - we return the success
 * value directly, and errors are thrown as exceptions. This is similar to Python's
 * componentize-py approach but different from Rust's explicit Result<T, E>.
 */
export function initialize(_request: InitializeRequest): InitializeResult {
  // Declare our capabilities - we support tools
  const toolsCapability: ToolsCapability = {
    listChanged: undefined,
  };

  const capabilities: ServerCapabilities = {
    experimental: undefined,
    logging: undefined,
    completions: undefined,
    prompts: undefined,
    resources: undefined,
    tools: toolsCapability,
  };

  const serverInfo: Implementation = {
    name: '{{project-name | kebab_case}}',
    version: '0.1.0',
    title: '{{project-name | kebab_case}} Provider',
    icons: undefined,
    websiteUrl: undefined,
  };

  return {
    protocolVersion: '0.1.0',
    capabilities,
    serverInfo,
    instructions: '{{project-description}}',
  };
}

/**
 * Called when the client has initialized.
 */
export function clientInitialized(): void {
  // No-op for this example
}

/**
 * Shutdown the server.
 */
export function shutdown(): void {
  // No-op for this example
}