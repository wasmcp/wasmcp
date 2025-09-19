use std::cell::RefCell;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    GuestListResourcesResult,
    ListResourcesResult as ListResourcesResultWrapper,
    ResourceOptions,
};
use super::helpers::apply_resource_options;

pub struct ListResourcesResult {
    internal: RefCell<Value>,
}

impl GuestListResourcesResult for ListResourcesResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "resources": [],
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

    fn set_next_cursor(&self, cursor: String) {
        self.internal.borrow_mut()["nextCursor"] = json!(cursor);
    }

    fn add_resource(&self, name: String, uri: String, options: Option<ResourceOptions>) {
        let mut resource = json!({
            "name": name,
            "uri": uri,
        });

        apply_resource_options(&mut resource, options);

        self.internal.borrow_mut()["resources"]
            .as_array_mut()
            .expect("resources should be an array")
            .push(resource);
    }

    fn finish_json(this: ListResourcesResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::exports::wasmcp::mcp::types::{Annotations, Role};
    use serde_json::json;

    #[test]
    fn test_list_resources_result_empty() {
        let result = ListResourcesResult::new();
        let wrapper = ListResourcesResultWrapper::new(result);
        let json = ListResourcesResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, json!({
            "resources": []
        }));
    }

    #[test]
    fn test_list_resources_result_with_resources() {
        let result = ListResourcesResult::new();

        result.add_resource("config.json".to_string(), "file:///config.json".to_string(), None);

        let resource_options = ResourceOptions {
            meta: Some(vec![("version".to_string(), "2".to_string())]),
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(1.0),
                last_modified: None,
            }),
            description: Some("Database schema".to_string()),
            mime_type: Some("application/sql".to_string()),
            size: Some(2048),
            title: Some("DB Schema".to_string()),
        };
        result.add_resource("schema.sql".to_string(), "file:///schema.sql".to_string(), Some(resource_options));

        result.set_next_cursor("res_cursor".to_string());
        result.add_meta("request_id".to_string(), "req456".to_string()).unwrap();

        let wrapper = ListResourcesResultWrapper::new(result);
        let json = ListResourcesResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["resources"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["resources"][0]["name"], "config.json");
        assert_eq!(parsed["resources"][1]["title"], "DB Schema");
        assert_eq!(parsed["resources"][1]["size"], 2048);
        assert_eq!(parsed["nextCursor"], "res_cursor");
        assert_eq!(parsed["_meta"]["request_id"], "req456");
    }
}