use std::cell::RefCell;
use std::collections::HashMap;
use serde_json::{json, Value};
use crate::bindings::exports::wasmcp::mcp::types::*;

// Type aliases for the wrapper types from bindings
type ListToolsResultWrapper = crate::bindings::exports::wasmcp::mcp::types::ListToolsResult;
type InitializeResultWrapper = crate::bindings::exports::wasmcp::mcp::types::InitializeResult;
type CallToolResultWrapper = crate::bindings::exports::wasmcp::mcp::types::CallToolResult;
type ListResourcesResultWrapper = crate::bindings::exports::wasmcp::mcp::types::ListResourcesResult;
type ListResourceTemplatesResultWrapper = crate::bindings::exports::wasmcp::mcp::types::ListResourceTemplatesResult;
type ReadResourceResultWrapper = crate::bindings::exports::wasmcp::mcp::types::ReadResourceResult;
type ListPromptsResultWrapper = crate::bindings::exports::wasmcp::mcp::types::ListPromptsResult;
type GetPromptResultWrapper = crate::bindings::exports::wasmcp::mcp::types::GetPromptResult;
type CompleteResultWrapper = crate::bindings::exports::wasmcp::mcp::types::CompleteResult;

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

    fn finish_json(this: ListToolsResultWrapper) -> Result<String, ()> {
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

// ==========================================
// InitializeResult - Server initialization
// ==========================================

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
        let existing_caps = internal["capabilities"].as_object_mut().unwrap();
        let new_caps = build_capabilities(capabilities);

        for (key, value) in new_caps.as_object().unwrap() {
            existing_caps[key] = value.clone();
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

// ======================================
// CallToolResult - Tool execution result
// ======================================

pub struct CallToolResult {
    internal: RefCell<Value>,
}

impl GuestCallToolResult for CallToolResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "content": [],
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

    fn add_text(&self, text: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "text",
            "text": text,
        });

        apply_content_options(&mut content, options);

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn add_image(&self, mime_type: String, data: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "image",
            "mimeType": mime_type,
            "data": data,
        });

        apply_content_options(&mut content, options);

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn add_audio(&self, mime_type: String, data: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "audio",
            "mimeType": mime_type,
            "data": data,
        });

        apply_content_options(&mut content, options);

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn add_resource_link(&self, name: String, uri: String, options: Option<ResourceOptions>) {
        let mut content = json!({
            "type": "resource_link",
            "name": name,
            "uri": uri,
        });

        apply_resource_options(&mut content, options);

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn add_text_resource(&self, uri: String, text: String, options: Option<ResourceContentsOptions>) {
        let mut resource = json!({
            "uri": uri,
            "text": text,
        });

        apply_resource_contents_options(&mut resource, options);

        let content = json!({
            "type": "resource",
            "resource": resource,
        });

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn add_blob_resource(&self, uri: String, blob: String, options: Option<ResourceContentsOptions>) {
        let mut resource = json!({
            "uri": uri,
            "blob": blob,
        });

        apply_resource_contents_options(&mut resource, options);

        let content = json!({
            "type": "resource",
            "resource": resource,
        });

        self.internal.borrow_mut()["content"]
            .as_array_mut()
            .expect("content should be an array")
            .push(content);
    }

    fn set_error(&self) {
        self.internal.borrow_mut()["isError"] = json!(true);
    }

    fn set_structured_content(&self, content: String) {
        let parsed = serde_json::from_str::<Value>(&content)
            .unwrap_or_else(|_| json!(content));
        self.internal.borrow_mut()["structuredContent"] = parsed;
    }

    fn finish_json(this: CallToolResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

// ==========================================
// ListResourcesResult - List resources
// ==========================================

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

// ===================================================
// ListResourceTemplatesResult - List resource templates
// ===================================================

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

// ==========================================
// ReadResourceResult - Read resource contents
// ==========================================

pub struct ReadResourceResult {
    internal: RefCell<Value>,
}

impl GuestReadResourceResult for ReadResourceResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "contents": [],
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

    fn add_text_resource(&self, uri: String, text: String, options: Option<ResourceContentsOptions>) {
        let mut contents = json!({
            "uri": uri,
            "text": text,
        });

        apply_resource_contents_options(&mut contents, options);

        self.internal.borrow_mut()["contents"]
            .as_array_mut()
            .expect("contents should be an array")
            .push(contents);
    }

    fn add_blob_resource(&self, uri: String, blob: String, options: Option<ResourceContentsOptions>) {
        let mut contents = json!({
            "uri": uri,
            "blob": blob,
        });

        apply_resource_contents_options(&mut contents, options);

        self.internal.borrow_mut()["contents"]
            .as_array_mut()
            .expect("contents should be an array")
            .push(contents);
    }

    fn finish_json(this: ReadResourceResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

// ==========================================
// ListPromptsResult - List prompts
// ==========================================

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

// ==========================================
// GetPromptResult - Get prompt messages
// ==========================================

pub struct GetPromptResult {
    internal: RefCell<Value>,
}

impl GuestGetPromptResult for GetPromptResult {
    fn new() -> Self {
        Self {
            internal: RefCell::new(json!({
                "messages": [],
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

    fn set_description(&self, description: String) {
        self.internal.borrow_mut()["description"] = json!(description);
    }

    fn add_text_message(&self, role: Role, text: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "text",
            "text": text,
        });

        apply_content_options(&mut content, options);

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn add_image_message(&self, role: Role, mime_type: String, data: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "image",
            "mimeType": mime_type,
            "data": data,
        });

        apply_content_options(&mut content, options);

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn add_audio_message(&self, role: Role, mime_type: String, data: String, options: Option<ContentOptions>) {
        let mut content = json!({
            "type": "audio",
            "mimeType": mime_type,
            "data": data,
        });

        apply_content_options(&mut content, options);

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn add_resource_link_message(&self, role: Role, name: String, uri: String, options: Option<ResourceOptions>) {
        let mut content = json!({
            "type": "resource_link",
            "name": name,
            "uri": uri,
        });

        apply_resource_options(&mut content, options);

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn add_text_resource_message(&self, role: Role, uri: String, text: String, options: Option<ResourceContentsOptions>) {
        let mut resource = json!({
            "uri": uri,
            "text": text,
        });

        apply_resource_contents_options(&mut resource, options);

        let content = json!({
            "type": "resource",
            "resource": resource,
        });

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn add_blob_resource_message(&self, role: Role, uri: String, blob: String, options: Option<ResourceContentsOptions>) {
        let mut resource = json!({
            "uri": uri,
            "blob": blob,
        });

        apply_resource_contents_options(&mut resource, options);

        let content = json!({
            "type": "resource",
            "resource": resource,
        });

        let message = json!({
            "role": role_to_string(role),
            "content": content,
        });

        self.internal.borrow_mut()["messages"]
            .as_array_mut()
            .expect("messages should be an array")
            .push(message);
    }

    fn finish_json(this: GetPromptResultWrapper) -> Result<String, ()> {
        let inner = this.into_inner::<Self>();
        Ok(inner.internal.into_inner().to_string())
    }
}

// ==========================================
// CompleteResult - Completion suggestions
// ==========================================

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

// ==========================================
// Context - Request context resource
// ==========================================

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
    pub fn new(request_id: String, client_id: Option<String>, session_id: Option<String>) -> Self {
        Self {
            request_id,
            client_id,
            session_id,
            state: RefCell::new(HashMap::new()),
        }
    }
}

// ==========================================
// Helper functions
// ==========================================

fn role_to_string(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

fn apply_content_options(content: &mut Value, options: Option<ContentOptions>) {
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

fn apply_resource_options(resource: &mut Value, options: Option<ResourceOptions>) {
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

fn apply_resource_contents_options(contents: &mut Value, options: Option<ResourceContentsOptions>) {
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

fn build_annotations(annotations: Annotations) -> Value {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ==========================================
    // ListToolsResult Tests
    // ==========================================

    #[test]
    fn test_list_tools_result_empty() {
        let result = ListToolsResult::new();
        let wrapper = ListToolsResultWrapper::new(result);
        let json = ListToolsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed, json!({
            "tools": []
        }));
    }

    #[test]
    fn test_list_tools_result_with_tools() {
        let result = ListToolsResult::new();

        // Add a simple tool
        result.add_tool(
            "get_weather".to_string(),
            r#"{"type": "object", "properties": {"city": {"type": "string"}}, "required": ["city"]}"#.to_string(),
            None
        );

        // Add a tool with all options
        let options = ToolOptions {
            meta: Some(vec![("version".to_string(), "1.0".to_string())]),
            annotations: Some(ToolAnnotations {
                hints: ToolHints::READ_ONLY | ToolHints::IDEMPOTENT,
                title: Some("Calculator Tool".to_string()),
            }),
            description: Some("Performs calculations".to_string()),
            output_schema: Some(r#"{"type": "number"}"#.to_string()),
            title: Some("Calculator".to_string()),
        };

        result.add_tool(
            "calculate".to_string(),
            r#"{"type": "object"}"#.to_string(),
            Some(options)
        );

        result.set_next_cursor("cursor123".to_string());
        result.add_meta("request_id".to_string(), "req123".to_string()).unwrap();

        let wrapper = ListToolsResultWrapper::new(result);
        let json = ListToolsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["tools"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["tools"][0]["name"], "get_weather");
        assert_eq!(parsed["tools"][1]["name"], "calculate");
        assert_eq!(parsed["tools"][1]["description"], "Performs calculations");
        assert_eq!(parsed["tools"][1]["annotations"]["readOnlyHint"], true);
        assert_eq!(parsed["tools"][1]["annotations"]["idempotentHint"], true);
        assert_eq!(parsed["tools"][1]["annotations"]["destructiveHint"], false);
        assert_eq!(parsed["nextCursor"], "cursor123");
        assert_eq!(parsed["_meta"]["request_id"], "req123");
    }

    // ==========================================
    // InitializeResult Tests
    // ==========================================

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

        assert_eq!(parsed["protocolVersion"], "0.1.0");
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

    // ==========================================
    // CallToolResult Tests
    // ==========================================

    #[test]
    fn test_call_tool_result_text_content() {
        let result = CallToolResult::new();

        result.add_text("Hello, world!".to_string(), None);

        let content_options = ContentOptions {
            meta: Some(vec![("key".to_string(), "value".to_string())]),
            annotations: Some(Annotations {
                audience: Some(vec![Role::User]),
                priority: Some(0.8),
                last_modified: Some("2024-01-01T00:00:00Z".to_string()),
            }),
        };
        result.add_text("Annotated text".to_string(), Some(content_options));

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["content"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["content"][0]["type"], "text");
        assert_eq!(parsed["content"][0]["text"], "Hello, world!");
        assert_eq!(parsed["content"][1]["annotations"]["priority"], 0.8);
        assert_eq!(parsed["content"][1]["annotations"]["audience"][0], "user");
    }

    #[test]
    fn test_call_tool_result_mixed_content() {
        let result = CallToolResult::new();

        // Add different content types
        result.add_text("Some text".to_string(), None);
        result.add_image("image/png".to_string(), "base64data".to_string(), None);
        result.add_audio("audio/mp3".to_string(), "audiodata".to_string(), None);

        let resource_options = ResourceOptions {
            meta: None,
            annotations: None,
            description: Some("A helpful resource".to_string()),
            mime_type: Some("text/plain".to_string()),
            size: Some(1024),
            title: Some("Resource Title".to_string()),
        };
        result.add_resource_link("my-resource".to_string(), "file:///path/to/resource".to_string(), Some(resource_options));

        result.add_text_resource("file:///doc.txt".to_string(), "Document content".to_string(), None);
        result.add_blob_resource("file:///image.png".to_string(), "blobdata".to_string(), None);

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["content"].as_array().unwrap().len(), 6);
        assert_eq!(parsed["content"][1]["type"], "image");
        assert_eq!(parsed["content"][2]["type"], "audio");
        assert_eq!(parsed["content"][3]["type"], "resource_link");
        assert_eq!(parsed["content"][3]["description"], "A helpful resource");
        assert_eq!(parsed["content"][4]["type"], "resource");
        assert_eq!(parsed["content"][4]["resource"]["text"], "Document content");
    }

    #[test]
    fn test_call_tool_result_with_error() {
        let result = CallToolResult::new();

        result.add_text("Error occurred".to_string(), None);
        result.set_error();
        result.set_structured_content(r#"{"error": "Something went wrong"}"#.to_string());

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["isError"], true);
        assert_eq!(parsed["structuredContent"]["error"], "Something went wrong");
    }

    // ==========================================
    // Resource-related Results Tests
    // ==========================================

    #[test]
    fn test_list_resources_result() {
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

        let wrapper = ListResourcesResultWrapper::new(result);
        let json = ListResourcesResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["resources"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["resources"][0]["name"], "config.json");
        assert_eq!(parsed["resources"][1]["title"], "DB Schema");
        assert_eq!(parsed["resources"][1]["size"], 2048);
        assert_eq!(parsed["nextCursor"], "res_cursor");
    }

    #[test]
    fn test_list_resource_templates_result() {
        let result = ListResourceTemplatesResult::new();

        let template_options = ResourceTemplateOptions {
            meta: None,
            annotations: None,
            description: Some("Template for user files".to_string()),
            mime_type: Some("text/plain".to_string()),
            title: Some("User Files".to_string()),
        };

        result.add_resource_template(
            "user-file".to_string(),
            "file:///users/{userId}/files/{fileId}".to_string(),
            Some(template_options)
        );

        let wrapper = ListResourceTemplatesResultWrapper::new(result);
        let json = ListResourceTemplatesResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["resourceTemplates"][0]["name"], "user-file");
        assert_eq!(parsed["resourceTemplates"][0]["uriTemplate"], "file:///users/{userId}/files/{fileId}");
        assert_eq!(parsed["resourceTemplates"][0]["description"], "Template for user files");
    }

    #[test]
    fn test_read_resource_result() {
        let result = ReadResourceResult::new();

        result.add_text_resource(
            "file:///readme.txt".to_string(),
            "This is the readme content".to_string(),
            Some(ResourceContentsOptions {
                meta: None,
                mime_type: Some("text/plain".to_string()),
            })
        );

        result.add_blob_resource(
            "file:///image.jpg".to_string(),
            "base64imagedata".to_string(),
            Some(ResourceContentsOptions {
                meta: Some(vec![("size".to_string(), "1024".to_string())]),
                mime_type: Some("image/jpeg".to_string()),
            })
        );

        let wrapper = ReadResourceResultWrapper::new(result);
        let json = ReadResourceResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["contents"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["contents"][0]["text"], "This is the readme content");
        assert_eq!(parsed["contents"][0]["mimeType"], "text/plain");
        assert_eq!(parsed["contents"][1]["blob"], "base64imagedata");
        assert_eq!(parsed["contents"][1]["_meta"]["size"], "1024");
    }

    // ==========================================
    // Prompt-related Results Tests
    // ==========================================

    #[test]
    fn test_list_prompts_result() {
        let result = ListPromptsResult::new();

        result.add_prompt("simple-prompt".to_string(), None);

        let prompt_options = PromptOptions {
            meta: None,
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

        let wrapper = ListPromptsResultWrapper::new(result);
        let json = ListPromptsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["prompts"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["prompts"][1]["arguments"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["prompts"][1]["arguments"][0]["required"], true);
        assert_eq!(parsed["prompts"][1]["title"], "Content Generator");
    }

    #[test]
    fn test_get_prompt_result() {
        let result = GetPromptResult::new();

        result.set_description("A helpful prompt".to_string());

        // Add various message types
        result.add_text_message(Role::User, "Hello AI".to_string(), None);
        result.add_text_message(
            Role::Assistant,
            "Hello! How can I help?".to_string(),
            Some(ContentOptions {
                meta: None,
                annotations: Some(Annotations {
                    audience: Some(vec![Role::User, Role::Assistant]),
                    priority: None,
                    last_modified: None,
                }),
            })
        );

        result.add_image_message(
            Role::User,
            "image/png".to_string(),
            "imagedata".to_string(),
            None
        );

        result.add_resource_link_message(
            Role::Assistant,
            "doc".to_string(),
            "file:///doc.txt".to_string(),
            None
        );

        let wrapper = GetPromptResultWrapper::new(result);
        let json = GetPromptResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["description"], "A helpful prompt");
        assert_eq!(parsed["messages"].as_array().unwrap().len(), 4);
        assert_eq!(parsed["messages"][0]["role"], "user");
        assert_eq!(parsed["messages"][0]["content"]["text"], "Hello AI");
        assert_eq!(parsed["messages"][1]["content"]["annotations"]["audience"].as_array().unwrap().len(), 2);
    }

    // ==========================================
    // CompleteResult Tests
    // ==========================================

    #[test]
    fn test_complete_result_basic() {
        let initial_values = vec!["option1".to_string(), "option2".to_string()];
        let result = CompleteResult::new(initial_values);

        result.add_value("option3".to_string());

        let wrapper = CompleteResultWrapper::new(result);
        let json = CompleteResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["completion"]["values"].as_array().unwrap().len(), 3);
        assert_eq!(parsed["completion"]["values"][2], "option3");
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

    // ==========================================
    // Context Tests
    // ==========================================

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

    // ==========================================
    // Edge Cases and Special Characters Tests
    // ==========================================

    #[test]
    fn test_special_characters_in_strings() {
        let result = ListToolsResult::new();

        // Test with special characters and unicode
        result.add_tool(
            "test_tool_😀".to_string(),
            r#"{"type": "object", "description": "Tool with \"quotes\" and \n newlines"}"#.to_string(),
            Some(ToolOptions {
                meta: None,
                annotations: None,
                description: Some("Description with special chars: <>&\"'".to_string()),
                output_schema: None,
                title: Some("Title with emoji 🚀".to_string()),
            })
        );

        let wrapper = ListToolsResultWrapper::new(result);
        let json = ListToolsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["tools"][0]["name"], "test_tool_😀");
        assert_eq!(parsed["tools"][0]["title"], "Title with emoji 🚀");
        assert_eq!(parsed["tools"][0]["description"], "Description with special chars: <>&\"'");
    }

    #[test]
    fn test_empty_optional_fields() {
        let result = CallToolResult::new();

        // Test with all None options
        result.add_text("text".to_string(), Some(ContentOptions {
            meta: None,
            annotations: None,
        }));

        result.add_resource_link("res".to_string(), "uri".to_string(), Some(ResourceOptions {
            meta: None,
            annotations: None,
            description: None,
            mime_type: None,
            size: None,
            title: None,
        }));

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        // Verify that empty options don't add unnecessary fields
        assert_eq!(parsed["content"][0].get("_meta"), None);
        assert_eq!(parsed["content"][0].get("annotations"), None);
        assert_eq!(parsed["content"][1].get("description"), None);
    }

    #[test]
    fn test_large_data_handling() {
        let result = CallToolResult::new();

        // Create a large string (1MB)
        let large_text = "a".repeat(1024 * 1024);
        result.add_text(large_text.clone(), None);

        let wrapper = CallToolResultWrapper::new(result);
        let json = CallToolResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["content"][0]["text"].as_str().unwrap().len(), 1024 * 1024);
    }

    #[test]
    fn test_json_schema_validation() {
        let result = ListToolsResult::new();

        // Valid JSON schema
        let valid_schema = r#"{
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "number", "minimum": 0}
            },
            "required": ["name"]
        }"#;

        result.add_tool("valid_tool".to_string(), valid_schema.to_string(), None);

        // Invalid JSON (will fallback to string)
        result.add_tool("invalid_tool".to_string(), "not valid json".to_string(), None);

        let wrapper = ListToolsResultWrapper::new(result);
        let json = ListToolsResult::finish_json(wrapper).unwrap();
        let parsed: Value = serde_json::from_str(&json).unwrap();

        // Valid schema should be parsed as object
        assert!(parsed["tools"][0]["inputSchema"].is_object());
        assert_eq!(parsed["tools"][0]["inputSchema"]["type"], "object");

        // Invalid JSON should fallback to default object
        assert_eq!(parsed["tools"][1]["inputSchema"]["type"], "object");
    }
}
