use std::sync::OnceLock;
use crate::bindings::wasmcp::otel_exporter::otel_provider_config;

// Import the WIT-generated types
use otel_provider_config::{OtelConfig, OtelProvider, OtelProtocol};

static CACHED_CONFIG: OnceLock<Option<OtelConfig>> = OnceLock::new();

pub fn get_otel_config() -> Option<&'static OtelConfig> {
    CACHED_CONFIG.get_or_init(|| {
        // Try to get configuration from user component - this might fail if user didn't provide it
        match std::panic::catch_unwind(|| {
            otel_provider_config::get_otel_config()
        }) {
            Ok(config) => {
                println!("[VARIANT-CONFIG] Loaded user configuration: provider={:?}, protocol={:?}",
                    variant_name(&config.provider), protocol_name(&config.protocol));
                Some(config)
            },
            Err(_) => {
                println!("[VARIANT-CONFIG] No OTEL configuration provided by user component - tracing disabled");
                None // User didn't provide OTEL configuration
            }
        }
    }).as_ref()
}

pub fn is_tracing_enabled() -> bool {
    get_otel_config().is_some()
}

// Helper functions to get variant names for logging
fn variant_name(provider: &OtelProvider) -> &'static str {
    match provider {
        OtelProvider::Grafana(_) => "grafana",
        OtelProvider::Jaeger(_) => "jaeger",
        OtelProvider::Datadog(_) => "datadog",
        OtelProvider::Honeycomb(_) => "honeycomb",
        OtelProvider::Newrelic(_) => "newrelic",
        OtelProvider::GenericOtlp(_) => "generic-otlp",
    }
}

fn protocol_name(protocol: &OtelProtocol) -> &'static str {
    match protocol {
        OtelProtocol::OtlpHttp(_) => "otlp-http",
        OtelProtocol::OtlpGrpc(_) => "otlp-grpc",
        OtelProtocol::JaegerThrift(_) => "jaeger-thrift",
        OtelProtocol::ZipkinJson(_) => "zipkin-json",
    }
}