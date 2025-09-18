use std::cell::RefCell;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    GuestListResourceTemplatesResult,
    ListResourceTemplatesResult as ListResourceTemplatesResultWrapper,
    ResourceTemplateOptions,
};
use super::helpers::build_annotations;

pub struct ListResourceTemplatesResult {
    internal: RefCell<Value>,
}

impl GuestListResourceTemplatesResult for ListResourceTemplatesResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "resourceTemplates": [],
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

    fn add_resource_template(&self, name: String, uri_template: String, options: Option<ResourceTemplateOptions>) {
        let mut template = json!({
            "name": name,
            "uriTemplate": uri_template,
        });

        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                let mut meta_obj = json!({});
                for (key, value) in meta {
                    meta_obj[key] = json!(value);
                }
                template["_meta"] = meta_obj;
            }
            if let Some(annotations) = opts.annotations {
                template["annotations"] = build_annotations(annotations);
            }
            if let Some(description) = opts.description {
                template["description"] = json!(description);
            }
            if let Some(mime_type) = opts.mime_type {
                template["mimeType"] = json!(mime_type);
            }
            if let Some(title) = opts.title {
                template["title"] = json!(title);
            }
        }

        self.internal.borrow_mut()["resourceTemplates"]
            .as_array_mut()
            .expect("resourceTemplates should be an array")
            .push(template);
    }

    fn finish_json(this: ListResourceTemplatesResultWrapper) -> Result<String, ()> {
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
    fn test_list_resource_templates_result_empty() {
        let result = ListResourceTemplatesResult::new();
        let wrapper = ListResourceTemplatesResultWrapper::new(result);
        let json = ListResourceTemplatesResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, json!({
            "resourceTemplates": []
        }));
    }

    #[test]
    fn test_list_resource_templates_result_with_templates() {
        let result = ListResourceTemplatesResult::new();

        // Simple template
        result.add_resource_template(
            "simple".to_string(),
            "file:///{path}".to_string(),
            None
        );

        // Template with all options
        let template_options = ResourceTemplateOptions {
            meta: Some(vec![("version".to_string(), "1.0".to_string())]),
            annotations: Some(Annotations {
                audience: Some(vec![Role::User, Role::Assistant]),
                priority: Some(0.5),
                last_modified: Some("2024-01-01T00:00:00Z".to_string()),
            }),
            description: Some("Template for user files".to_string()),
            mime_type: Some("text/plain".to_string()),
            title: Some("User Files".to_string()),
        };

        result.add_resource_template(
            "user-file".to_string(),
            "file:///users/{userId}/files/{fileId}".to_string(),
            Some(template_options)
        );

        result.set_next_cursor("tmpl_cursor".to_string());

        let wrapper = ListResourceTemplatesResultWrapper::new(result);
        let json = ListResourceTemplatesResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["resourceTemplates"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["resourceTemplates"][0]["name"], "simple");
        assert_eq!(parsed["resourceTemplates"][1]["name"], "user-file");
        assert_eq!(parsed["resourceTemplates"][1]["uriTemplate"], "file:///users/{userId}/files/{fileId}");
        assert_eq!(parsed["resourceTemplates"][1]["description"], "Template for user files");
        assert_eq!(parsed["resourceTemplates"][1]["annotations"]["priority"], 0.5);
        assert_eq!(parsed["nextCursor"], "tmpl_cursor");
    }
}