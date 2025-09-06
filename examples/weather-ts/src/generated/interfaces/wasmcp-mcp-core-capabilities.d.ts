/** @module Interface wasmcp:mcp/core-capabilities@0.1.0 **/
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
 * If auth configuration is provided, the transport will enforce authorization
 */
export function getAuthConfig(): ProviderAuthConfig | undefined;
/**
 * Get cached JWKS for a given URI (optional - return none if not cached or not implemented)
 * Allows providers to implement JWKS caching via WASI-KV or other persistence mechanisms
 * The transport will call this before fetching from jwks-uri to check for cached keys
 */
export function jwksCacheGet(jwksUri: string): string | undefined;
/**
 * Cache JWKS for a given URI (optional - no-op if caching not implemented)
 * The transport calls this after successfully fetching JWKS from jwks-uri
 * Providers can implement caching via WASI-KV or other persistence mechanisms
 * The jwks parameter contains the raw JWKS JSON string to cache
 */
export function jwksCacheSet(jwksUri: string, jwks: string): void;
export type McpError = import('./wasmcp-mcp-types.js').McpError;
export type InitializeRequest = import('./wasmcp-mcp-core-types.js').InitializeRequest;
export type InitializeResponse = import('./wasmcp-mcp-core-types.js').InitializeResponse;
export type ProviderAuthConfig = import('./wasmcp-mcp-authorization-types.js').ProviderAuthConfig;
