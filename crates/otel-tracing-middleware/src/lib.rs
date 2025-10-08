//! OpenTelemetry tracing middleware for MCP servers.
//!
//! This middleware instruments MCP requests with distributed tracing spans,
//! capturing request metadata and timing information for observability.
//!
//! # Architecture
//!
//! This component is completely decoupled from the OTEL SDK implementation:
//! - Depends ONLY on WIT interface definitions (wasi:otel-sdk/trace, etc.)
//! - The actual SDK implementation is provided at composition time
//! - When OTEL SDK moves to an independent project, this middleware remains unchanged
//!
//! # Composition
//!
//! The middleware is composed into the MCP request chain:
//! ```text
//! Transport → Tracing Middleware → Handler(s) → Initialize
//! ```
//!
//! At composition time, the OTEL SDK component satisfies the trace/context imports.

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        world: "stuff",
        generate_all,
    });
}

use bindings::exports::wasmcp::mcp::incoming_handler::{Guest as IncomingHandlerGuest, OutputStream, Request};
use bindings::exports::wasmcp::mcp::otel_instrumentation::Guest as OtelInstrumentationGuest;
use bindings::wasmcp::mcp::incoming_handler as next_handler;
use bindings::wasmcp::mcp::otel_config;
use bindings::wasi::otel_providers::common_providers;
use bindings::wasi::otel_sdk::common::{Attribute, AttributeValue};
use bindings::wasi::otel_sdk::context;
use bindings::wasi::otel_sdk::http_transport;
use bindings::wasi::otel_sdk::trace::{
    Span, SpanData, SpanKind, SpanStatus, TraceLimitsConfig, TracerProvider, SamplerConfig, TraceExporter,
};
use bindings::wasi::otel_sdk::transport::ExporterTransport;

use std::cell::RefCell;
use std::sync::OnceLock;

pub struct Component;

/// Global state for tracer provider and transport
/// Initialized lazily on first request using OnceLock for thread-safe initialization
/// Stores Option<T> to handle disabled providers gracefully
static TRACER_PROVIDER: OnceLock<Option<TracerProvider>> = OnceLock::new();
static TRACER_ID: OnceLock<Option<u32>> = OnceLock::new();
static TRANSPORT: OnceLock<Option<ExporterTransport>> = OnceLock::new();

/// Child spans collected during request processing
thread_local! {
    static CHILD_SPANS: RefCell<Vec<Span>> = RefCell::new(Vec::new());
    static NEXT_SPAN_ID: RefCell<u64> = RefCell::new(1);
}

impl Component {
    /// Initialize OTEL provider and transport lazily on first request
    fn ensure_initialized() {
        // Initialize all components together
        TRANSPORT.get_or_init(|| {
            // Get provider configuration from user component
            let provider_config = otel_config::get_config();

            // Convert provider config to HTTP config using wasi:otel-providers
            let http_config = match common_providers::to_http_config(&provider_config) {
                Ok(cfg) => cfg,
                Err(_) => {
                    // Provider is disabled or invalid config
                    return None;
                }
            };

            // Create HTTP transport
            http_transport::create_http_transport(&http_config).ok()
        });

        // Initialize tracer provider (only if we have a transport)
        TRACER_PROVIDER.get_or_init(|| {
            if TRANSPORT.get().and_then(|t| t.as_ref()).is_none() {
                return None;
            }

            Some(TracerProvider::new(
                Some(SamplerConfig::AlwaysOn),
                Some(TraceLimitsConfig {
                    max_attributes_per_span: 128,
                    max_events_per_span: 128,
                    max_links_per_span: 128,
                    attribute_value_length_limit: 4096,
                    span_attribute_count_limit: 128,
                }),
                None, // Resource will be set by application
            ))
        });

        // Initialize tracer ID (only if we have a provider)
        TRACER_ID.get_or_init(|| {
            TRACER_PROVIDER.get()
                .and_then(|p| p.as_ref())
                .map(|provider| {
                    provider.get_tracer(
                        "wasmcp-otel-tracing-middleware",
                        Some("0.1.0"),
                        None,
                        &[],
                    )
                })
        });
    }

    /// Get the tracer provider (returns None if not initialized or disabled)
    fn get_tracer_provider() -> Option<&'static TracerProvider> {
        TRACER_PROVIDER.get().and_then(|p| p.as_ref())
    }

    /// Get the tracer ID (returns None if not initialized or disabled)
    fn get_tracer_id() -> Option<u32> {
        TRACER_ID.get().and_then(|t| t.as_ref()).copied()
    }

    /// Get the transport (returns None if not initialized or disabled)
    fn get_transport() -> Option<&'static ExporterTransport> {
        TRANSPORT.get().and_then(|t| t.as_ref())
    }

    /// Extract span attributes from MCP request
    fn extract_attributes(request: &Request) -> Vec<Attribute> {
        // Convert ID to string representation
        let id_string = match request.id() {
            bindings::wasmcp::mcp::types::Id::Number(n) => n.to_string(),
            bindings::wasmcp::mcp::types::Id::String(s) => s,
        };

        let mut attrs = vec![
            Attribute {
                key: "mcp.request.id".to_string(),
                value: AttributeValue::String(id_string),
            },
        ];

        // Add method if available from params
        if let Ok(params) = request.params() {
            let method = match params {
                bindings::wasmcp::mcp::request::Params::Initialize(_) => "initialize",
                bindings::wasmcp::mcp::request::Params::ToolsList(_) => "tools/list",
                bindings::wasmcp::mcp::request::Params::ToolsCall(_) => "tools/call",
                bindings::wasmcp::mcp::request::Params::ResourcesList(_) => "resources/list",
                bindings::wasmcp::mcp::request::Params::ResourcesRead(_) => "resources/read",
                bindings::wasmcp::mcp::request::Params::ResourcesTemplatesList(_) => "resource_templates/list",
                bindings::wasmcp::mcp::request::Params::PromptsList(_) => "prompts/list",
                bindings::wasmcp::mcp::request::Params::PromptsGet(_) => "prompts/get",
                bindings::wasmcp::mcp::request::Params::CompletionComplete(_) => "completion/complete",
            };

            attrs.push(Attribute {
                key: "mcp.method".to_string(),
                value: AttributeValue::String(method.to_string()),
            });
        }

        attrs
    }
}

impl IncomingHandlerGuest for Component {
    fn handle(request: Request, output: OutputStream) {
        // Initialize OTEL on first request
        Self::ensure_initialized();

        // If not initialized (provider disabled), just forward
        let Some(provider) = Self::get_tracer_provider() else {
            next_handler::handle(request, output);
            return;
        };

        let Some(tracer_id) = Self::get_tracer_id() else {
            next_handler::handle(request, output);
            return;
        };

        // Extract attributes from request
        let attributes = Self::extract_attributes(&request);

        // Determine span name from request - convert ID to string
        let id_string = match request.id() {
            bindings::wasmcp::mcp::types::Id::Number(n) => n.to_string(),
            bindings::wasmcp::mcp::types::Id::String(s) => s,
        };
        let span_name = format!("MCP {}", id_string);

        // Start span for this MCP request
        let span = provider.start_span(
            tracer_id,
            &span_name,
            SpanKind::Server, // MCP server processing a request
            &attributes,
            &[], // No links
            None,   // Use current time
        );

        // Get span context and set as active
        let span_context = span.get_context();
        context::set_active_context(&span_context);

        // Forward request to next handler in chain
        next_handler::handle(request, output);

        // End span - request processing complete
        span.set_status(&SpanStatus::Ok);
        let span_data = Span::end(span, None);

        // Collect all child spans
        let child_span_data: Vec<SpanData> = CHILD_SPANS.with(|spans| {
            let mut spans_vec = spans.borrow_mut();
            spans_vec.drain(..).map(|child_span| {
                child_span.set_status(&SpanStatus::Ok);
                Span::end(child_span, None)
            }).collect()
        });

        // Export synchronously before returning
        if let Some(transport) = Self::get_transport() {
            let exporter = TraceExporter::new(Some(1)); // Batch size 1 for immediate export

            // Add parent span
            exporter.add_spans(&[span_data]);

            // Add all child spans
            if !child_span_data.is_empty() {
                exporter.add_spans(&child_span_data);
            }

            // Synchronous export - MUST complete before returning
            let _ = exporter.export_batch(transport);
        }

        // Clear active context
        context::clear_active_context();
    }
}

impl OtelInstrumentationGuest for Component {
    fn add_span_attribute(key: String, value: String) {
        // Get current active context to access the span
        if let Some(span_context) = context::get_active_context() {
            // Note: We can't easily modify the parent span's attributes after it's created
            // This would require storing the parent span handle, which complicates the design
            // For now, this is a no-op - attributes should be added during span creation
            // TODO: Consider storing parent span handle for attribute updates
        }
    }

    fn add_span_event(name: String, attributes: Vec<Attribute>) {
        // Similar limitation - would need parent span handle
        // TODO: Implement with parent span storage
    }

    fn start_child_span(name: String, kind: SpanKind) -> u64 {
        let Some(provider) = Component::get_tracer_provider() else {
            return 0;
        };

        let Some(tracer_id) = Component::get_tracer_id() else {
            return 0;
        };

        // Create child span under current active context
        let child_span = provider.start_span(
            tracer_id,
            &name,
            kind,
            &[],
            &[],
            None,
        );

        // Add to child spans list and return a span ID
        CHILD_SPANS.with(|spans| {
            spans.borrow_mut().push(child_span);
        });

        NEXT_SPAN_ID.with(|id| {
            let current = *id.borrow();
            *id.borrow_mut() += 1;
            current
        })
    }

    fn end_child_span(_span_id: u64) {
        // Child spans are automatically collected and exported at the end of handle()
        // No action needed here - spans remain in the Vec until export
    }
}

bindings::export!(Component with_types_in bindings);
