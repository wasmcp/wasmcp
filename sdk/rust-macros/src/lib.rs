use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemMod, ItemFn, Attribute, FnArg, Type, Pat};
use proc_macro2::TokenStream as TokenStream2;

/// Mark a module as containing MCP tools.
/// Functions in this module will be automatically registered as tools.
#[proc_macro_attribute]
pub fn mcp_tools(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let module = parse_macro_input!(input as ItemMod);
    
    // Collect all functions in the module as tools
    let mut tool_defs = Vec::<TokenStream2>::new();
    let mut tool_dispatches = Vec::<TokenStream2>::new();
    
    if let Some((_, items)) = &module.content {
        for item in items.iter() {
            if let syn::Item::Fn(func) = item {
                let (def, dispatch) = generate_tool_info(func);
                tool_defs.push(def);
                tool_dispatches.push(dispatch);
            }
        }
    }
    
    let module_name = &module.ident;
    
    // Generate the component and tool handler implementation
    quote! {
        #module
        
        pub struct Component;
        
        // Core implementation (always required)
        impl crate::bindings::exports::fastertools::mcp::core::Guest for Component {
            fn handle_initialize(_request: crate::bindings::fastertools::mcp::session::InitializeRequest) 
                -> Result<crate::bindings::fastertools::mcp::session::InitializeResponse, crate::bindings::fastertools::mcp::types::McpError> {
                Ok(crate::bindings::fastertools::mcp::session::InitializeResponse {
                    protocol_version: "2025-06-18".to_string(),
                    capabilities: crate::bindings::fastertools::mcp::session::ServerCapabilities {
                        tools: Some(crate::bindings::fastertools::mcp::session::ToolsCapability { 
                            list_changed: Some(false) 
                        }),
                        resources: None,
                        prompts: None,
                        experimental: None,
                        logging: None,
                        completions: None,
                    },
                    server_info: crate::bindings::fastertools::mcp::session::ImplementationInfo {
                        name: env!("CARGO_PKG_NAME").to_string(),
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        title: Some(env!("CARGO_PKG_NAME").to_string()),
                    },
                    instructions: None,
                    meta: None,
                })
            }
            
            fn handle_initialized() -> Result<(), crate::bindings::fastertools::mcp::types::McpError> {
                Ok(())
            }
            
            fn handle_ping() -> Result<(), crate::bindings::fastertools::mcp::types::McpError> {
                Ok(())
            }
            
            fn handle_shutdown() -> Result<(), crate::bindings::fastertools::mcp::types::McpError> {
                Ok(())
            }
        }
        
        // Tool handler implementation
        impl crate::bindings::exports::fastertools::mcp::tool_handler::Guest for Component {
            fn handle_list_tools(_request: crate::bindings::fastertools::mcp::tools::ListToolsRequest) 
                -> Result<crate::bindings::fastertools::mcp::tools::ListToolsResponse, crate::bindings::fastertools::mcp::types::McpError> {
                Ok(crate::bindings::fastertools::mcp::tools::ListToolsResponse {
                    tools: vec![#(#tool_defs),*],
                    next_cursor: None,
                    meta: None,
                })
            }
            
            fn handle_call_tool(request: crate::bindings::fastertools::mcp::tools::CallToolRequest) 
                -> Result<crate::bindings::fastertools::mcp::tools::ToolResult, crate::bindings::fastertools::mcp::types::McpError> {
                let args = if let Some(args_str) = &request.arguments {
                    serde_json::from_str(args_str)
                        .map_err(|e| crate::bindings::fastertools::mcp::types::McpError {
                            code: crate::bindings::fastertools::mcp::types::ErrorCode::InvalidParams,
                            message: format!("Invalid arguments: {}", e),
                            data: None,
                        })?
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                };
                
                use #module_name::*;
                
                match request.name.as_str() {
                    #(#tool_dispatches)*
                    _ => Err(crate::bindings::fastertools::mcp::types::McpError {
                        code: crate::bindings::fastertools::mcp::types::ErrorCode::ToolNotFound,
                        message: format!("Unknown tool: {}", request.name),
                        data: None,
                    })
                }
            }
        }
        
        // Empty stub implementations for unused capabilities
        impl crate::bindings::exports::fastertools::mcp::resource_handler::Guest for Component {
            fn handle_list_resources(_request: crate::bindings::fastertools::mcp::resources::ListResourcesRequest) 
                -> Result<crate::bindings::fastertools::mcp::resources::ListResourcesResponse, crate::bindings::fastertools::mcp::types::McpError> {
                Ok(crate::bindings::fastertools::mcp::resources::ListResourcesResponse {
                    resources: vec![],
                    next_cursor: None,
                    meta: None,
                })
            }
            
            fn handle_list_resource_templates(_request: crate::bindings::fastertools::mcp::resources::ListTemplatesRequest) 
                -> Result<crate::bindings::fastertools::mcp::resources::ListTemplatesResponse, crate::bindings::fastertools::mcp::types::McpError> {
                Ok(crate::bindings::fastertools::mcp::resources::ListTemplatesResponse {
                    templates: vec![],
                    next_cursor: None,
                    meta: None,
                })
            }
            
            fn handle_read_resource(_request: crate::bindings::fastertools::mcp::resources::ReadResourceRequest) 
                -> Result<crate::bindings::fastertools::mcp::resources::ReadResourceResponse, crate::bindings::fastertools::mcp::types::McpError> {
                Err(crate::bindings::fastertools::mcp::types::McpError {
                    code: crate::bindings::fastertools::mcp::types::ErrorCode::ResourceNotFound,
                    message: "This server does not provide resources".to_string(),
                    data: None,
                })
            }
            
            fn handle_subscribe_resource(_request: crate::bindings::fastertools::mcp::resources::SubscribeRequest) 
                -> Result<(), crate::bindings::fastertools::mcp::types::McpError> {
                Ok(())
            }
            
            fn handle_unsubscribe_resource(_request: crate::bindings::fastertools::mcp::resources::UnsubscribeRequest) 
                -> Result<(), crate::bindings::fastertools::mcp::types::McpError> {
                Ok(())
            }
        }
        
        // Prompt handler implementation (stub)
        impl crate::bindings::exports::fastertools::mcp::prompt_handler::Guest for Component {
            fn handle_list_prompts(_request: crate::bindings::fastertools::mcp::prompts::ListPromptsRequest) 
                -> Result<crate::bindings::fastertools::mcp::prompts::ListPromptsResponse, crate::bindings::fastertools::mcp::types::McpError> {
                Ok(crate::bindings::fastertools::mcp::prompts::ListPromptsResponse {
                    prompts: vec![],
                    next_cursor: None,
                    meta: None,
                })
            }
            
            fn handle_get_prompt(_request: crate::bindings::fastertools::mcp::prompts::GetPromptRequest) 
                -> Result<crate::bindings::fastertools::mcp::prompts::GetPromptResponse, crate::bindings::fastertools::mcp::types::McpError> {
                Err(crate::bindings::fastertools::mcp::types::McpError {
                    code: crate::bindings::fastertools::mcp::types::ErrorCode::PromptNotFound,
                    message: "This server does not provide prompts".to_string(),
                    data: None,
                })
            }
        }
    }.into()
}

/// Mark a module as containing MCP resources.
/// Functions in this module will be automatically registered as resources.
#[proc_macro_attribute]
pub fn mcp_resources(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let module = parse_macro_input!(input as ItemMod);
    
    // For now, just return the module unchanged
    // TODO: Implement resource collection and generation
    quote! {
        #module
        
        // Resources implementation would go here
    }.into()
}

/// Mark a module as containing MCP prompts.
/// Functions in this module will be automatically registered as prompts.
#[proc_macro_attribute]
pub fn mcp_prompts(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let module = parse_macro_input!(input as ItemMod);
    
    // For now, just return the module unchanged
    // TODO: Implement prompt collection and generation
    quote! {
        #module
        
        // Prompts implementation would go here
    }.into()
}

// Keep the original mcp_component for backwards compatibility
#[proc_macro_attribute]
pub fn mcp_component(_attr: TokenStream, input: TokenStream) -> TokenStream {
    mcp_tools(_attr, input)
}

fn generate_tool_info(func: &ItemFn) -> (TokenStream2, TokenStream2) {
    let name = func.sig.ident.to_string();
    let func_ident = &func.sig.ident;
    let doc = extract_doc_comment(&func.attrs);
    
    // Extract parameter information
    let mut params = Vec::new();
    let mut param_idents = Vec::new();
    for input in &func.sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(ident) = &*pat_type.pat {
                let param_name = ident.ident.to_string();
                let param_type = type_to_json_schema(&*pat_type.ty);
                params.push((param_name.clone(), param_type));
                param_idents.push(ident.ident.clone());
            }
        }
    }
    
    // Generate JSON schema for parameters
    let schema = generate_params_schema(&params);
    
    // Generate the tool definition
    let tool_def = quote! {
        crate::bindings::fastertools::mcp::tools::Tool {
            base: crate::bindings::fastertools::mcp::types::BaseMetadata {
                name: #name.to_string(),
                title: Some(#name.to_string()),
            },
            description: Some(#doc.to_string()),
            input_schema: #schema.to_string(),
            output_schema: None,
            annotations: None,
            meta: None,
        }
    };
    
    // Generate the dispatch case
    let param_extracts: Vec<TokenStream2> = params.iter().zip(&param_idents).map(|((name, _), ident)| {
        let ident_token = quote! { #ident };
        quote! {
            let #ident_token = args[#name].as_str()
                .ok_or_else(|| crate::bindings::fastertools::mcp::types::McpError {
                    code: crate::bindings::fastertools::mcp::types::ErrorCode::InvalidParams,
                    message: format!("Missing or invalid field: {}", #name),
                    data: None,
                })?
                .to_string();
        }
    }).collect();
    
    let is_async = func.sig.asyncness.is_some();
    let call_expr = if is_async {
        quote! { spin_executor::run(#func_ident(#(#param_idents),*)) }
    } else {
        quote! { #func_ident(#(#param_idents),*) }
    };
    
    let dispatch = quote! {
        #name => {
            #(#param_extracts)*
            
            match #call_expr {
                Ok(result) => Ok(crate::bindings::fastertools::mcp::tools::ToolResult {
                    content: vec![crate::bindings::fastertools::mcp::types::ContentBlock::Text(
                        crate::bindings::fastertools::mcp::types::TextContent {
                            text: result,
                            annotations: None,
                            meta: None,
                        }
                    )],
                    is_error: Some(false),
                    structured_content: None,
                    meta: None,
                }),
                Err(e) => Ok(crate::bindings::fastertools::mcp::tools::ToolResult {
                    content: vec![crate::bindings::fastertools::mcp::types::ContentBlock::Text(
                        crate::bindings::fastertools::mcp::types::TextContent {
                            text: format!("Error: {}", e),
                            annotations: None,
                            meta: None,
                        }
                    )],
                    is_error: Some(true),
                    structured_content: None,
                    meta: None,
                })
            }
        }
    };
    
    (tool_def, dispatch)
}

fn extract_doc_comment(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                attr.parse_args::<syn::LitStr>().ok().map(|lit| lit.value())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

fn type_to_json_schema(ty: &Type) -> String {
    match ty {
        Type::Path(path) => {
            if let Some(ident) = path.path.get_ident() {
                match ident.to_string().as_str() {
                    "i32" | "i64" | "u32" | "u64" => "\"type\": \"integer\"".to_string(),
                    "f32" | "f64" => "\"type\": \"number\"".to_string(),
                    "String" => "\"type\": \"string\"".to_string(),
                    "bool" => "\"type\": \"boolean\"".to_string(),
                    _ => "\"type\": \"object\"".to_string(),
                }
            } else {
                "\"type\": \"object\"".to_string()
            }
        }
        _ => "\"type\": \"object\"".to_string(),
    }
}

fn generate_params_schema(params: &[(String, String)]) -> String {
    if params.is_empty() {
        return "{}".to_string();
    }
    
    let properties: Vec<String> = params
        .iter()
        .map(|(name, schema)| format!("\"{}\":  {{{}}}", name, schema))
        .collect();
    
    let required: Vec<String> = params
        .iter()
        .map(|(name, _)| format!("\"{}\"", name))
        .collect();
    
    format!(
        "{{\"type\": \"object\", \"properties\": {{{}}}, \"required\": [{}]}}",
        properties.join(", "),
        required.join(", ")
    )
}

/// Mark a function as an MCP tool (for backwards compatibility).
#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// Mark a function as an MCP resource.
#[proc_macro_attribute]
pub fn resource(_attr: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// Mark a function as an MCP prompt.
#[proc_macro_attribute]
pub fn prompt(_attr: TokenStream, input: TokenStream) -> TokenStream {
    input
}