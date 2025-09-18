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
                "protocolVersion": "0.1.0", // MCP protocol version
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
