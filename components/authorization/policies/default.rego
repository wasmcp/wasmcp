# Default authorization policy for MCP servers
# This policy implements basic OAuth scope-based authorization

package mcp.authorization

import future.keywords.if
import future.keywords.in

# Default deny
default allow = false

# Allow if user is authenticated (has a valid subject)
allow if {
    input.token.sub != ""
    valid_request
}

# Check if the request is valid based on method and scopes
valid_request if {
    # Check method-specific authorization
    method_allowed
}

# Method-specific authorization rules
method_allowed if {
    input.mcp.method == "tools/list"
    "mcp:tools:read" in input.token.scopes
}

method_allowed if {
    input.mcp.method == "tools/call"
    "mcp:tools:write" in input.token.scopes
    tool_allowed
}

method_allowed if {
    input.mcp.method == "resources/list"
    "mcp:resources:read" in input.token.scopes
}

method_allowed if {
    input.mcp.method == "resources/read"
    "mcp:resources:read" in input.token.scopes
    resource_allowed
}

method_allowed if {
    input.mcp.method == "prompts/list"
    "mcp:prompts:read" in input.token.scopes
}

method_allowed if {
    input.mcp.method == "prompts/get"
    "mcp:prompts:read" in input.token.scopes
}

# Tool-specific authorization
tool_allowed if {
    # Allow by default if user has write scope
    "mcp:tools:write" in input.token.scopes
    not dangerous_tool
}

tool_allowed if {
    # Dangerous tools require admin scope
    dangerous_tool
    "admin" in input.token.scopes
}

# Define dangerous tools
dangerous_tool if {
    input.mcp.tool in ["delete_database", "drop_table", "reset_system", "execute_sql"]
}

# Resource-specific authorization
resource_allowed if {
    # Public resources are always allowed with read scope
    not sensitive_resource
}

resource_allowed if {
    # Sensitive resources require additional scope
    sensitive_resource
    "sensitive" in input.token.scopes
}

# Define sensitive resources
sensitive_resource if {
    startswith(input.request.path, "/sensitive/")
}

# Provide detailed denial reasons
deny_reason = msg if {
    not allow
    msg := determine_denial_reason
}

determine_denial_reason = "No valid authentication token" if {
    not input.token.sub
}

determine_denial_reason = "Insufficient scope for method" if {
    input.token.sub
    not method_allowed
}

determine_denial_reason = msg if {
    input.mcp.method == "tools/call"
    dangerous_tool
    not "admin" in input.token.scopes
    msg := sprintf("Tool '%s' requires admin scope", [input.mcp.tool])
}

determine_denial_reason = "Access denied by policy" if {
    input.token.sub
}