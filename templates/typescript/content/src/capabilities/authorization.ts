/**
 * Authorization capability implementation for {{project-name | kebab_case}} MCP server.
 * 
 * This module handles OAuth 2.0/JWT authorization configuration.
 * The transport layer enforces authorization based on the configuration
 * we provide here.
 */

import type { ProviderAuthConfig } from '../generated/interfaces/wasmcp-mcp-authorization-types.js';

/**
 * Get provider's auth configuration.
 * 
 * Returning undefined disables authorization. TypeScript's optional types
 * map naturally to WIT's option<T>, similar to Rust's Option<T> but
 * without the explicit Some() wrapper.
 */
export function getAuthConfig(): ProviderAuthConfig | undefined {
  // Return undefined to disable authorization for this example
  return undefined;
  
  // Uncomment and configure to enable OAuth authorization:
  // return {
  //   expectedIssuer: 'https://xxx.authkit.app',
  //   expectedAudiences: ['client_xxx'],
  //   jwksUri: 'https://xxx.authkit.app/oauth2/jwks',
  //   passJwt: false,
  //   expectedSubject: undefined,
  //   policy: undefined,
  //   policyData: undefined,
  // };
}

/**
 * Get cached JWKS for a given URI.
 * 
 * Optional caching interface - return undefined if not implemented.
 * Could use WASI-KV or other persistence mechanisms if available.
 */
export function jwksCacheGet(_jwksUri: string): string | undefined {
  // No caching for this example
  return undefined;
}

/**
 * Cache JWKS for a given URI.
 * 
 * Optional caching interface - no-op if not implemented.
 * The transport calls this after successfully fetching JWKS.
 */
export function jwksCacheSet(_jwksUri: string, _jwks: string): void {
  // No caching for this example
}