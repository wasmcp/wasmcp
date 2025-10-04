use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::foundation;
use crate::bindings::wasi::otel_sdk::context;
use crate::span::SpanImpl;
use crate::{ensure_registry, TRACER_REGISTRY};

use std::sync::Mutex;

// Define configuration types locally since they're not available in exports
#[derive(Clone)]
enum SamplerConfig {
    AlwaysOn,
    AlwaysOff,
    TraceIdRatio(f64),
    ParentBased,
}

#[derive(Clone)]
struct TraceLimitsConfig {
    max_attributes_per_span: u32,
    max_events_per_span: u32,
    max_links_per_span: u32,
    attribute_value_length_limit: u32,
    span_attribute_count_limit: u32,
}

/// Implementation of the tracer provider resource
pub struct TracerProviderImpl {
    inner: Mutex<TracerProviderInner>,
}

struct TracerProviderInner {
    resource: Option<foundation::OtelResource>,
    sampler_config: SamplerConfig,
    limits: TraceLimitsConfig,
    enabled: bool,
}

impl TracerProviderImpl {
    /// Create a new tracer provider
    pub fn new() -> Self {
        ensure_registry();

        Self {
            inner: Mutex::new(TracerProviderInner {
                resource: None,
                sampler_config: SamplerConfig::AlwaysOn,
                limits: TraceLimitsConfig {
                    max_attributes_per_span: 128,
                    max_events_per_span: 128,
                    max_links_per_span: 32,
                    attribute_value_length_limit: 1024,
                    span_attribute_count_limit: 256,
                },
                enabled: true,
            }),
        }
    }

    /// Configure the tracer provider with resource and sampling
    pub fn configure(
        &self,
        resource: Option<foundation::OtelResource>,
        sampler_config: Option<SamplerConfig>,
        limits: Option<TraceLimitsConfig>,
    ) {
        let mut inner = self.inner.lock().unwrap();

        if let Some(res) = resource {
            inner.resource = Some(res);
        }

        if let Some(sampler) = sampler_config {
            inner.sampler_config = sampler;
        }

        if let Some(lim) = limits {
            inner.limits = lim;
        }
    }

    /// Check if a span should be sampled based on the sampling configuration
    fn should_sample(&self, trace_id: &[u8]) -> bool {
        let inner = self.inner.lock().unwrap();

        match &inner.sampler_config {
            SamplerConfig::AlwaysOn => true,
            SamplerConfig::AlwaysOff => false,
            SamplerConfig::TraceIdRatio(ratio) => {
                // Use the last 8 bytes of trace ID for sampling decision
                if trace_id.len() >= 8 {
                    let last_8_bytes = &trace_id[trace_id.len() - 8..];
                    let mut value = 0u64;
                    for (i, byte) in last_8_bytes.iter().enumerate() {
                        value |= (*byte as u64) << (i * 8);
                    }
                    let threshold = (ratio * u64::MAX as f64) as u64;
                    value < threshold
                } else {
                    true // Sample if trace ID is invalid
                }
            }
            SamplerConfig::ParentBased => {
                // Check if parent context is sampled
                if let Some(parent_ctx) = context::get_active_context() {
                    context::is_sampled(&parent_ctx)
                } else {
                    // No parent, fall back to always on
                    true
                }
            }
        }
    }

    /// Apply limits to attributes
    fn apply_attribute_limits(&self, mut attributes: Vec<foundation::Attribute>) -> Vec<foundation::Attribute> {
        let inner = self.inner.lock().unwrap();
        let limits = &inner.limits;

        // Truncate to max attributes
        attributes.truncate(limits.max_attributes_per_span as usize);

        // Truncate string values
        for attr in &mut attributes {
            if let foundation::AttributeValue::String(s) = &mut attr.value {
                if s.len() > limits.attribute_value_length_limit as usize {
                    s.truncate(limits.attribute_value_length_limit as usize);
                }
            }
        }

        attributes
    }

    /// Apply limits to events
    fn apply_event_limits(&self, mut events: Vec<trace::SpanEvent>) -> Vec<trace::SpanEvent> {
        let inner = self.inner.lock().unwrap();
        events.truncate(inner.limits.max_events_per_span as usize);
        events
    }

    /// Apply limits to links
    fn apply_link_limits(&self, mut links: Vec<trace::SpanLink>) -> Vec<trace::SpanLink> {
        let inner = self.inner.lock().unwrap();
        links.truncate(inner.limits.max_links_per_span as usize);
        links
    }
}

impl trace::GuestTracerProvider for TracerProviderImpl {
    fn get_tracer(
        &self,
        name: String,
        version: Option<String>,
        schema_url: Option<String>,
        attributes: Vec<foundation::Attribute>,
    ) -> trace::Tracer {
        let mut registry = TRACER_REGISTRY.lock().unwrap();
        let registry = registry.as_mut().unwrap();

        registry.register_tracer(name, version, schema_url, attributes)
    }

    fn start_span(
        &self,
        tracer: trace::Tracer,
        name: String,
        kind: trace::SpanKind,
        attributes: Vec<foundation::Attribute>,
        links: Vec<trace::SpanLink>,
        start_time: Option<u64>,
    ) -> trace::Span {
        // Get tracer info
        let registry = TRACER_REGISTRY.lock().unwrap();
        let registry = registry.as_ref().unwrap();
        let tracer_info = registry.get_tracer_info(tracer).unwrap();

        // Get or create span context
        let (ctx, parent_span_id) = if let Some(parent_ctx) = context::get_active_context() {
            // Create child context
            let span_id = context::generate_span_id();
            let child_ctx = context::create_child_context(&parent_ctx, &span_id);

            // Extract parent span ID
            let parent_id = if parent_ctx.span_id.len() == 8 {
                Some(parent_ctx.span_id.clone())
            } else {
                None
            };

            (child_ctx, parent_id)
        } else {
            // Create root context
            let trace_id = context::generate_trace_id();
            let span_id = context::generate_span_id();

            // Check sampling decision
            let should_sample = self.should_sample(&trace_id);
            let trace_flags = if should_sample { 0x01 } else { 0x00 };

            let root_ctx = context::create_root_context(&trace_id, &span_id, trace_flags);
            (root_ctx, None)
        };

        // Check if span should be recorded
        let is_sampled = context::is_sampled(&ctx);

        // Apply limits to attributes, events, and links
        let limited_attributes = if is_sampled {
            self.apply_attribute_limits(attributes)
        } else {
            Vec::new() // Don't store attributes for non-sampled spans
        };

        let limited_links = if is_sampled {
            self.apply_link_limits(links)
        } else {
            Vec::new()
        };

        // Create instrumentation scope
        let scope = foundation::InstrumentationScope {
            name: tracer_info.name.clone(),
            version: tracer_info.version.clone(),
            schema_url: tracer_info.schema_url.clone(),
            attributes: tracer_info.attributes.clone(),
        };

        // Create the span implementation
        let span_impl = SpanImpl::new(
            name,
            ctx.clone(),
            parent_span_id,
            kind,
            start_time,
            limited_attributes,
            limited_links,
            scope,
        );

        // Clone the inner Arc before creating the WIT resource
        let inner_arc = span_impl.inner_arc();

        // Create the WIT resource wrapper - this assigns a handle and consumes span_impl
        let span_resource = trace::Span::new(span_impl);

        // Register the span data using the handle that wit-bindgen assigned
        let handle = span_resource.handle();
        crate::register_span_with_handle(handle, inner_arc);

        // Set as active context
        context::set_active_context(&ctx);

        span_resource
    }

    fn tracer_enabled(&self, _tracer: trace::Tracer) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.enabled
    }
}