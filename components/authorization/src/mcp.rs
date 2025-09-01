use serde_json;

use crate::bindings::exports::fastertools::mcp::{
    authorization::AuthError,
    mcp_authorization::{McpAuthRequest, ResourceAuthRequest, ResourceOperation, ToolAuthRequest},
};

/// Authorize an MCP method call
pub fn authorize_method(request: McpAuthRequest) -> Result<(), AuthError> {
    // Check if user has required scope for the method
    let required_scope = match request.method.as_str() {
        "tools/list" => "mcp:tools:read",
        "tools/call" => "mcp:tools:write",
        "resources/list" => "mcp:resources:read",
        "resources/read" => "mcp:resources:read",
        "resources/subscribe" => "mcp:resources:read",
        "resources/unsubscribe" => "mcp:resources:read",
        "prompts/list" => "mcp:prompts:read",
        "prompts/get" => "mcp:prompts:read",
        _ => {
            // Unknown method - allow if authenticated
            return Ok(());
        }
    };
    
    // Check if the user has the required scope
    if !request.context.scopes.contains(&required_scope.to_string()) {
        return Err(AuthError {
            status: 403,
            error_code: "insufficient_scope".to_string(),
            description: format!("Missing required scope: {}", required_scope),
            www_authenticate: Some(format!(r#"Bearer scope="{}""#, required_scope)),
        });
    }
    
    // Additional method-specific checks
    if request.method == "tools/call" {
        // Extract tool name from params if available
        if let Some(params) = request.params {
            if let Ok(params_obj) = serde_json::from_str::<serde_json::Value>(&params) {
                if let Some(tool_name) = params_obj.get("name").and_then(|n| n.as_str()) {
                    // Check tool-specific authorization
                    return authorize_tool_internal(
                        &request.context,
                        tool_name,
                        params_obj.get("arguments").cloned(),
                    );
                }
            }
        }
    }
    
    Ok(())
}

/// Authorize a specific tool call
pub fn authorize_tool(request: ToolAuthRequest) -> Result<(), AuthError> {
    authorize_tool_internal(
        &request.context,
        &request.tool_name,
        request.arguments.and_then(|a| serde_json::from_str(&a).ok()),
    )
}

fn authorize_tool_internal(
    context: &crate::bindings::exports::fastertools::mcp::authorization::AuthContext,
    tool_name: &str,
    arguments: Option<serde_json::Value>,
) -> Result<(), AuthError> {
    // Check base write permission
    if !context.scopes.contains(&"mcp:tools:write".to_string()) {
        return Err(AuthError {
            status: 403,
            error_code: "insufficient_scope".to_string(),
            description: "Missing mcp:tools:write scope".to_string(),
            www_authenticate: Some(r#"Bearer scope="mcp:tools:write""#.to_string()),
        });
    }
    
    // Tool-specific authorization rules
    // These would typically be configured via policy or environment
    match tool_name {
        // Example: Dangerous tools require admin scope
        "delete_database" | "drop_table" | "reset_system" => {
            if !context.scopes.contains(&"admin".to_string()) {
                return Err(AuthError {
                    status: 403,
                    error_code: "insufficient_scope".to_string(),
                    description: format!("Tool '{}' requires admin scope", tool_name),
                    www_authenticate: Some(r#"Bearer scope="admin""#.to_string()),
                });
            }
        }
        
        // Example: Financial tools require finance scope
        tool if tool.starts_with("finance_") || tool.starts_with("payment_") => {
            if !context.scopes.contains(&"finance".to_string()) {
                return Err(AuthError {
                    status: 403,
                    error_code: "insufficient_scope".to_string(),
                    description: format!("Tool '{}' requires finance scope", tool_name),
                    www_authenticate: Some(r#"Bearer scope="finance""#.to_string()),
                });
            }
        }
        
        // Example: Check specific argument values
        "execute_sql" => {
            if let Some(args) = arguments {
                if let Some(query) = args.get("query").and_then(|q| q.as_str()) {
                    let query_lower = query.to_lowercase();
                    // Block destructive SQL operations without admin scope
                    if (query_lower.contains("drop") || 
                        query_lower.contains("delete") || 
                        query_lower.contains("truncate")) &&
                       !context.scopes.contains(&"admin".to_string()) {
                        return Err(AuthError {
                            status: 403,
                            error_code: "insufficient_scope".to_string(),
                            description: "Destructive SQL operations require admin scope".to_string(),
                            www_authenticate: Some(r#"Bearer scope="admin""#.to_string()),
                        });
                    }
                }
            }
        }
        
        _ => {
            // Default: allow if user has tools:write scope
        }
    }
    
    Ok(())
}

/// Authorize resource access
pub fn authorize_resource(request: ResourceAuthRequest) -> Result<(), AuthError> {
    // Determine required scope based on operation
    let required_scope = match request.operation {
        ResourceOperation::List => "mcp:resources:read",
        ResourceOperation::Read => "mcp:resources:read",
        ResourceOperation::Subscribe => "mcp:resources:read",
        ResourceOperation::Unsubscribe => "mcp:resources:read",
    };
    
    // Check base permission
    if !request.context.scopes.contains(&required_scope.to_string()) {
        return Err(AuthError {
            status: 403,
            error_code: "insufficient_scope".to_string(),
            description: format!("Missing required scope: {}", required_scope),
            www_authenticate: Some(format!(r#"Bearer scope="{}""#, required_scope)),
        });
    }
    
    // URI-specific authorization rules
    let uri = &request.uri;
    
    // Example: Sensitive resources require additional scopes
    if uri.starts_with("secret://") || uri.starts_with("private://") {
        if !request.context.scopes.contains(&"sensitive".to_string()) {
            return Err(AuthError {
                status: 403,
                error_code: "insufficient_scope".to_string(),
                description: "Access to sensitive resources requires 'sensitive' scope".to_string(),
                www_authenticate: Some(r#"Bearer scope="sensitive""#.to_string()),
            });
        }
    }
    
    // Example: System resources require admin scope
    if uri.starts_with("system://") {
        if !request.context.scopes.contains(&"admin".to_string()) {
            return Err(AuthError {
                status: 403,
                error_code: "insufficient_scope".to_string(),
                description: "Access to system resources requires 'admin' scope".to_string(),
                www_authenticate: Some(r#"Bearer scope="admin""#.to_string()),
            });
        }
    }
    
    // Example: User-specific resources
    if uri.starts_with("user://") {
        // Extract user ID from URI (e.g., user://user123/profile)
        if let Some(uri_user) = uri.strip_prefix("user://")
            .and_then(|s| s.split('/').next()) {
            // Check if user is accessing their own resources
            if let Some(ref user_id) = request.context.user_id {
                if uri_user != user_id && !request.context.scopes.contains(&"admin".to_string()) {
                    return Err(AuthError {
                        status: 403,
                        error_code: "insufficient_scope".to_string(),
                        description: "Cannot access other users' resources without admin scope".to_string(),
                        www_authenticate: Some(r#"Bearer scope="admin""#.to_string()),
                    });
                }
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::exports::fastertools::mcp::authorization::AuthContext;
    
    fn create_test_context(scopes: Vec<&str>) -> AuthContext {
        AuthContext {
            client_id: Some("test-client".to_string()),
            user_id: Some("user123".to_string()),
            scopes: scopes.into_iter().map(String::from).collect(),
            issuer: Some("https://auth.example.com".to_string()),
            audience: Some("https://mcp.example.com".to_string()),
            claims: vec![],
            exp: Some(1234567890),
            iat: Some(1234567800),
        }
    }
    
    #[test]
    fn test_authorize_method_with_scope() {
        let request = McpAuthRequest {
            context: create_test_context(vec!["mcp:tools:read"]),
            method: "tools/list".to_string(),
            params: None,
        };
        
        assert!(authorize_method(request).is_ok());
    }
    
    #[test]
    fn test_authorize_method_without_scope() {
        let request = McpAuthRequest {
            context: create_test_context(vec![]),
            method: "tools/list".to_string(),
            params: None,
        };
        
        assert!(authorize_method(request).is_err());
    }
    
    #[test]
    fn test_authorize_dangerous_tool() {
        let request = ToolAuthRequest {
            context: create_test_context(vec!["mcp:tools:write"]),
            tool_name: "delete_database".to_string(),
            arguments: None,
        };
        
        // Should fail without admin scope
        assert!(authorize_tool(request).is_err());
        
        let request_admin = ToolAuthRequest {
            context: create_test_context(vec!["mcp:tools:write", "admin"]),
            tool_name: "delete_database".to_string(),
            arguments: None,
        };
        
        // Should succeed with admin scope
        assert!(authorize_tool(request_admin).is_ok());
    }
    
    #[test]
    fn test_authorize_user_resource() {
        let request = ResourceAuthRequest {
            context: create_test_context(vec!["mcp:resources:read"]),
            uri: "user://user123/profile".to_string(),
            operation: ResourceOperation::Read,
        };
        
        // User can access their own resource
        assert!(authorize_resource(request).is_ok());
        
        let request_other = ResourceAuthRequest {
            context: create_test_context(vec!["mcp:resources:read"]),
            uri: "user://user456/profile".to_string(),
            operation: ResourceOperation::Read,
        };
        
        // User cannot access other user's resource without admin
        assert!(authorize_resource(request_other).is_err());
    }
}