# Role-Based Access Control (RBAC) policy for MCP
# This policy uses roles and permissions defined in external data

package mcp.authorization

import future.keywords.if
import future.keywords.in

# Default deny
default allow = false

# Allow if user has required permission for the action
allow if {
    user_roles := get_user_roles(input.token.sub)
    required_permission := get_required_permission
    some role in user_roles
    required_permission in data.roles[role].permissions
}

# Get user's roles from data or token claims
get_user_roles(user_id) = roles if {
    roles := data.users[user_id].roles
}

get_user_roles(user_id) = roles if {
    not data.users[user_id]
    roles := input.token.claims.roles
}

get_user_roles(user_id) = ["user"] if {
    not data.users[user_id]
    not input.token.claims.roles
}

# Map MCP methods to required permissions
get_required_permission = permission if {
    input.mcp.method == "tools/list"
    permission := "tools:list"
}

get_required_permission = permission if {
    input.mcp.method == "tools/call"
    permission := sprintf("tools:call:%s", [input.mcp.tool])
}

get_required_permission = permission if {
    input.mcp.method == "resources/list"
    permission := "resources:list"
}

get_required_permission = permission if {
    input.mcp.method == "resources/read"
    permission := "resources:read"
}

get_required_permission = permission if {
    input.mcp.method == "prompts/list"
    permission := "prompts:list"
}

get_required_permission = permission if {
    input.mcp.method == "prompts/get"
    permission := "prompts:get"
}

# Special handling for admin operations
allow if {
    "admin" in get_user_roles(input.token.sub)
}

# Audit log generation
audit_log = entry if {
    entry := {
        "timestamp": time.now_ns(),
        "user": input.token.sub,
        "method": input.mcp.method,
        "tool": input.mcp.tool,
        "allowed": allow,
        "roles": get_user_roles(input.token.sub),
    }
}

# Example data structure (would be provided externally)
# data = {
#     "users": {
#         "user1": {
#             "roles": ["developer", "tester"]
#         },
#         "user2": {
#             "roles": ["admin"]
#         }
#     },
#     "roles": {
#         "developer": {
#             "permissions": [
#                 "tools:list",
#                 "tools:call:build",
#                 "tools:call:test",
#                 "resources:list",
#                 "resources:read"
#             ]
#         },
#         "tester": {
#             "permissions": [
#                 "tools:list",
#                 "tools:call:test",
#                 "resources:list"
#             ]
#         },
#         "admin": {
#             "permissions": ["*"]
#         }
#     }
# }