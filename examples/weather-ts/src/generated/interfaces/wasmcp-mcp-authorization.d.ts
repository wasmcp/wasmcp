/** @module Interface wasmcp:mcp/authorization@0.2.0 **/
/**
 * Get provider's auth configuration
 * The transport should enforce authorization
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
export type ProviderAuthConfig = import('./wasmcp-mcp-authorization-types.js').ProviderAuthConfig;
