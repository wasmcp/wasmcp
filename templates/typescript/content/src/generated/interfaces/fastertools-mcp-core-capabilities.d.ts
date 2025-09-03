/** @module Interface fastertools:mcp/core-capabilities@0.1.23 **/
/**
 * Handle session initialization
 * Implementations should declare their capabilities here
 */
export function handleInitialize(request: InitializeRequest): InitializeResponse;
/**
 * Handle initialization complete notification
 */
export function handleInitialized(): void;
/**
 * Handle ping request for keepalive
 */
export function handlePing(): void;
/**
 * Handle shutdown request
 */
export function handleShutdown(): void;
/**
 * Get provider's auth configuration (optional - return none for no auth)
 * If auth configuration is provided, the transport will enforce authentication
 */
export function getAuthConfig(): ProviderAuthConfig | undefined;
export type McpError = import('./fastertools-mcp-types.js').McpError;
export type InitializeRequest = import('./fastertools-mcp-session-types.js').InitializeRequest;
export type InitializeResponse = import('./fastertools-mcp-session-types.js').InitializeResponse;
export type ProviderAuthConfig = import('./fastertools-mcp-authorization-types.js').ProviderAuthConfig;
