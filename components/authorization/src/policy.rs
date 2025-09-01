use regorus::{Engine, Value};
use serde_json;

use crate::bindings::exports::fastertools::mcp::policy_engine::{
    PolicyRequest, PolicyResult,
};

pub fn evaluate(request: PolicyRequest) -> PolicyResult {
    // Create a new Regorus engine
    let mut engine = Engine::new();
    
    // Add the policy
    if let Err(e) = engine.add_policy("authorization.rego".to_string(), request.policy) {
        return PolicyResult::Error(format!("Failed to parse policy: {}", e));
    }
    
    // Add external data if provided
    if let Some(data_json) = request.data {
        match Value::from_json_str(&data_json) {
            Ok(data_value) => {
                if let Err(e) = engine.add_data(data_value) {
                    return PolicyResult::Error(format!("Failed to add policy data: {}", e));
                }
            }
            Err(e) => {
                return PolicyResult::Error(format!("Failed to parse policy data: {}", e));
            }
        }
    }
    
    // Parse and set the input
    match Value::from_json_str(&request.input) {
        Ok(input_value) => {
            engine.set_input(input_value);
        }
        Err(e) => {
            return PolicyResult::Error(format!("Failed to parse input: {}", e));
        }
    }
    
    // Determine the query to evaluate
    let query = request.query
        .unwrap_or_else(|| "data.mcp.authorization.allow".to_string());
    
    // Evaluate the policy
    match engine.eval_rule(query) {
        Ok(value) => {
            match value {
                Value::Bool(true) => PolicyResult::Allow,
                Value::Bool(false) => {
                    // Try to get a denial reason if available
                    match engine.eval_rule("data.mcp.authorization.deny_reason".to_string()) {
                        Ok(Value::String(reason)) => PolicyResult::Deny(reason.to_string()),
                        _ => PolicyResult::Deny("Access denied by policy".to_string()),
                    }
                }
                Value::Undefined => {
                    // Undefined means the rule doesn't exist or didn't match
                    PolicyResult::Deny("Policy rule undefined or not matched".to_string())
                }
                _ => {
                    PolicyResult::Error(format!("Policy returned non-boolean value: {:?}", value))
                }
            }
        }
        Err(e) => {
            PolicyResult::Error(format!("Policy evaluation failed: {}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_allow_policy() {
        let policy = r#"
            package mcp.authorization
            
            default allow = false
            
            allow {
                input.token.sub == "user123"
            }
        "#;
        
        let input = r#"{
            "token": {
                "sub": "user123"
            }
        }"#;
        
        let request = PolicyRequest {
            policy: policy.to_string(),
            data: None,
            input: input.to_string(),
            query: Some("data.mcp.authorization.allow".to_string()),
        };
        
        match evaluate(request) {
            PolicyResult::Allow => {}
            other => panic!("Expected Allow, got {:?}", other),
        }
    }
    
    #[test]
    fn test_simple_deny_policy() {
        let policy = r#"
            package mcp.authorization
            
            default allow = false
            
            allow {
                input.token.sub == "admin"
            }
            
            deny_reason = "User is not admin" {
                input.token.sub != "admin"
            }
        "#;
        
        let input = r#"{
            "token": {
                "sub": "user123"
            }
        }"#;
        
        let request = PolicyRequest {
            policy: policy.to_string(),
            data: None,
            input: input.to_string(),
            query: Some("data.mcp.authorization.allow".to_string()),
        };
        
        match evaluate(request) {
            PolicyResult::Deny(reason) => {
                assert!(reason.contains("User is not admin") || reason.contains("Access denied"));
            }
            other => panic!("Expected Deny, got {:?}", other),
        }
    }
    
    #[test]
    fn test_policy_with_external_data() {
        let policy = r#"
            package mcp.authorization
            
            default allow = false
            
            allow {
                input.token.sub == data.allowed_users[_]
            }
        "#;
        
        let data = r#"{
            "allowed_users": ["user1", "user2", "user3"]
        }"#;
        
        let input = r#"{
            "token": {
                "sub": "user2"
            }
        }"#;
        
        let request = PolicyRequest {
            policy: policy.to_string(),
            data: Some(data.to_string()),
            input: input.to_string(),
            query: Some("data.mcp.authorization.allow".to_string()),
        };
        
        match evaluate(request) {
            PolicyResult::Allow => {}
            other => panic!("Expected Allow, got {:?}", other),
        }
    }
}