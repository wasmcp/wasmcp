use crate::error::McpError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use regorus::{Engine, Value};

/// Policy engine for evaluating authorization policies using Rego
pub struct PolicyEngine {
    policies: HashMap<String, String>,
    default_query: String,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            default_query: "data.mcp.authorization.allow".to_string(),
        }
    }

    /// Set the default query to evaluate (defaults to "data.mcp.authorization.allow")
    pub fn with_default_query(mut self, query: String) -> Self {
        self.default_query = query;
        self
    }

    /// Load a policy from Rego source
    pub fn load_policy(&mut self, name: &str, rego_source: String) -> Result<(), McpError> {
        // Validate that the policy can be parsed
        let mut engine = Engine::new();
        engine.add_policy(format!("{}.rego", name), rego_source.clone())
            .map_err(|e| McpError::Policy(format!("Invalid policy '{}': {}", name, e)))?;
        
        self.policies.insert(name.to_string(), rego_source);
        Ok(())
    }

    /// Evaluate a policy with the given input
    pub async fn evaluate(
        &self,
        policy_name: &str,
        input: PolicyInput,
    ) -> Result<PolicyDecision, McpError> {
        let policy = self.policies.get(policy_name)
            .ok_or_else(|| McpError::Policy(format!("Policy '{}' not found", policy_name)))?;

        // Create a new Regorus engine for this evaluation
        let mut engine = Engine::new();

        // Add the policy
        engine.add_policy(format!("{}.rego", policy_name), policy.clone())
            .map_err(|e| McpError::Policy(format!("Failed to load policy: {}", e)))?;

        // Convert input to Rego format
        let input_json = serde_json::json!({
            "method": input.method,
            "path": input.path,
            "token": input.token_claims,
            "metadata": input.metadata,
        });

        let input_value = Value::from_json_str(&input_json.to_string())
            .map_err(|e| McpError::Policy(format!("Failed to parse input: {}", e)))?;
        
        engine.set_input(input_value);

        // Add external data if provided
        if let Some(data) = input.external_data {
            let data_value = Value::from_json_str(&data.to_string())
                .map_err(|e| McpError::Policy(format!("Failed to parse external data: {}", e)))?;
            engine.add_data(data_value)
                .map_err(|e| McpError::Policy(format!("Failed to add external data: {}", e)))?;
        }

        // Evaluate the policy using the configured query
        let query = input.query.as_ref().unwrap_or(&self.default_query);
        
        match engine.eval_rule(query.clone()) {
            Ok(value) => {
                match value {
                    Value::Bool(true) => Ok(PolicyDecision {
                        allow: true,
                        reason: None,
                    }),
                    Value::Bool(false) => {
                        // Try to get a denial reason if available
                        let reason = match engine.eval_rule("data.mcp.authorization.deny_reason".to_string()) {
                            Ok(Value::String(reason)) => Some(reason.to_string()),
                            _ => Some("Access denied by policy".to_string()),
                        };
                        Ok(PolicyDecision {
                            allow: false,
                            reason,
                        })
                    }
                    Value::Undefined => {
                        // Undefined means the rule doesn't exist or didn't match
                        Ok(PolicyDecision {
                            allow: false,
                            reason: Some("Policy rule undefined or not matched".to_string()),
                        })
                    }
                    _ => Err(McpError::Policy(format!("Policy returned non-boolean value: {:?}", value))),
                }
            }
            Err(e) => Err(McpError::Policy(format!("Policy evaluation failed: {}", e))),
        }
    }

    /// Load a standard MCP authorization policy
    pub fn load_standard_policy(&mut self) -> Result<(), McpError> {
        let standard_policy = r#"
            package mcp.authorization
            
            # Default deny
            default allow = false
            
            # Allow if token is valid and not expired
            allow {
                input.token.sub != ""
                input.token.exp > time.now_ns() / 1000000000
            }
            
            # Allow specific methods without token for discovery
            allow {
                input.method == "GET"
                input.path == "/.well-known/oauth-authorization-server"
            }
            
            allow {
                input.method == "GET"
                input.path == "/.well-known/openid-configuration"
            }
            
            # Denial reason when token is expired
            deny_reason = "Token expired" {
                input.token.exp <= time.now_ns() / 1000000000
            }
            
            # Denial reason when no token provided
            deny_reason = "No token provided" {
                not input.token
            }
        "#;

        self.load_policy("standard", standard_policy.to_string())
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInput {
    pub method: String,
    pub path: String,
    pub token_claims: Option<JsonValue>,
    pub metadata: HashMap<String, JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_data: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub allow: bool,
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_allow_policy() {
        let mut engine = PolicyEngine::new();
        
        let policy = r#"
            package mcp.authorization
            
            default allow = false
            
            allow if {
                input.token.sub == "user123"
            }
        "#;
        
        engine.load_policy("test", policy.to_string()).unwrap();
        
        let input = PolicyInput {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            token_claims: Some(serde_json::json!({
                "sub": "user123"
            })),
            metadata: HashMap::new(),
            external_data: None,
            query: None,
        };
        
        let decision = engine.evaluate("test", input).await.unwrap();
        assert!(decision.allow);
    }

    #[tokio::test]
    async fn test_deny_policy() {
        let mut engine = PolicyEngine::new();
        
        let policy = r#"
            package mcp.authorization
            
            default allow = false
            
            allow if {
                input.token.sub == "admin"
            }
            
            deny_reason = "Not an admin" if {
                input.token.sub != "admin"
            }
        "#;
        
        engine.load_policy("test", policy.to_string()).unwrap();
        
        let input = PolicyInput {
            method: "GET".to_string(),
            path: "/api/test".to_string(),
            token_claims: Some(serde_json::json!({
                "sub": "user123"
            })),
            metadata: HashMap::new(),
            external_data: None,
            query: None,
        };
        
        let decision = engine.evaluate("test", input).await.unwrap();
        assert!(!decision.allow);
        assert_eq!(decision.reason, Some("Not an admin".to_string()));
    }
}