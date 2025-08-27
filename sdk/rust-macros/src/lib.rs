use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, ItemStruct, FnArg, Pat, Lit, Type};

const WIT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/wit");

/// The main entry point for an MCP server.
/// 
/// # Example
/// 
/// ```rust
/// use wasmcp::prelude::*;
/// 
/// #[mcp::main]
/// struct MyServer;
/// ```
/// 
/// With state:
/// ```rust
/// #[mcp::main]
/// struct MyServer {
///     #[mcp::state]
///     db: Database,
/// }
/// ```
#[proc_macro_attribute]
pub fn main(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_struct = parse_macro_input!(input as ItemStruct);
    let struct_name = &input_struct.ident;
    let struct_def = &input_struct;
    
    // Generate the component implementation
    let output = quote! {
        #struct_def
        
        // Thread-safe registration storage
        use std::sync::Mutex;
        static MCP_TOOLS: Mutex<Vec<wasmcp::runtime::ToolRegistration>> = Mutex::new(Vec::new());
        static MCP_RESOURCES: Mutex<Vec<wasmcp::runtime::ResourceRegistration>> = Mutex::new(Vec::new());
        static MCP_PROMPTS: Mutex<Vec<wasmcp::runtime::PromptRegistration>> = Mutex::new(Vec::new());
        
        // Registration functions called by tool/resource/prompt macros
        #[doc(hidden)]
        pub fn __mcp_register_tool(tool: wasmcp::runtime::ToolRegistration) {
            MCP_TOOLS.lock().unwrap().push(tool);
        }
        
        #[doc(hidden)]
        pub fn __mcp_register_resource(resource: wasmcp::runtime::ResourceRegistration) {
            MCP_RESOURCES.lock().unwrap().push(resource);
        }
        
        #[doc(hidden)]
        pub fn __mcp_register_prompt(prompt: wasmcp::runtime::PromptRegistration) {
            MCP_PROMPTS.lock().unwrap().push(prompt);
        }
        
        // Implement the Guest trait
        impl wasmcp::bindings::exports::mcp::protocol::handler::Guest for #struct_name {
            fn handle_initialize(
                _request: wasmcp::InitializeRequest
            ) -> Result<wasmcp::InitializeResponse, wasmcp::McpError> {
                use wasmcp::{InitializeResponse, ServerCapabilities, ImplementationInfo, ToolsCapability};
                
                Ok(InitializeResponse {
                    protocol_version: "1.0.0".to_string(),
                    capabilities: ServerCapabilities {
                        tools: if !MCP_TOOLS.lock().unwrap().is_empty() {
                            Some(ToolsCapability { list_changed: None })
                        } else {
                            None
                        },
                        resources: if !MCP_RESOURCES.lock().unwrap().is_empty() {
                            Some(wasmcp::ResourcesCapability { list_changed: None, subscribe: None })
                        } else {
                            None
                        },
                        prompts: if !MCP_PROMPTS.lock().unwrap().is_empty() {
                            Some(wasmcp::PromptsCapability { list_changed: None })
                        } else {
                            None
                        },
                        logging: None,
                        completions: None,
                        experimental: None,
                    },
                    server_info: ImplementationInfo {
                        name: env!("CARGO_PKG_NAME").to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        title: None,
                    },
                    instructions: None,
                    meta: None,
                })
            }
            
            fn handle_initialized() -> Result<(), wasmcp::McpError> {
                Ok(())
            }
            
            fn handle_shutdown() -> Result<(), wasmcp::McpError> {
                Ok(())
            }
            
            fn handle_ping() -> Result<(), wasmcp::McpError> {
                Ok(())
            }
            
            fn handle_list_tools(
                _request: wasmcp::ListToolsRequest
            ) -> Result<wasmcp::ListToolsResponse, wasmcp::McpError> {
                use wasmcp::{ListToolsResponse, McpTool, BaseMetadata};
                
                let tools = MCP_TOOLS.lock().unwrap()
                    .iter().map(|reg| {
                        McpTool {
                            base: BaseMetadata {
                                name: reg.name.clone(),
                                title: None,
                            },
                            description: Some(reg.description.clone()),
                            input_schema: reg.schema.clone(),
                            output_schema: None,
                            annotations: None,
                            meta: None,
                        }
                    }).collect();
                
                Ok(ListToolsResponse {
                    tools,
                    next_cursor: None,
                    meta: None,
                })
            }
            
            fn handle_call_tool(
                request: wasmcp::CallToolRequest
            ) -> Result<wasmcp::ToolResult, wasmcp::McpError> {
                use wasmcp::{ToolResult, ContentBlock, TextContent};
                
                let tools = MCP_TOOLS.lock().unwrap();
                
                for tool in tools {
                    if tool.name == request.name {
                        let args = request.arguments
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or(serde_json::Value::Null);
                        
                        match (tool.handler)(args) {
                            Ok(result) => {
                                return Ok(ToolResult {
                                    content: vec![ContentBlock::Text(TextContent {
                                        text: result.to_string(),
                                        annotations: None,
                                        meta: None,
                                    })],
                                    structured_content: None,
                                    is_error: Some(false),
                                    meta: None,
                                })
                            }
                            Err(e) => {
                                return Ok(ToolResult {
                                    content: vec![ContentBlock::Text(TextContent {
                                        text: e.to_string(),
                                        annotations: None,
                                        meta: None,
                                    })],
                                    structured_content: None,
                                    is_error: Some(true),
                                    meta: None,
                                })
                            }
                        }
                    }
                }
                
                Err(wasmcp::McpError {
                    code: wasmcp::ErrorCode::ToolNotFound,
                    message: format!("Tool '{}' not found", request.name),
                    data: None,
                })
            }
            
            fn handle_list_resources(
                _request: wasmcp::ListResourcesRequest
            ) -> Result<wasmcp::ListResourcesResponse, wasmcp::McpError> {
                use wasmcp::{ListResourcesResponse, McpResource, BaseMetadata};
                
                let resources = MCP_RESOURCES.lock().unwrap()
                    .iter().map(|reg| {
                        McpResource {
                            base: BaseMetadata {
                                name: reg.name.clone(),
                                title: None,
                            },
                            uri: reg.uri_pattern.clone(),
                            description: Some(reg.description.clone()),
                            mime_type: reg.mime_type.clone(),
                            size: None,
                            annotations: None,
                            meta: None,
                        }
                    }).collect();
                
                Ok(ListResourcesResponse {
                    resources,
                    next_cursor: None,
                    meta: None,
                })
            }
            
            fn handle_read_resource(
                request: wasmcp::ReadResourceRequest
            ) -> Result<wasmcp::ReadResourceResponse, wasmcp::McpError> {
                use wasmcp::bindings::mcp::protocol::types::{ResourceContents, TextResourceContents};
                
                let resources = MCP_RESOURCES.lock().unwrap();
                
                for resource in resources {
                    if wasmcp::runtime::uri_matches(&resource.uri_pattern, &request.uri) {
                        match (resource.handler)(request.uri.clone()) {
                            Ok(content) => {
                                return Ok(wasmcp::ReadResourceResponse {
                                    contents: vec![ResourceContents::Text(TextResourceContents {
                                        uri: request.uri,
                                        mime_type: resource.mime_type.clone(),
                                        text: content,
                                        meta: None,
                                    })],
                                    meta: None,
                                })
                            }
                            Err(e) => {
                                return Err(wasmcp::McpError {
                                    code: wasmcp::ErrorCode::InternalError,
                                    message: e.to_string(),
                                    data: None,
                                })
                            }
                        }
                    }
                }
                
                Err(wasmcp::McpError {
                    code: wasmcp::ErrorCode::ResourceNotFound,
                    message: format!("Resource '{}' not found", request.uri),
                    data: None,
                })
            }
            
            fn handle_list_prompts(
                _request: wasmcp::ListPromptsRequest
            ) -> Result<wasmcp::ListPromptsResponse, wasmcp::McpError> {
                use wasmcp::{ListPromptsResponse, McpPrompt, BaseMetadata};
                
                let prompts = MCP_PROMPTS.lock().unwrap()
                    .iter().map(|reg| {
                        McpPrompt {
                            base: BaseMetadata {
                                name: reg.name.clone(),
                                title: None,
                            },
                            description: Some(reg.description.clone()),
                            arguments: Vec::new(), // TODO: Add argument support
                            annotations: None,
                            meta: None,
                        }
                    }).collect();
                
                Ok(ListPromptsResponse {
                    prompts,
                    next_cursor: None,
                    meta: None,
                })
            }
            
            fn handle_get_prompt(
                request: wasmcp::GetPromptRequest
            ) -> Result<wasmcp::GetPromptResponse, wasmcp::McpError> {
                let prompts = MCP_PROMPTS.lock().unwrap();
                
                for prompt in prompts {
                    if prompt.name == request.name {
                        let args = request.arguments
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or(serde_json::Value::Null);
                        
                        match (prompt.handler)(args) {
                            Ok(messages) => {
                                return Ok(wasmcp::GetPromptResponse {
                                    messages,
                                    description: Some(prompt.description.clone()),
                                    meta: None,
                                })
                            }
                            Err(e) => {
                                return Err(wasmcp::McpError {
                                    code: wasmcp::ErrorCode::InternalError,
                                    message: e.to_string(),
                                    data: None,
                                })
                            }
                        }
                    }
                }
                
                Err(wasmcp::McpError {
                    code: wasmcp::ErrorCode::PromptNotFound,
                    message: format!("Prompt '{}' not found", request.name),
                    data: None,
                })
            }
        }
        
        // Export the component
        wasmcp::bindings::export!(#struct_name with_types_in wasmcp::bindings);
    };
    
    output.into()
}

/// Register a function as an MCP tool.
/// 
/// # Example
/// 
/// ```rust
/// #[mcp::tool]
/// /// Add two numbers together
/// fn add(
///     #[arg(description = "First number")]
///     a: i32,
///     #[arg(description = "Second number")]
///     b: i32
/// ) -> Result<i32> {
///     Ok(a + b)
/// }
/// ```
#[proc_macro_attribute]
pub fn tool(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    
    // Extract the doc comment as description
    let description = extract_doc_comment(&input_fn.attrs);
    
    // Generate JSON schema and handler from function signature
    let (schema, handler_body) = generate_tool_handler(&input_fn);
    
    // Create a unique static name for the initialization
    let init_fn_name = quote::format_ident!("__mcp_tool_init_{}", fn_name);
    
    let output = quote! {
        #input_fn
        
        // Auto-registration using constructor attribute
        #[doc(hidden)]
        #[used]
        #[cfg_attr(target_family = "wasm", link_section = ".init_array")]
        static #init_fn_name: extern "C" fn() = {
            extern "C" fn init() {
                let registration = wasmcp::runtime::ToolRegistration {
                    name: #fn_name_str.to_string(),
                    description: #description.to_string(),
                    schema: #schema.to_string(),
                    handler: Box::new(#handler_body),
                };
                __mcp_register_tool(registration);
            }
            init
        };
    };
    
    output.into()
}

/// Register a function as an MCP resource.
/// 
/// # Example
/// 
/// ```rust
/// #[mcp::resource("file://{path}")]
/// /// Read a file from the filesystem
/// fn read_file(path: String) -> Result<String> {
///     std::fs::read_to_string(path)
/// }
/// ```
#[proc_macro_attribute]
pub fn resource(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_name = &input_fn.sig.ident;
    
    // Parse URI pattern from args
    let uri_pattern = if !args.is_empty() {
        let lit_str = parse_macro_input!(args as syn::LitStr);
        lit_str.value()
    } else {
        format!("{}://{{id}}", fn_name)
    };
    
    // Extract the doc comment as description
    let description = extract_doc_comment(&input_fn.attrs);
    
    // Generate handler based on function signature
    let handler_body = generate_resource_handler(&input_fn, &uri_pattern);
    
    // Create a unique static name for the initialization
    let init_fn_name = quote::format_ident!("__mcp_resource_init_{}", fn_name);
    
    let output = quote! {
        #input_fn
        
        // Auto-registration using constructor attribute
        #[doc(hidden)]
        #[used]
        #[cfg_attr(target_family = "wasm", link_section = ".init_array")]
        static #init_fn_name: extern "C" fn() = {
            extern "C" fn init() {
                let registration = wasmcp::runtime::ResourceRegistration {
                    name: stringify!(#fn_name).to_string(),
                    uri_pattern: #uri_pattern.to_string(),
                    description: #description.to_string(),
                    mime_type: None,
                    handler: Box::new(#handler_body),
                };
                __mcp_register_resource(registration);
            }
            init
        };
    };
    
    output.into()
}

/// Register a function as an MCP prompt.
/// 
/// # Example
/// 
/// ```rust
/// #[mcp::prompt("code_review")]
/// /// Generate a code review prompt
/// fn code_review_prompt(
///     #[arg(description = "The code to review")]
///     code: String,
/// ) -> Prompt {
///     prompt! {
///         system: "You are a thorough code reviewer.",
///         user: "Review this code:\n\n{}", code
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn prompt(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(input as ItemFn);
    let fn_name = &input_fn.sig.ident;
    
    // Parse prompt name from args
    let prompt_name = if !args.is_empty() {
        let lit_str = parse_macro_input!(args as syn::LitStr);
        lit_str.value()
    } else {
        fn_name.to_string()
    };
    
    // Extract the doc comment as description
    let description = extract_doc_comment(&input_fn.attrs);
    
    // Generate handler based on function signature
    let handler_body = generate_prompt_handler(&input_fn);
    
    // Create a unique static name for the initialization
    let init_fn_name = quote::format_ident!("__mcp_prompt_init_{}", fn_name);
    
    let output = quote! {
        #input_fn
        
        // Auto-registration using constructor attribute
        #[doc(hidden)]
        #[used]
        #[cfg_attr(target_family = "wasm", link_section = ".init_array")]
        static #init_fn_name: extern "C" fn() = {
            extern "C" fn init() {
                let registration = wasmcp::runtime::PromptRegistration {
                    name: #prompt_name.to_string(),
                    description: #description.to_string(),
                    handler: Box::new(#handler_body),
                };
                __mcp_register_prompt(registration);
            }
            init
        };
    };
    
    output.into()
}

// Helper functions

fn extract_doc_comment(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(meta) = &attr.meta {
                    if let syn::Expr::Lit(expr_lit) = &meta.value {
                        if let Lit::Str(s) = &expr_lit.lit {
                            Some(s.value().trim().to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn generate_tool_handler(func: &ItemFn) -> (String, proc_macro2::TokenStream) {
    let fn_name = &func.sig.ident;
    let is_async = func.sig.asyncness.is_some();
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    let mut param_names = Vec::new();
    let mut param_extracts = Vec::new();
    
    // Analyze function parameters
    for arg in &func.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(ident) = &*pat_type.pat {
                let param_name = ident.ident.to_string();
                param_names.push(ident.ident.clone());
                required.push(param_name.clone());
                
                // Extract type info for schema
                let type_schema = type_to_json_schema(&pat_type.ty);
                properties.insert(param_name.clone(), type_schema);
                
                // Generate extraction code
                let param_ident = &ident.ident;
                let param_str = param_name.clone();
                param_extracts.push(quote! {
                    let #param_ident = args.get(#param_str)
                        .ok_or_else(|| format!("Missing parameter: {}", #param_str))?
                        .clone();
                    let #param_ident = serde_json::from_value(#param_ident)
                        .map_err(|e| format!("Invalid parameter '{}': {}", #param_str, e))?;
                });
            }
        }
    }
    
    // Generate the schema
    let schema = serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    }).to_string();
    
    // Generate the handler body - use spin_executor::run for async
    let call_expr = if is_async {
        quote! { #fn_name(#(#param_names),*) }
    } else {
        quote! { async move { #fn_name(#(#param_names),*) } }
    };
    
    let handler = if param_names.is_empty() {
        if is_async {
            quote! {
                |_args| {
                    let result = ::spin_sdk::executor::run(#fn_name());
                    match result {
                        Ok(val) => serde_json::to_value(val)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
                        Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
                    }
                }
            }
        } else {
            quote! {
                |_args| {
                    let result = #fn_name();
                    match result {
                        Ok(val) => serde_json::to_value(val)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
                        Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
                    }
                }
            }
        }
    } else {
        if is_async {
            quote! {
                |args| {
                    let args = args.as_object()
                        .ok_or_else(|| "Arguments must be an object")?;
                    
                    #(#param_extracts)*
                    
                    let result = ::spin_sdk::executor::run(#fn_name(#(#param_names),*));
                    match result {
                        Ok(val) => serde_json::to_value(val)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
                        Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
                    }
                }
            }
        } else {
            quote! {
                |args| {
                    let args = args.as_object()
                        .ok_or_else(|| "Arguments must be an object")?;
                    
                    #(#param_extracts)*
                    
                    let result = #fn_name(#(#param_names),*);
                    match result {
                        Ok(val) => serde_json::to_value(val)
                            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>),
                        Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
                    }
                }
            }
        }
    };
    
    (schema, handler)
}

fn generate_resource_handler(func: &ItemFn, _uri_pattern: &str) -> proc_macro2::TokenStream {
    let fn_name = &func.sig.ident;
    
    // For now, simple URI extraction - can be enhanced later
    quote! {
        |uri| {
            // TODO: Extract parameters from URI based on pattern
            let result = #fn_name(uri);
            match result {
                Ok(content) => Ok(content),
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
            }
        }
    }
}

fn generate_prompt_handler(func: &ItemFn) -> proc_macro2::TokenStream {
    let fn_name = &func.sig.ident;
    let mut param_names = Vec::new();
    let mut param_extracts = Vec::new();
    
    // Analyze function parameters
    for arg in &func.sig.inputs {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(ident) = &*pat_type.pat {
                let param_name = ident.ident.to_string();
                param_names.push(ident.ident.clone());
                
                let param_ident = &ident.ident;
                let param_str = param_name.clone();
                param_extracts.push(quote! {
                    let #param_ident = args.get(#param_str)
                        .ok_or_else(|| format!("Missing parameter: {}", #param_str))?
                        .clone();
                    let #param_ident = serde_json::from_value(#param_ident)
                        .map_err(|e| format!("Invalid parameter '{}': {}", #param_str, e))?;
                });
            }
        }
    }
    
    if param_names.is_empty() {
        quote! {
            |_args| {
                let prompt = #fn_name();
                Ok(prompt.into_messages())
            }
        }
    } else {
        quote! {
            |args| {
                let args = args.as_object()
                    .ok_or_else(|| "Arguments must be an object")?;
                
                #(#param_extracts)*
                
                let prompt = #fn_name(#(#param_names),*);
                Ok(prompt.into_messages())
            }
        }
    }
}

fn type_to_json_schema(ty: &syn::Type) -> serde_json::Value {
    // Basic type mapping - can be enhanced
    match ty {
        Type::Path(path) => {
            if let Some(segment) = path.path.segments.last() {
                match segment.ident.to_string().as_str() {
                    "String" | "str" => serde_json::json!({ "type": "string" }),
                    "i32" | "i64" | "i128" | "isize" => serde_json::json!({ "type": "integer" }),
                    "u32" | "u64" | "u128" | "usize" => serde_json::json!({ "type": "integer", "minimum": 0 }),
                    "f32" | "f64" => serde_json::json!({ "type": "number" }),
                    "bool" => serde_json::json!({ "type": "boolean" }),
                    "Vec" => serde_json::json!({ "type": "array" }),
                    _ => serde_json::json!({ "type": "object" }),
                }
            } else {
                serde_json::json!({ "type": "object" })
            }
        }
        _ => serde_json::json!({ "type": "object" }),
    }
}