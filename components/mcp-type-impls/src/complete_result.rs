use std::cell::RefCell;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    GuestCompleteResult,
    CompleteResult as CompleteResultWrapper,
};

pub struct CompleteResult {
    internal: RefCell<Value>,
}

impl GuestCompleteResult for CompleteResult {
    fn new(initial_values: Vec<String>) -> Self {
        Self {
            internal: RefCell::new(json!({
                "completion": {
                    "values": initial_values,
                }
            })),
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

    fn set_has_more(&self) {
        self.internal.borrow_mut()["completion"]["hasMore"] = json!(true);
    }

    fn set_total(&self, total: u16) {
        self.internal.borrow_mut()["completion"]["total"] = json!(total);
    }

    fn add_value(&self, value: String) {
        self.internal.borrow_mut()["completion"]["values"]
            .as_array_mut()
            .expect("values should be an array")
            .push(json!(value));
    }

    fn finish_json(this: CompleteResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn test_complete_result_basic() {
        let initial_values = vec!["option1".to_string(), "option2".to_string()];
        let result = CompleteResult::new(initial_values);

        result.add_value("option3".to_string());

        let wrapper = CompleteResultWrapper::new(result);
        let json = CompleteResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["completion"]["values"].as_array().unwrap().len(), 3);
        assert_eq!(parsed["completion"]["values"][0], "option1");
        assert_eq!(parsed["completion"]["values"][1], "option2");
        assert_eq!(parsed["completion"]["values"][2], "option3");
        assert_eq!(parsed["completion"].get("hasMore"), None); // Not set
        assert_eq!(parsed["completion"].get("total"), None); // Not set
    }

    #[test]
    fn test_complete_result_with_pagination() {
        let initial_values = vec!["value1".to_string()];
        let result = CompleteResult::new(initial_values);

        result.set_has_more();
        result.set_total(100);
        result.add_meta("context".to_string(), "search".to_string()).unwrap();

        for i in 2..=10 {
            result.add_value(format!("value{}", i));
        }

        let wrapper = CompleteResultWrapper::new(result);
        let json = CompleteResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["completion"]["hasMore"], true);
        assert_eq!(parsed["completion"]["total"], 100);
        assert_eq!(parsed["completion"]["values"].as_array().unwrap().len(), 10);
        assert_eq!(parsed["_meta"]["context"], "search");
    }

    #[test]
    fn test_complete_result_empty() {
        let result = CompleteResult::new(vec![]);

        let wrapper = CompleteResultWrapper::new(result);
        let json = CompleteResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["completion"]["values"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_complete_result_large_list() {
        let mut initial_values = Vec::new();
        for i in 0..50 {
            initial_values.push(format!("initial_{}", i));
        }

        let result = CompleteResult::new(initial_values);

        // Add more values
        for i in 50..100 {
            result.add_value(format!("added_{}", i));
        }

        result.set_has_more();
        result.set_total(500);

        let wrapper = CompleteResultWrapper::new(result);
        let json = CompleteResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["completion"]["values"].as_array().unwrap().len(), 100);
        assert_eq!(parsed["completion"]["values"][0], "initial_0");
        assert_eq!(parsed["completion"]["values"][49], "initial_49");
        assert_eq!(parsed["completion"]["values"][50], "added_50");
        assert_eq!(parsed["completion"]["values"][99], "added_99");
        assert_eq!(parsed["completion"]["hasMore"], true);
        assert_eq!(parsed["completion"]["total"], 500);
    }
}