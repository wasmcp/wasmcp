//! Todo List with Authorization Patterns
//!
//! Demonstrates different authorization strategies for MCP tools:
//! - Scope-based: Tools require specific OAuth scopes (mcp:read, mcp:write)
//! - Role-based: Administrative operations require role=admin claim
//! - Attribute-based: Fine-grained control via allowed_tools claim
//!
//! Authorization is checked using JWT claims from MessageContext.
//!
//! Authorization model:
//! - mcp:read scope: list_items (view todos)
//! - mcp:write scope: add_item (create todos)
//! - role=admin claim: remove_item, clear_all (delete operations)
//!
//! State management:
//! - Uses session storage to persist todo items across requests
//! - Each session maintains its own todo list

mod bindings {
    wit_bindgen::generate!({
        world: "todo-list",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp_v20250618::tools::Guest;
use bindings::wasmcp::auth::types::JwtClaims;
use bindings::wasmcp::keyvalue::store::TypedValue;
use bindings::wasmcp::mcp_v20250618::mcp::*;
use bindings::wasmcp::mcp_v20250618::server_handler::MessageContext;
use bindings::wasmcp::mcp_v20250618::sessions;
use serde::{Deserialize, Serialize};
use serde_json::Value;

struct TodoListAuth;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TodoItem {
    id: usize,
    title: String,
    created_by: String,
}

impl Guest for TodoListAuth {
    fn list_tools(
        _ctx: MessageContext,
        _request: ListToolsRequest,
    ) -> Result<ListToolsResult, ErrorCode> {
        Ok(ListToolsResult {
            tools: vec![
                Tool {
                    name: "add_item".to_string(),
                    input_schema: r#"{
                        "type": "object",
                        "properties": {
                            "title": {"type": "string", "description": "Todo item description"}
                        },
                        "required": ["title"]
                    }"#
                    .to_string(),
                    options: Some(ToolOptions {
                        description: Some("Add a new todo item (requires mcp:write scope)".to_string()),
                        title: Some("Add Item".to_string()),
                        meta: Some(r#"{"component_id":"todo-list-auth","tags":{"category":"productivity","tool-level":"foundational"}}"#.to_string()),
                        annotations: None,
                        output_schema: None,
                    }),
                },
                Tool {
                    name: "list_items".to_string(),
                    input_schema: r#"{
                        "type": "object",
                        "properties": {}
                    }"#
                    .to_string(),
                    options: Some(ToolOptions {
                        description: Some("List all todo items (requires mcp:read scope)".to_string()),
                        title: Some("List Items".to_string()),
                        meta: Some(r#"{"component_id":"todo-list-auth","tags":{"category":"productivity","tool-level":"foundational"}}"#.to_string()),
                        annotations: None,
                        output_schema: None,
                    }),
                },
                Tool {
                    name: "remove_item".to_string(),
                    input_schema: r#"{
                        "type": "object",
                        "properties": {
                            "id": {"type": "number", "description": "Todo item ID to remove"}
                        },
                        "required": ["id"]
                    }"#
                    .to_string(),
                    options: Some(ToolOptions {
                        description: Some(
                            "Remove a todo item (requires role=admin claim)".to_string()
                        ),
                        title: Some("Remove Item".to_string()),
                        meta: Some(r#"{"component_id":"todo-list-auth","tags":{"category":"productivity","tool-level":"foundational"}}"#.to_string()),
                        annotations: None,
                        output_schema: None,
                    }),
                },
                Tool {
                    name: "clear_all".to_string(),
                    input_schema: r#"{
                        "type": "object",
                        "properties": {}
                    }"#
                    .to_string(),
                    options: Some(ToolOptions {
                        description: Some(
                            "Clear all todo items (requires role=admin claim)".to_string()
                        ),
                        title: Some("Clear All".to_string()),
                        meta: Some(r#"{"component_id":"todo-list-auth","tags":{"category":"productivity","tool-level":"foundational"}}"#.to_string()),
                        annotations: None,
                        output_schema: None,
                    }),
                },
            ],
            next_cursor: None,
            meta: None,
        })
    }

    fn call_tool(
        ctx: MessageContext,
        request: CallToolRequest,
    ) -> Result<Option<CallToolResult>, ErrorCode> {
        // Extract JWT claims from identity (if present)
        let claims = ctx.identity.as_ref().map(|id| &id.claims);

        // Get user identifier from JWT subject claim
        let user = ctx
            .identity
            .as_ref()
            .and_then(|id| bindings::wasmcp::auth::helpers::get_claim(&id.claims, "sub"))
            .unwrap_or_else(|| "anonymous".to_string());

        // Get session - required for state persistence
        let session = match &ctx.session {
            Some(s) => s,
            None => {
                return Ok(Some(error_result(
                    "Session required for todo operations".to_string(),
                )));
            }
        };

        let result = match request.name.as_str() {
            "add_item" => {
                // Requires mcp:write scope (creates new state)
                if !check_scope(claims, "mcp:write") {
                    return Ok(Some(auth_error("add_item", "mcp:write scope required")));
                }
                if !check_tool_allowed(claims, "add_item") {
                    return Ok(Some(auth_error("add_item", "Tool not in allowed_tools list")));
                }
                Some(execute_add_item(&request.arguments, &user, session))
            }
            "list_items" => {
                // Requires mcp:read scope
                if !check_scope(claims, "mcp:read") {
                    return Ok(Some(auth_error("list_items", "mcp:read scope required")));
                }
                if !check_tool_allowed(claims, "list_items") {
                    return Ok(Some(auth_error("list_items", "Tool not in allowed_tools list")));
                }
                Some(execute_list_items(session))
            }
            "remove_item" => {
                // Requires role=admin claim (delete operation)
                if !check_role(claims, "admin") {
                    return Ok(Some(auth_error("remove_item", "role=admin claim required")));
                }
                Some(execute_remove_item(&request.arguments, session))
            }
            "clear_all" => {
                // Requires role=admin claim (delete operation)
                if !check_role(claims, "admin") {
                    return Ok(Some(auth_error("clear_all", "role=admin claim required")));
                }
                Some(execute_clear_all(session))
            }
            _ => None,
        };

        Ok(result)
    }
}

/// Check if JWT claims contain a specific scope using the auth helper functions
fn check_scope(claims: Option<&JwtClaims>, required_scope: &str) -> bool {
    match claims {
        Some(c) => bindings::wasmcp::auth::helpers::has_scope(c, required_scope),
        None => false, // No claims = no authorization
    }
}

/// Check if JWT claims contain a specific role (custom claim)
fn check_role(claims: Option<&JwtClaims>, required_role: &str) -> bool {
    match claims {
        Some(c) => {
            // Use get-claim helper to access custom 'role' claim
            match bindings::wasmcp::auth::helpers::get_claim(c, "role") {
                Some(role) => role == required_role,
                None => false,
            }
        }
        None => false,
    }
}

/// Check if tool is allowed based on allowed_tools claim
/// If allowed_tools claim is not present, allow all tools (no ABAC restriction)
fn check_tool_allowed(claims: Option<&JwtClaims>, tool_name: &str) -> bool {
    match claims {
        Some(c) => {
            // Use get-claim helper to access custom 'allowed_tools' claim
            match bindings::wasmcp::auth::helpers::get_claim(c, "allowed_tools") {
                Some(allowed) => {
                    // Parse comma-separated list
                    allowed.split(',').any(|t| t.trim() == tool_name)
                }
                None => true, // No allowed_tools claim means allow all
            }
        }
        None => false, // No claims = no authorization
    }
}

fn auth_error(tool_name: &str, message: &str) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(format!(
                "Authorization failed for '{}': {}",
                tool_name, message
            )),
            options: None,
        })],
        is_error: Some(true),
        meta: None,
        structured_content: None,
    }
}

/// Load todo list from session storage
fn load_todo_list(session: &Session) -> Vec<TodoItem> {
    let session_obj = match sessions::Session::open(&session.session_id, &session.store_id) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    match session_obj.get("todo:list") {
        Ok(Some(TypedValue::AsBytes(bytes))) => serde_json::from_slice(&bytes).unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Save todo list to session storage
fn save_todo_list(session: &Session, list: &[TodoItem]) {
    if let Ok(session_obj) = sessions::Session::open(&session.session_id, &session.store_id) {
        if let Ok(json_bytes) = serde_json::to_vec(list) {
            let _ = session_obj.set("todo:list", &TypedValue::AsBytes(json_bytes));
        }
    }
}

/// Get next ID from session storage
fn get_next_id(session: &Session) -> usize {
    let session_obj = match sessions::Session::open(&session.session_id, &session.store_id) {
        Ok(s) => s,
        Err(_) => return 1,
    };

    match session_obj.get("todo:next_id") {
        Ok(Some(TypedValue::AsString(id_str))) => id_str.parse().unwrap_or(1),
        _ => 1,
    }
}

/// Save next ID to session storage
fn save_next_id(session: &Session, id: usize) {
    if let Ok(session_obj) = sessions::Session::open(&session.session_id, &session.store_id) {
        let id_str = id.to_string();
        let _ = session_obj.set("todo:next_id", &TypedValue::AsString(id_str));
    }
}

fn execute_add_item(arguments: &Option<String>, user: &str, session: &Session) -> CallToolResult {
    let args_str = match arguments.as_ref() {
        Some(s) => s,
        None => return error_result("Missing arguments".to_string()),
    };

    let json: Value = match serde_json::from_str(args_str) {
        Ok(v) => v,
        Err(e) => return error_result(format!("Invalid JSON arguments: {}", e)),
    };

    let title = match json.get("title").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => return error_result("Missing or invalid parameter 'title'".to_string()),
    };

    // Load current state from session
    let mut list = load_todo_list(session);
    let id = get_next_id(session);

    // Add new item
    list.push(TodoItem {
        id,
        title: title.clone(),
        created_by: user.to_string(),
    });

    // Save updated state to session
    save_todo_list(session, &list);
    save_next_id(session, id + 1);

    success_result(format!(
        "Added todo item #{}: {} (created by: {})",
        id, title, user
    ))
}

fn execute_list_items(session: &Session) -> CallToolResult {
    let list = load_todo_list(session);

    if list.is_empty() {
        success_result("No todo items".to_string())
    } else {
        let mut items = Vec::new();
        for item in &list {
            items.push(format!("#{}: {} (by: {})", item.id, item.title, item.created_by));
        }
        success_result(format!("Todo items:\n{}", items.join("\n")))
    }
}

fn execute_remove_item(arguments: &Option<String>, session: &Session) -> CallToolResult {
    let args_str = match arguments.as_ref() {
        Some(s) => s,
        None => return error_result("Missing arguments".to_string()),
    };

    let json: Value = match serde_json::from_str(args_str) {
        Ok(v) => v,
        Err(e) => return error_result(format!("Invalid JSON arguments: {}", e)),
    };

    let id = match json.get("id").and_then(|v| v.as_u64()) {
        Some(i) => i as usize,
        None => return error_result("Missing or invalid parameter 'id'".to_string()),
    };

    // Load current state from session
    let mut list = load_todo_list(session);
    let original_len = list.len();

    // Remove item
    list.retain(|item| item.id != id);

    if list.len() < original_len {
        // Save updated state to session
        save_todo_list(session, &list);
        success_result(format!("Removed todo item #{}", id))
    } else {
        error_result(format!("Todo item #{} not found", id))
    }
}

fn execute_clear_all(session: &Session) -> CallToolResult {
    let list = load_todo_list(session);
    let count = list.len();

    // Clear list in session
    save_todo_list(session, &[]);
    save_next_id(session, 1);

    success_result(format!("Cleared {} todo items", count))
}

fn success_result(result: String) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(result),
            options: None,
        })],
        is_error: None,
        meta: None,
        structured_content: None,
    }
}

fn error_result(message: String) -> CallToolResult {
    CallToolResult {
        content: vec![ContentBlock::Text(TextContent {
            text: TextData::Text(message),
            options: None,
        })],
        is_error: Some(true),
        meta: None,
        structured_content: None,
    }
}

bindings::export!(TodoListAuth with_types_in bindings);
