use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::{
    Annotations, ContentOptions, ResourceOptions, ResourceContentsOptions,
    Role, ToolAnnotations, ToolHints,
};

pub fn role_to_string(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

pub fn apply_content_options(content: &mut Value, options: Option<ContentOptions>) {
    if let Some(opts) = options {
        if let Some(meta) = opts.meta {
            let mut meta_obj = json!({});
            for (key, value) in meta {
                meta_obj[key] = json!(value);
            }
            content["_meta"] = meta_obj;
        }
        if let Some(annotations) = opts.annotations {
            content["annotations"] = build_annotations(annotations);
        }
    }
}

pub fn apply_resource_options(resource: &mut Value, options: Option<ResourceOptions>) {
    if let Some(opts) = options {
        if let Some(meta) = opts.meta {
            let mut meta_obj = json!({});
            for (key, value) in meta {
                meta_obj[key] = json!(value);
            }
            resource["_meta"] = meta_obj;
        }
        if let Some(annotations) = opts.annotations {
            resource["annotations"] = build_annotations(annotations);
        }
        if let Some(description) = opts.description {
            resource["description"] = json!(description);
        }
        if let Some(mime_type) = opts.mime_type {
            resource["mimeType"] = json!(mime_type);
        }
        if let Some(size) = opts.size {
            resource["size"] = json!(size);
        }
        if let Some(title) = opts.title {
            resource["title"] = json!(title);
        }
    }
}

pub fn apply_resource_contents_options(contents: &mut Value, options: Option<ResourceContentsOptions>) {
    if let Some(opts) = options {
        if let Some(meta) = opts.meta {
            let mut meta_obj = json!({});
            for (key, value) in meta {
                meta_obj[key] = json!(value);
            }
            contents["_meta"] = meta_obj;
        }
        if let Some(mime_type) = opts.mime_type {
            contents["mimeType"] = json!(mime_type);
        }
    }
}

pub fn build_annotations(annotations: Annotations) -> Value {
    let mut ann = json!({});

    if let Some(audience) = annotations.audience {
        let audience_strs: Vec<&str> = audience.iter().map(|r| role_to_string(*r)).collect();
        ann["audience"] = json!(audience_strs);
    }
    if let Some(priority) = annotations.priority {
        ann["priority"] = json!(priority);
    }
    if let Some(last_modified) = annotations.last_modified {
        ann["lastModified"] = json!(last_modified);
    }

    ann
}

pub fn build_tool_annotations(annotations: ToolAnnotations) -> Value {
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