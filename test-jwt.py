#!/usr/bin/env python3
"""
Generate a test JWT token for local testing.
For production, use your actual OAuth provider.
"""

import jwt
import json
import time
from datetime import datetime, timedelta

# Test configuration - matches our default test values
ISSUER = "https://test.example.com/"
AUDIENCE = "https://mcp.example.com"
SECRET = "test-secret-key-do-not-use-in-production"

def create_test_token(
    sub="test-user",
    client_id="test-client", 
    scopes=["mcp:tools:read", "mcp:tools:write"],
    expires_in=3600
):
    """Create a test JWT token"""
    
    now = datetime.utcnow()
    
    payload = {
        "iss": ISSUER,
        "sub": sub,
        "aud": [AUDIENCE],
        "iat": int(now.timestamp()),
        "exp": int((now + timedelta(seconds=expires_in)).timestamp()),
        "nbf": int(now.timestamp()),
        "client_id": client_id,
        "scopes": scopes,
        # Additional claims
        "email": f"{sub}@example.com",
        "name": f"Test User {sub}",
    }
    
    # For testing, we use HS256 (symmetric). 
    # Real providers use RS256 (asymmetric) with JWKS
    token = jwt.encode(payload, SECRET, algorithm="HS256")
    
    return token

def decode_and_print(token):
    """Decode and pretty-print a token for debugging"""
    # Decode without verification to see contents
    decoded = jwt.decode(token, options={"verify_signature": False})
    
    print("\nToken payload:")
    print(json.dumps(decoded, indent=2))
    
    # Show expiration time
    exp = datetime.fromtimestamp(decoded['exp'])
    print(f"\nExpires at: {exp} (in {(exp - datetime.now()).total_seconds():.0f} seconds)")

if __name__ == "__main__":
    import sys
    
    if len(sys.argv) > 1 and sys.argv[1] == "--decode":
        # Decode mode: decode a provided token
        if len(sys.argv) < 3:
            print("Usage: python test-jwt.py --decode <token>")
            sys.exit(1)
        decode_and_print(sys.argv[2])
    else:
        # Generate mode
        print("Generating test JWT token...")
        print(f"Issuer: {ISSUER}")
        print(f"Audience: {AUDIENCE}")
        
        token = create_test_token()
        
        print(f"\nToken:\n{token}")
        
        decode_and_print(token)
        
        print("\n" + "="*50)
        print("Test the token with:")
        print(f"""
curl -X POST http://localhost:3000 \\
  -H "Content-Type: application/json" \\
  -H "Authorization: Bearer {token}" \\
  -d '{{"jsonrpc":"2.0","method":"tools/list","id":1}}'
""")