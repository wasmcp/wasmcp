/** @module Interface wasmcp:mcp/authorization-types@0.2.0 **/
export type MetaFields = import('./wasmcp-mcp-mcp-types.js').MetaFields;
/**
 * Provider declares its authorization requirements
 * This is returned by get-auth-config()
 * and used by the transport to enforce authorization
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
   * Pass raw JWT token to tools via "jwt.token" meta field.
   */
  passJwt: boolean,
  /**
   * Expected JWT subject - if set, only this exact subject is allowed
   */
  expectedSubject?: string,
  /**
   * Optional Rego policy for complex authorization rules
   */
  policy?: string,
  /**
   * Optional data for policy evaluation
   */
  policyData?: string,
}
/**
 * Authorization context passed between components after successful authorization
 */
export interface AuthContext {
  /**
   * OAuth client ID that made the request
   */
  clientId?: string,
  /**
   * Subject claim from the token - always present from validated JWT
   */
  sub: string,
  /**
   * OAuth scopes granted to this token
   */
  scopes: Array<string>,
  /**
   * Issuer claim from the token - always present from validated JWT
   */
  iss: string,
  /**
   * Audience claim from token (aud) - always validated, can be multiple values
   */
  aud: Array<string>,
  /**
   * Additional claims from token as key-value pairs
   */
  claims: MetaFields,
  /**
   * Expiration timestamp (Unix seconds) - always validated and required for security
   */
  exp: bigint,
  /**
   * Issued at timestamp (Unix seconds)
   */
  iat?: bigint,
  /**
   * Not before timestamp (Unix seconds)
   */
  nbf?: bigint,
  /**
   * Raw JWT iff enabled by pass-jwt flag in provider-auth-config
   */
  jwt?: string,
}
