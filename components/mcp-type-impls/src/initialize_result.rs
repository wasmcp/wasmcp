use std::cell::RefCell;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    GuestInitializeResult,
    InitializeResult as InitializeResultWrapper,
    ServerCapabilities,
};

pub struct InitializeResult {
    internal: RefCell<Value>,
    title: RefCell<Option<String>>,
    instructions: RefCell<Option<String>>,
}

impl GuestInitializeResult for InitializeResult {
    fn new(name: String, version: String, capabilities: ServerCapabilities) -> Self {
        Self {
            internal: RefCell::new(json!({
                "protocolVersion": "2025-06-18", // MCP protocol version
                "capabilities": build_capabilities(capabilities),
                "serverInfo": {
                    "name": name,
                    "version": version,
                },
            })),
            title: RefCell::new(None),
            instructions: RefCell::new(None),
        }
    }

    fn add_capabilities(&self, capabilities: ServerCapabilities) {
        let mut internal = self.internal.borrow_mut();
        let new_caps = build_capabilities(capabilities);

        if let Some(existing_caps) = internal.get_mut("capabilities").and_then(|v| v.as_object_mut()) {
            for (key, value) in new_caps.as_object().unwrap() {
                existing_caps.insert(key.clone(), value.clone());
            }
        }
    }

    fn add_meta(&self, key: String, value: String) -> Result<(), ()> {
        let mut internal = self.internal.borrow_mut();
        let meta = internal
            .as_object_mut()
            .ok_or(())?
            .entry("_meta")
            .or_insert_with(|| json!({}));

        meta[key] = json!(value);
        Ok(())
    }

    fn title(&self) -> Option<String> {
        self.title.borrow().clone()
    }

    fn set_title(&self, title: String) {
        *self.title.borrow_mut() = Some(title.clone());
        self.internal.borrow_mut()["serverInfo"]["title"] = json!(title);
    }

    fn instructions(&self) -> Option<String> {
        self.instructions.borrow().clone()
    }

    fn set_instructions(&self, instructions: String) {
        *self.instructions.borrow_mut() = Some(instructions.clone());
        self.internal.borrow_mut()["instructions"] = json!(instructions);
    }

    fn finish_json(this: InitializeResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

fn build_capabilities(capabilities: ServerCapabilities) -> Value {
    let mut caps = json!({});

    if capabilities.contains(ServerCapabilities::COMPLETIONS) {
        caps["completions"] = json!({});
    }
    if capabilities.contains(ServerCapabilities::PROMPTS) {
        caps["prompts"] = json!({});
    }
    if capabilities.contains(ServerCapabilities::RESOURCES) {
        caps["resources"] = json!({});
    }
    if capabilities.contains(ServerCapabilities::TOOLS) {
        caps["tools"] = json!({});
    }

    caps
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn test_initialize_result_basic() {
        let capabilities = ServerCapabilities::TOOLS | ServerCapabilities::RESOURCES;
        let result = InitializeResult::new(
            "test-server".to_string(),
            "1.0.0".to_string(),
            capabilities
        );

        let wrapper = InitializeResultWrapper::new(result);
        let json = InitializeResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["protocolVersion"], "2025-06-18");
        assert_eq!(parsed["serverInfo"]["name"], "test-server");
        assert_eq!(parsed["serverInfo"]["version"], "1.0.0");
        assert!(parsed["capabilities"]["tools"].is_object());
        assert!(parsed["capabilities"]["resources"].is_object());
        assert!(parsed["capabilities"]["prompts"].is_null());
    }

    #[test]
    fn test_initialize_result_with_optional_fields() {
        let capabilities = ServerCapabilities::all();
        let result = InitializeResult::new(
            "my-server".to_string(),
            "2.0.0".to_string(),
            capabilities
        );

        result.set_title("My Awesome Server".to_string());
        result.set_instructions("Use this server for awesome things".to_string());
        result.add_meta("session_id".to_string(), "sess123".to_string()).unwrap();

        let wrapper = InitializeResultWrapper::new(result);
        let json = InitializeResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["serverInfo"]["title"], "My Awesome Server");
        assert_eq!(parsed["instructions"], "Use this server for awesome things");
        assert_eq!(parsed["_meta"]["session_id"], "sess123");
    }

    #[test]
    fn test_initialize_getters() {
        let capabilities = ServerCapabilities::TOOLS;
        let result = InitializeResult::new(
            "test".to_string(),
            "1.0.0".to_string(),
            capabilities
        );

        assert_eq!(result.title(), None);
        assert_eq!(result.instructions(), None);

        result.set_title("Test Title".to_string());
        result.set_instructions("Test Instructions".to_string());

        assert_eq!(result.title(), Some("Test Title".to_string()));
        assert_eq!(result.instructions(), Some("Test Instructions".to_string()));
    }

    #[test]
    fn test_add_capabilities() {
        let result = InitializeResult::new(
            "test".to_string(),
            "1.0.0".to_string(),
            ServerCapabilities::TOOLS
        );

        // Add more capabilities after creation
        result.add_capabilities(ServerCapabilities::RESOURCES | ServerCapabilities::PROMPTS);

        let wrapper = InitializeResultWrapper::new(result);
        let json = InitializeResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        // Should have all three capabilities
        assert!(parsed["capabilities"]["tools"].is_object());
        assert!(parsed["capabilities"]["resources"].is_object());
        assert!(parsed["capabilities"]["prompts"].is_object());
        assert!(parsed["capabilities"]["completions"].is_null());
    }
}