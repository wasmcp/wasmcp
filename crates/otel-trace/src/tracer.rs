use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::common;
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
    resource: Option<common::OtelResource>,
    sampler_config: SamplerConfig,
    limits: TraceLimitsConfig,
    enabled: bool,
}

impl TracerProviderImpl {
    /// Create a new tracer provider with configuration
    fn with_config(
        sampler: Option<trace::SamplerConfig>,
        limits: Option<trace::TraceLimitsConfig>,
        service_resource: Option<common::OtelResource>,
    ) -> Self {
        ensure_registry();

        // Convert WIT sampler config to internal type
        let sampler_config = sampler.map(|s| match s {
            trace::SamplerConfig::AlwaysOn => SamplerConfig::AlwaysOn,
            trace::SamplerConfig::AlwaysOff => SamplerConfig::AlwaysOff,
            trace::SamplerConfig::TraceIdRatio(ratio) => SamplerConfig::TraceIdRatio(ratio),
            trace::SamplerConfig::ParentBased => SamplerConfig::ParentBased,
        }).unwrap_or(SamplerConfig::AlwaysOn);

        // Convert WIT limits config to internal type
        let limits_config = limits.map(|l| TraceLimitsConfig {
            max_attributes_per_span: l.max_attributes_per_span,
            max_events_per_span: l.max_events_per_span,
            max_links_per_span: l.max_links_per_span,
            attribute_value_length_limit: l.attribute_value_length_limit,
            span_attribute_count_limit: l.span_attribute_count_limit,
        }).unwrap_or(TraceLimitsConfig {
            max_attributes_per_span: 128,
            max_events_per_span: 128,
            max_links_per_span: 128,
            attribute_value_length_limit: 1024,
            span_attribute_count_limit: 256,
        });

        Self {
            inner: Mutex::new(TracerProviderInner {
                resource: service_resource,
                sampler_config,
                limits: limits_config,
                enabled: true,
            }),
        }
    }

    /// Check if a span should be sampled based on the sampling configuration
    fn should_sample(&self, trace_id: &[u8]) -> bool {
        let inner = self.inner.lock().unwrap();

        match &inner.sampler_config {
            SamplerConfig::AlwaysOn => true,
            SamplerConfig::AlwaysOff => false,
            SamplerConfig::TraceIdRatio(ratio) => {
                // Match OpenTelemetry spec: use big-endian last 8 bytes
                if *ratio >= 1.0 {
                    return true;
                }

                if trace_id.len() < 8 {
                    return true; // Sample if trace ID is invalid
                }

                // Use last 8 bytes of trace ID for deterministic sampling
                let last_8_bytes = &trace_id[trace_id.len() - 8..];
                let trace_id_low = u64::from_be_bytes(last_8_bytes.try_into().unwrap());
                let rnd_from_trace_id = trace_id_low >> 1;
                let prob_upper_bound = (ratio.max(0.0) * (1u64 << 63) as f64) as u64;

                rnd_from_trace_id < prob_upper_bound
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
    fn apply_attribute_limits(&self, mut attributes: Vec<common::Attribute>) -> Vec<common::Attribute> {
        let inner = self.inner.lock().unwrap();
        let limits = &inner.limits;

        // Truncate to max attributes
        attributes.truncate(limits.max_attributes_per_span as usize);

        // Truncate string values
        for attr in &mut attributes {
            if let common::AttributeValue::String(s) = &mut attr.value {
                if s.len() > limits.attribute_value_length_limit as usize {
                    s.truncate(limits.attribute_value_length_limit as usize);
                }
            }
        }

        attributes
    }

    /// Apply limits to links
    fn apply_link_limits(&self, mut links: Vec<trace::SpanLink>) -> Vec<trace::SpanLink> {
        let inner = self.inner.lock().unwrap();
        links.truncate(inner.limits.max_links_per_span as usize);
        links
    }
}

impl trace::GuestTracerProvider for TracerProviderImpl {
    fn new(
        sampler: Option<trace::SamplerConfig>,
        limits: Option<trace::TraceLimitsConfig>,
        service_resource: Option<common::OtelResource>,
    ) -> Self {
        Self::with_config(sampler, limits, service_resource)
    }

    fn get_tracer(
        &self,
        name: String,
        version: Option<String>,
        schema_url: Option<String>,
        attributes: Vec<common::Attribute>,
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
        attributes: Vec<common::Attribute>,
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
        let scope = common::InstrumentationScope {
            name: tracer_info.name.clone(),
            version: tracer_info.version.clone(),
            schema_url: tracer_info.schema_url.clone(),
            attributes: tracer_info.attributes.clone(),
        };

        // Get limits from provider
        let limits = {
            let inner = self.inner.lock().unwrap();
            inner.limits.clone()
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
            limits.max_attributes_per_span,
            limits.max_events_per_span,
            limits.max_links_per_span,
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

    fn force_flush(&self) -> bool {
        // For tracer provider, force_flush is a no-op as spans are exported by the exporter
        // Return true to indicate success
        true
    }

    fn shutdown(_provider: trace::TracerProvider) -> bool {
        // Consume the provider and mark shutdown complete
        // The provider resource will be dropped automatically
        true
    }
}