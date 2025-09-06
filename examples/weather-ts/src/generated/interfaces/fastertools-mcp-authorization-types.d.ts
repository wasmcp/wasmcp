/** @module Interface fastertools:mcp/authorization-types@0.4.0 **/
export type MetaFields = import('./fastertools-mcp-types.js').MetaFields;
/**
 * Provider declares its authentication requirements
 * This is returned by core-capabilities::get-auth-config()
 * and used by the transport to enforce authentication
 */
export interface ProviderAuthConfig {
  /**
   * Expected JWT issuer (REQUIRED for auth)
   */
  expectedIssuer: string,
  /**
   * Expected JWT audiences (REQUIRED for auth - must have at least one)
   */
  expectedAudiences: Array<string>,
  /**
   * JWKS URI for key discovery (REQUIRED for auth)
   */
  jwksUri: string,
  /**
   * Optional Rego policy for authorization
   */
  policy?: string,
  /**
   * Optional data for policy evaluation
   */
  policyData?: string,
}
/**
 * Authorization context passed between components after successful authentication
 */
export interface AuthContext {
  /**
   * OAuth client ID that made the request
   */
  clientId?: string,
  /**
   * Subject (user ID) from the token
   */
  userId?: string,
  /**
   * OAuth scopes granted to this token
   */
  scopes: Array<string>,
  /**
   * Token issuer URL
   */
  issuer?: string,
  /**
   * Audience claim from token
   */
  audience?: string,
  /**
   * Additional claims from token as key-value pairs
   */
  claims: MetaFields,
  /**
   * Expiration timestamp (Unix seconds)
   */
  exp?: bigint,
  /**
   * Issued at timestamp (Unix seconds)
   */
  iat?: bigint,
}
/**
 * Authorization request containing all context needed for authorization decisions
 */
export interface AuthRequest {
  /**
   * Bearer token extracted from Authorization header
   */
  token: string,
  /**
   * HTTP method (GET, POST, etc.)
   */
  method: string,
  /**
   * Request path
   */
  path: string,
  /**
   * Request headers as key-value pairs
   */
  headers: Array<[string, string]>,
  /**
   * Request body for policy evaluation (e.g., MCP JSON-RPC payload)
   */
  body?: Uint8Array,
  /**
   * Expected issuer for validation
   */
  expectedIssuer: string,
  /**
   * Expected audiences for validation (token must match at least one)
   */
  expectedAudiences: Array<string>,
  /**
   * JWKS URI for key discovery
   */
  jwksUri: string,
  /**
   * Optional Rego policy to evaluate (if not provided, allows all authenticated requests)
   */
  policy?: string,
  /**
   * Optional data for policy evaluation (JSON string)
   */
  policyData?: string,
}
/**
 * Authorization error details
 */
export interface AuthError {
  /**
   * HTTP status code (401, 403, etc.)
   */
  status: number,
  /**
   * OAuth error code (invalid_token, insufficient_scope, etc.)
   */
  errorCode: string,
  /**
   * Human-readable error description
   */
  description: string,
  /**
   * WWW-Authenticate header value for 401 responses
   */
  wwwAuthenticate?: string,
}
/**
 * Authorization response
 */
export type AuthResponse = AuthResponseAuthorized | AuthResponseUnauthorized;
/**
 * Request is authorized with context
 */
export interface AuthResponseAuthorized {
  tag: 'authorized',
  val: AuthContext,
}
/**
 * Request is unauthorized with error details
 */
export interface AuthResponseUnauthorized {
  tag: 'unauthorized',
  val: AuthError,
}
