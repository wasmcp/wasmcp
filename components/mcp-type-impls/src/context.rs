use std::cell::RefCell;
use std::collections::HashMap;
use crate::bindings::exports::wasmcp::mcp::types::GuestContext;

pub struct Context {
    request_id: String,
    client_id: Option<String>,
    session_id: Option<String>,
    state: RefCell<HashMap<String, String>>,
}

impl GuestContext for Context {
    fn request_id(&self) -> String {
        self.request_id.clone()
    }

    fn client_id(&self) -> Option<String> {
        self.client_id.clone()
    }

    fn session_id(&self) -> Option<String> {
        self.session_id.clone()
    }

    fn get_state(&self, key: String) -> Option<String> {
        self.state.borrow().get(&key).cloned()
    }

    fn set_state(&self, key: String, value: String) -> Result<(), ()> {
        self.state.borrow_mut().insert(key, value);
        Ok(())
    }
}

impl Context {
    #[cfg(test)]
    pub fn new(request_id: String, client_id: Option<String>, session_id: Option<String>) -> Self {
        Self {
            request_id,
            client_id,
            session_id,
            state: RefCell::new(HashMap::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_basic() {
        let context = Context::new(
            "req-123".to_string(),
            Some("client-456".to_string()),
            Some("session-789".to_string())
        );

        assert_eq!(context.request_id(), "req-123");
        assert_eq!(context.client_id(), Some("client-456".to_string()));
        assert_eq!(context.session_id(), Some("session-789".to_string()));
    }

    #[test]
    fn test_context_optional_fields() {
        let context = Context::new(
            "req-abc".to_string(),
            None,
            None
        );

        assert_eq!(context.request_id(), "req-abc");
        assert_eq!(context.client_id(), None);
        assert_eq!(context.session_id(), None);
    }

    #[test]
    fn test_context_state_management() {
        let context = Context::new(
            "req-abc".to_string(),
            None,
            None
        );

        // Initially empty
        assert_eq!(context.get_state("key1".to_string()), None);

        // Set and get state
        context.set_state("key1".to_string(), "value1".to_string()).unwrap();
        context.set_state("key2".to_string(), "value2".to_string()).unwrap();

        assert_eq!(context.get_state("key1".to_string()), Some("value1".to_string()));
        assert_eq!(context.get_state("key2".to_string()), Some("value2".to_string()));

        // Update existing key
        context.set_state("key1".to_string(), "updated_value".to_string()).unwrap();
        assert_eq!(context.get_state("key1".to_string()), Some("updated_value".to_string()));
    }

    #[test]
    fn test_context_state_isolation() {
        let context1 = Context::new("req1".to_string(), None, None);
        let context2 = Context::new("req2".to_string(), None, None);

        context1.set_state("key".to_string(), "value1".to_string()).unwrap();
        context2.set_state("key".to_string(), "value2".to_string()).unwrap();

        assert_eq!(context1.get_state("key".to_string()), Some("value1".to_string()));
        assert_eq!(context2.get_state("key".to_string()), Some("value2".to_string()));
    }

    #[test]
    fn test_context_multiple_state_entries() {
        let context = Context::new("req".to_string(), None, None);

        // Add multiple state entries
        for i in 0..10 {
            context.set_state(format!("key_{}", i), format!("value_{}", i)).unwrap();
        }

        // Verify all entries
        for i in 0..10 {
            assert_eq!(
                context.get_state(format!("key_{}", i)),
                Some(format!("value_{}", i))
            );
        }

        // Non-existent key should return None
        assert_eq!(context.get_state("non_existent".to_string()), None);
    }
}