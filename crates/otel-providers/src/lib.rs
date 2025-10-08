#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        world: "providers",
        generate_all,
    });
}

use bindings::exports::wasi::otel_providers::common_providers::{
    Guest, Provider, GrafanaConfig, DatadogConfig, HoneycombConfig,
};
use bindings::wasi::otel_sdk::http_transport::{
    HttpConfig, AuthConfig, BearerTokenConfig, CompressionType, RetryConfig,
};
use bindings::wasi::otel_sdk::transport::ContentType;

pub struct Component;

impl Guest for Component {
    fn to_http_config(config: Provider) -> Result<HttpConfig, String> {
        match config {
            Provider::Grafana(g) => grafana_to_http_config(g),
            Provider::Datadog(d) => datadog_to_http_config(d),
            Provider::Honeycomb(h) => honeycomb_to_http_config(h),
            Provider::OtlpHttp(cfg) => Ok(cfg),
            Provider::Disabled => Err("Telemetry disabled".to_string()),
        }
    }
}

fn grafana_to_http_config(config: GrafanaConfig) -> Result<HttpConfig, String> {
    Ok(HttpConfig {
        endpoint: format!(
            "https://otlp-gateway-{}.grafana.net/otlp",
            config.region
        ),
        authentication: AuthConfig::Bearer(BearerTokenConfig {
            token: format!("{}:{}", config.instance_id, config.api_key),
        }),
        timeout_ms: 10000,
        protocol: ContentType::Protobuf,
        compression: CompressionType::Gzip,
        retry: Some(RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 10000,
        }),
    })
}

fn datadog_to_http_config(config: DatadogConfig) -> Result<HttpConfig, String> {
    Ok(HttpConfig {
        endpoint: format!("https://trace.agent.{}/v0.4/traces", config.site),
        authentication: AuthConfig::Bearer(BearerTokenConfig {
            token: config.api_key,
        }),
        timeout_ms: 10000,
        protocol: ContentType::Protobuf,
        compression: CompressionType::Gzip,
        retry: Some(RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 10000,
        }),
    })
}

fn honeycomb_to_http_config(config: HoneycombConfig) -> Result<HttpConfig, String> {
    // Note: Honeycomb dataset header would need to be supported via auth-config::headers variant
    // For now, dataset is ignored if provided
    Ok(HttpConfig {
        endpoint: config.endpoint.unwrap_or_else(|| "https://api.honeycomb.io".to_string()),
        authentication: AuthConfig::Bearer(BearerTokenConfig {
            token: config.api_key,
        }),
        timeout_ms: 10000,
        protocol: ContentType::Protobuf,
        compression: CompressionType::Gzip,
        retry: Some(RetryConfig {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 10000,
        }),
    })
}

bindings::export!(Component with_types_in bindings);
