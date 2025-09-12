package capabilities

import (
	"go.bytecodealliance.org/cm"
	authorizationtypes "weather_go/internal/wasmcp/mcp/authorization-types"
)

// GetAuthConfig returns the provider's auth configuration
func GetAuthConfig() cm.Option[authorizationtypes.ProviderAuthConfig] {
	// No auth required for this example
	return cm.None[authorizationtypes.ProviderAuthConfig]()
}

// JwksCacheGet retrieves cached JWKS for a given URI
func JwksCacheGet(jwksURI string) cm.Option[string] {
	// No caching implemented for this example
	return cm.None[string]()
}

// JwksCacheSet caches JWKS for a given URI
func JwksCacheSet(jwksURI string, jwks string) {
	// No caching implemented for this example
}