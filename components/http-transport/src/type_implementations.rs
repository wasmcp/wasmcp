use std::cell::RefCell;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    GuestListToolsResult,
    ListToolsResult as ListToolsResultResource,
    ToolOptions,
    ToolAnnotations,
    ToolHints,
};

pub struct ListToolsResult {
    internal: RefCell<Value>,
}

impl GuestListToolsResult for ListToolsResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "tools": [],
            })),
        }
    }

    fn add_meta(&self, key: String, value: String) -> Result<(), ()> {
        let mut internal = self.internal.borrow_mut();
        let meta = internal
            .as_object_mut()
            .ok_or(())?
            .entry("_meta")  // MCP spec uses _meta not meta
            .or_insert_with(|| json!({}));

        meta[key] = json!(value);
        Ok(())
    }

    fn set_next_cursor(&self, cursor: String) {
        self.internal.borrow_mut()["nextCursor"] = json!(cursor);
    }

    fn add_tool(&self, name: String, input_schema: String, options: Option<ToolOptions>) {
        // Parse inputSchema as JSON - it should be a valid JSON Schema object
        let parsed_schema = serde_json::from_str::<Value>(&input_schema)
            .unwrap_or_else(|_| json!({ "type": "object" }));

        let mut tool = json!({
            "name": name,
            "inputSchema": parsed_schema,
        });

        if let Some(opts) = options {
            // Process optional fields according to MCP spec
            if let Some(desc) = opts.description {
                tool["description"] = json!(desc);
            }
            if let Some(title) = opts.title {
                tool["title"] = json!(title);
            }
            if let Some(meta) = opts.meta {
                let mut meta_obj = json!({});
                for (key, value) in meta {
                    meta_obj[key] = json!(value);
                }
                tool["_meta"] = meta_obj;
            }
            if let Some(output_schema) = opts.output_schema {
                tool["outputSchema"] = serde_json::from_str::<Value>(&output_schema)
                    .unwrap_or_else(|_| json!({ "type": "object" }));
            }
            if let Some(annotations) = opts.annotations {
                tool["annotations"] = build_tool_annotations(annotations);
            }
        }

        self.internal.borrow_mut()["tools"]
            .as_array_mut()
            .expect("tools should be an array")
            .push(tool);
    }

    fn finish_json(this: ListToolsResultResource) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

fn build_tool_annotations(annotations: ToolAnnotations) -> Value {
    let mut ann = json!({});

    // Map WIT flags to MCP spec boolean hints
    let hints = annotations.hints;

    if hints.contains(ToolHints::READ_ONLY) {
        ann["readOnlyHint"] = json!(true);
    } else {
        ann["readOnlyHint"] = json!(false);
    }

    if hints.contains(ToolHints::DESTRUCTIVE) {
        ann["destructiveHint"] = json!(true);
    } else {
        ann["destructiveHint"] = json!(false);
    }

    if hints.contains(ToolHints::IDEMPOTENT) {
        ann["idempotentHint"] = json!(true);
    } else {
        ann["idempotentHint"] = json!(false);
    }

    if hints.contains(ToolHints::OPEN_WORLD) {
        ann["openWorldHint"] = json!(true);
    } else {
        ann["openWorldHint"] = json!(false);
    }

    if let Some(title) = annotations.title {
        ann["title"] = json!(title);
    }

    ann
}
