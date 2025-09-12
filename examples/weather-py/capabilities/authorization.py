"""Authorization implementation for weather-py MCP server."""

from typing import Optional
from wit_world.imports.authorization_types import ProviderAuthConfig


class Authorization:
    """Handle MCP authorization methods."""
    
    def get_auth_config(self) -> Optional[ProviderAuthConfig]:
        """Return auth configuration. None disables auth."""
        return None
        # Uncomment and configure for OAuth authorization:
        # return ProviderAuthConfig(
        #     expected_issuer="https://xxx.authkit.app",
        #     expected_audiences=["client_xxx"],
        #     jwks_uri="https://xxx.authkit.app/oauth2/jwks",
        #     pass_jwt=False,
        #     expected_subject=None,
        #     policy=None,  # Optional: Add Rego policy for additional authorization
        #     policy_data=None,  # Optional: Add policy data as JSON string
        # )
    
    def jwks_cache_get(self, jwks_uri: str) -> Optional[str]:
        """Get cached JWKS. No caching for this example."""
        return None
    
    def jwks_cache_set(self, jwks_uri: str, jwks: str) -> None:
        """Cache JWKS. No caching for this example."""
        pass