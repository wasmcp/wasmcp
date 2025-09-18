use std::cell::RefCell;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    GuestListPromptsResult,
    ListPromptsResult as ListPromptsResultWrapper,
    PromptOptions,
};

pub struct ListPromptsResult {
    internal: RefCell<Value>,
}

impl GuestListPromptsResult for ListPromptsResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "prompts": [],
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

    fn add_prompt(&self, name: String, options: Option<PromptOptions>) {
        let mut prompt = json!({
            "name": name,
        });

        if let Some(opts) = options {
            if let Some(meta) = opts.meta {
                let mut meta_obj = json!({});
                for (key, value) in meta {
                    meta_obj[key] = json!(value);
                }
                prompt["_meta"] = meta_obj;
            }
            if let Some(arguments) = opts.arguments {
                let args: Vec<Value> = arguments.into_iter().map(|arg| json!({
                    "name": arg.name,
                    "description": arg.description,
                    "required": arg.required.unwrap_or(false),
                    "title": arg.title,
                })).collect();
                prompt["arguments"] = json!(args);
            }
            if let Some(description) = opts.description {
                prompt["description"] = json!(description);
            }
            if let Some(title) = opts.title {
                prompt["title"] = json!(title);
            }
        }

        self.internal.borrow_mut()["prompts"]
            .as_array_mut()
            .expect("prompts should be an array")
            .push(prompt);
    }

    fn finish_json(this: ListPromptsResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::exports::wasmcp::mcp::types::PromptArgument;
    use serde_json::json;

    #[test]
    fn test_list_prompts_result_empty() {
        let result = ListPromptsResult::new();
        let wrapper = ListPromptsResultWrapper::new(result);
        let json = ListPromptsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, json!({
            "prompts": []
        }));
    }

    #[test]
    fn test_list_prompts_result_simple() {
        let result = ListPromptsResult::new();

        result.add_prompt("simple-prompt".to_string(), None);
        result.add_prompt("another-prompt".to_string(), None);

        let wrapper = ListPromptsResultWrapper::new(result);
        let json = ListPromptsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["prompts"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["prompts"][0]["name"], "simple-prompt");
        assert_eq!(parsed["prompts"][1]["name"], "another-prompt");
    }

    #[test]
    fn test_list_prompts_result_with_options() {
        let result = ListPromptsResult::new();

        let prompt_options = PromptOptions {
            meta: Some(vec![("category".to_string(), "text-generation".to_string())]),
            arguments: Some(vec![
                PromptArgument {
                    name: "topic".to_string(),
                    description: Some("The topic to write about".to_string()),
                    required: Some(true),
                    title: Some("Topic".to_string()),
                },
                PromptArgument {
                    name: "style".to_string(),
                    description: Some("Writing style".to_string()),
                    required: Some(false),
                    title: None,
                },
            ]),
            description: Some("Generate content on a topic".to_string()),
            title: Some("Content Generator".to_string()),
        };

        result.add_prompt("content-generator".to_string(), Some(prompt_options));
        result.set_next_cursor("prompt_cursor".to_string());
        result.add_meta("version".to_string(), "1.0".to_string()).unwrap();

        let wrapper = ListPromptsResultWrapper::new(result);
        let json = ListPromptsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["prompts"][0]["name"], "content-generator");
        assert_eq!(parsed["prompts"][0]["title"], "Content Generator");
        assert_eq!(parsed["prompts"][0]["description"], "Generate content on a topic");
        assert_eq!(parsed["prompts"][0]["arguments"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["prompts"][0]["arguments"][0]["name"], "topic");
        assert_eq!(parsed["prompts"][0]["arguments"][0]["required"], true);
        assert_eq!(parsed["prompts"][0]["arguments"][1]["required"], false);
        assert_eq!(parsed["prompts"][0]["_meta"]["category"], "text-generation");
        assert_eq!(parsed["nextCursor"], "prompt_cursor");
        assert_eq!(parsed["_meta"]["version"], "1.0");
    }

    #[test]
    fn test_prompt_arguments_defaults() {
        let result = ListPromptsResult::new();

        let prompt_options = PromptOptions {
            meta: None,
            arguments: Some(vec![
                PromptArgument {
                    name: "arg1".to_string(),
                    description: None,
                    required: None, // Should default to false
                    title: None,
                },
            ]),
            description: None,
            title: None,
        };

        result.add_prompt("test".to_string(), Some(prompt_options));

        let wrapper = ListPromptsResultWrapper::new(result);
        let json = ListPromptsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["prompts"][0]["arguments"][0]["required"], false);
        assert_eq!(parsed["prompts"][0]["arguments"][0]["description"], Value::Null);
        assert_eq!(parsed["prompts"][0]["arguments"][0]["title"], Value::Null);
    }
}