# Fine-grained tool authorization policy for MCP
# Controls access to specific tools based on various criteria

package mcp.authorization

import future.keywords.if
import future.keywords.in
import future.keywords.contains

# Default deny
default allow = false

# Allow non-tool methods if authenticated
allow if {
    input.token.sub != ""
    input.mcp.method != "tools/call"
    has_read_scope
}

# Allow tool calls with proper authorization
allow if {
    input.mcp.method == "tools/call"
    tool_authorized
}

# Check basic read scope
has_read_scope if {
    input.mcp.method == "tools/list"
    "mcp:tools:read" in input.token.scopes
}

has_read_scope if {
    input.mcp.method in ["resources/list", "resources/read"]
    "mcp:resources:read" in input.token.scopes
}

has_read_scope if {
    input.mcp.method in ["prompts/list", "prompts/get"]
    "mcp:prompts:read" in input.token.scopes
}

# Tool authorization rules
tool_authorized if {
    # Must have base write permission
    "mcp:tools:write" in input.token.scopes
    
    # Check tool-specific rules
    tool_name := input.mcp.tool
    tool_check_passes(tool_name)
}

# Categorize tools and their requirements
tool_check_passes(tool) if {
    tool in safe_tools
}

tool_check_passes(tool) if {
    tool in financial_tools
    "finance" in input.token.scopes
}

tool_check_passes(tool) if {
    tool in admin_tools
    "admin" in input.token.scopes
}

tool_check_passes(tool) if {
    tool in data_modification_tools
    check_data_modification_permission
}

# Safe tools that any authenticated user with write scope can use
safe_tools := [
    "echo",
    "get_time",
    "get_weather",
    "search",
    "calculate",
    "translate"
]

# Financial tools requiring finance scope
financial_tools := [
    "process_payment",
    "refund_transaction",
    "generate_invoice",
    "update_billing",
    "access_financial_reports"
]

# Admin tools requiring admin scope
admin_tools := [
    "delete_database",
    "reset_system",
    "manage_users",
    "update_permissions",
    "access_logs"
]

# Data modification tools with special checks
data_modification_tools := [
    "execute_sql",
    "update_data",
    "bulk_delete",
    "import_data"
]

# Check data modification permission based on arguments
check_data_modification_permission if {
    # Check if it's a read-only operation
    is_read_only_operation
}

check_data_modification_permission if {
    # Destructive operations require admin
    not is_read_only_operation
    "admin" in input.token.scopes
}

# Determine if operation is read-only based on arguments
is_read_only_operation if {
    input.mcp.tool == "execute_sql"
    sql := lower(input.mcp.arguments.query)
    startswith(sql, "select")
    not contains(sql, "update")
    not contains(sql, "delete")
    not contains(sql, "drop")
    not contains(sql, "truncate")
    not contains(sql, "insert")
}

is_read_only_operation if {
    input.mcp.tool == "update_data"
    input.mcp.arguments.readonly == true
}

# Rate limiting rules
rate_limit_exceeded if {
    tool := input.mcp.tool
    user := input.token.sub
    
    # Get user's recent calls count from data (would be tracked externally)
    recent_calls := data.rate_limits[user][tool]
    recent_calls > get_rate_limit(tool)
}

get_rate_limit(tool) = 100 if { tool in safe_tools }
get_rate_limit(tool) = 10 if { tool in financial_tools }
get_rate_limit(tool) = 5 if { tool in admin_tools }
get_rate_limit(tool) = 20 if { tool in data_modification_tools }
get_rate_limit(tool) = 50 # default

# Deny with specific reason
deny_reason = msg if {
    not allow
    msg := determine_denial_reason
}

determine_denial_reason = "Authentication required" if {
    not input.token.sub
}

determine_denial_reason = "Missing mcp:tools:write scope" if {
    input.mcp.method == "tools/call"
    not "mcp:tools:write" in input.token.scopes
}

determine_denial_reason = msg if {
    input.mcp.method == "tools/call"
    input.mcp.tool in financial_tools
    not "finance" in input.token.scopes
    msg := sprintf("Tool '%s' requires finance scope", [input.mcp.tool])
}

determine_denial_reason = msg if {
    input.mcp.method == "tools/call"
    input.mcp.tool in admin_tools
    not "admin" in input.token.scopes
    msg := sprintf("Tool '%s' requires admin scope", [input.mcp.tool])
}

determine_denial_reason = msg if {
    input.mcp.method == "tools/call"
    input.mcp.tool in data_modification_tools
    not is_read_only_operation
    not "admin" in input.token.scopes
    msg := sprintf("Destructive operation on '%s' requires admin scope", [input.mcp.tool])
}

determine_denial_reason = msg if {
    rate_limit_exceeded
    msg := sprintf("Rate limit exceeded for tool '%s'", [input.mcp.tool])
}

determine_denial_reason = "Access denied by policy"