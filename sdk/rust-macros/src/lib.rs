use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse::Parser, punctuated::Punctuated, ItemMod, Meta, Token};

const WIT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/wit");

/// Creates an MCP handler with the specified tools, resources, and prompts.
///
/// This macro generates all the necessary WebAssembly bindings and handler logic
/// with zero runtime overhead. It handles WIT file generation automatically,
/// so you don't need any local WIT files in your project.
///
/// # Example
///
/// ```rust,ignore
/// use wasmcp::mcp_handler;
///
/// #[mcp_handler(
///     tools(EchoTool, CalculatorTool),
///     resources(ConfigResource),
///     prompts(GreetingPrompt),
/// )]
/// mod handler {}
/// ```
#[proc_macro_attribute]
pub fn mcp_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let args = parse_macro_input!(args with parser);
    let _input_mod = parse_macro_input!(input as ItemMod);
    
    // Parse the arguments to extract tools, resources, and prompts
    let mut tools = Vec::new();
    let mut resources = Vec::new(); 
    let mut prompts = Vec::new();
    
    for arg in args {
        match arg {
            Meta::List(list) => {
                let name = list.path.get_ident().map(|i| i.to_string());
                match name.as_deref() {
                    Some("tools") => {
                        // Parse the tokens inside tools(...)
                        let tokens = list.tokens.clone();
                        let parser = Punctuated::<syn::Path, Token![,]>::parse_terminated;
                        if let Ok(paths) = parser.parse(tokens.into()) {
                            for path in paths {
                                if let Some(ident) = path.get_ident() {
                                    tools.push(ident.clone());
                                }
                            }
                        }
                    }
                    Some("resources") => {
                        // Parse the tokens inside resources(...)
                        let tokens = list.tokens.clone();
                        let parser = Punctuated::<syn::Path, Token![,]>::parse_terminated;
                        if let Ok(paths) = parser.parse(tokens.into()) {
                            for path in paths {
                                if let Some(ident) = path.get_ident() {
                                    resources.push(ident.clone());
                                }
                            }
                        }
                    }
                    Some("prompts") => {
                        // Parse the tokens inside prompts(...)
                        let tokens = list.tokens.clone();
                        let parser = Punctuated::<syn::Path, Token![,]>::parse_terminated;
                        if let Ok(paths) = parser.parse(tokens.into()) {
                            for path in paths {
                                if let Some(ident) = path.get_ident() {
                                    prompts.push(ident.clone());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    
    // Generate the bindings and handler implementation
    let preamble = generate_preamble();
    let handler_impl = generate_handler_impl(&tools, &resources, &prompts);
    
    quote! {
        #preamble
        #handler_impl
    }.into()
}

fn generate_preamble() -> proc_macro2::TokenStream {
    // Create a string literal token from the WIT_PATH constant
    let wit_path = proc_macro2::Literal::string(WIT_PATH);
    quote! {
        #[allow(warnings)]
        mod __wasmcp_bindings {
            #![allow(missing_docs)]
            ::wasmcp::wit_bindgen::generate!({
                world: "mcp-handler",
                path: #wit_path,
                runtime_path: "::wit_bindgen_rt",
                generate_all,
            });
            pub use self::exports::wasmcp::mcp::handler;
        }
    }
}

fn generate_handler_impl(
    tools: &[syn::Ident],
    resources: &[syn::Ident],  
    prompts: &[syn::Ident],
) -> proc_macro2::TokenStream {
    // Generate tool handling code
    let tool_list = if tools.is_empty() {
        quote! { vec![] }
    } else {
        quote! {
            vec![
                #( 
                    __wasmcp_bindings::handler::Tool {
                        name: <#tools as ::wasmcp::ToolHandler>::NAME.to_string(),
                        description: <#tools as ::wasmcp::ToolHandler>::DESCRIPTION.to_string(),
                        input_schema: <#tools as ::wasmcp::ToolHandler>::input_schema().to_string(),
                    }
                ),*
            ]
        }
    };

    let tool_call = if tools.is_empty() {
        quote! {
            __wasmcp_bindings::handler::ToolResult::Error(__wasmcp_bindings::handler::Error {
                code: -32601,
                message: format!("Tool not found: {}", name),
                data: None,
            })
        }
    } else {
        quote! {
            match name.as_str() {
                #(
                    <#tools as ::wasmcp::ToolHandler>::NAME => {
                        // Call through ToolHandler trait - it handles both sync and async
                        match <#tools as ::wasmcp::ToolHandler>::execute(args_json) {
                            Ok(result) => __wasmcp_bindings::handler::ToolResult::Text(result),
                            Err(e) => __wasmcp_bindings::handler::ToolResult::Error(__wasmcp_bindings::handler::Error {
                                code: -32603,
                                message: e,
                                data: None,
                            }),
                        }
                    }
                )*
                _ => __wasmcp_bindings::handler::ToolResult::Error(__wasmcp_bindings::handler::Error {
                    code: -32601,
                    message: format!("Tool not found: {}", name),
                    data: None,
                }),
            }
        }
    };

    // Similar for resources
    let resource_list = if resources.is_empty() {
        quote! { vec![] }
    } else {
        quote! {
            vec![
                #(
                    {
                        let info = <#resources as ::wasmcp::ResourceHandler>::list();
                        info.into_iter().map(|r| __wasmcp_bindings::handler::ResourceInfo {
                            uri: r.uri,
                            name: r.name,  
                            description: r.description,
                            mime_type: r.mime_type,
                        }).collect::<Vec<_>>()
                    }
                ),*
            ].into_iter().flatten().collect()
        }
    };

    let resource_read = if resources.is_empty() {
        quote! {
            __wasmcp_bindings::handler::ResourceResult::Error(__wasmcp_bindings::handler::Error {
                code: -32601,
                message: format!("Resource not found: {}", uri),
                data: None,
            })
        }
    } else {
        quote! {
            #(
                if let Ok(contents) = <#resources as ::wasmcp::ResourceHandler>::read(&uri) {
                    return __wasmcp_bindings::handler::ResourceResult::Contents(__wasmcp_bindings::handler::ResourceContents {
                        uri: contents.uri,
                        mime_type: contents.mime_type,
                        text: contents.text,
                        blob: contents.blob,
                    });
                }
            )*
            __wasmcp_bindings::handler::ResourceResult::Error(__wasmcp_bindings::handler::Error {
                code: -32601,
                message: format!("Resource not found: {}", uri),
                data: None,
            })
        }
    };

    // Similar for prompts
    let prompt_list = if prompts.is_empty() {
        quote! { vec![] }
    } else {
        quote! {
            vec![
                #(
                    {
                        let prompt = <#prompts as ::wasmcp::PromptHandler>::describe();
                        __wasmcp_bindings::handler::Prompt {
                            name: prompt.name,
                            description: prompt.description,
                            arguments: prompt.arguments.into_iter().map(|a| __wasmcp_bindings::handler::PromptArgument {
                                name: a.name,
                                description: a.description,
                                required: a.required,
                            }).collect(),
                        }
                    }
                ),*
            ]
        }
    };

    let prompt_get = if prompts.is_empty() {
        quote! {
            __wasmcp_bindings::handler::PromptResult::Error(__wasmcp_bindings::handler::Error {
                code: -32601,
                message: format!("Prompt not found: {}", name),
                data: None,
            })
        }
    } else {
        quote! {
            match name.as_str() {
                #(
                    <#prompts as ::wasmcp::PromptHandler>::NAME => {
                        match <#prompts as ::wasmcp::PromptHandler>::get_messages(args_json) {
                            Ok(messages) => __wasmcp_bindings::handler::PromptResult::Messages(
                                messages.into_iter().map(|m| __wasmcp_bindings::handler::PromptMessage {
                                    role: match m.role {
                                        ::wasmcp::PromptRole::User => "user".to_string(),
                                        ::wasmcp::PromptRole::Assistant => "assistant".to_string(),
                                    },
                                    content: m.content,
                                }).collect()
                            ),
                            Err(e) => __wasmcp_bindings::handler::PromptResult::Error(__wasmcp_bindings::handler::Error {
                                code: -32603,
                                message: e,
                                data: None,
                            }),
                        }
                    }
                )*
                _ => __wasmcp_bindings::handler::PromptResult::Error(__wasmcp_bindings::handler::Error {
                    code: -32601,
                    message: format!("Prompt not found: {}", name),
                    data: None,
                }),
            }
        }
    };


    quote! {
        struct __WasmcpHandler;

        impl __wasmcp_bindings::handler::Guest for __WasmcpHandler {
            fn list_tools() -> Vec<__wasmcp_bindings::handler::Tool> {
                #tool_list
            }

            fn call_tool(name: String, arguments: String) -> __wasmcp_bindings::handler::ToolResult {
                let args_json: ::serde_json::Value = match ::serde_json::from_str(&arguments) {
                    Ok(v) => v,
                    Err(e) => {
                        return __wasmcp_bindings::handler::ToolResult::Error(__wasmcp_bindings::handler::Error {
                            code: -32700,
                            message: format!("Invalid JSON: {}", e),
                            data: None,
                        });
                    }
                };

                #tool_call
            }

            fn list_resources() -> Vec<__wasmcp_bindings::handler::ResourceInfo> {
                #resource_list
            }

            fn read_resource(uri: String) -> __wasmcp_bindings::handler::ResourceResult {
                #resource_read
            }

            fn list_prompts() -> Vec<__wasmcp_bindings::handler::Prompt> {
                #prompt_list
            }

            fn get_prompt(name: String, arguments: String) -> __wasmcp_bindings::handler::PromptResult {
                let args_json: ::serde_json::Value = match ::serde_json::from_str(&arguments) {
                    Ok(v) => v,
                    Err(e) => {
                        return __wasmcp_bindings::handler::PromptResult::Error(__wasmcp_bindings::handler::Error {
                            code: -32700,
                            message: format!("Invalid JSON: {}", e),
                            data: None,
                        });
                    }
                };
                
                #prompt_get
            }
        }

        __wasmcp_bindings::export!(__WasmcpHandler with_types_in __wasmcp_bindings);
    }
}