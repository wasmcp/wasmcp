use super::{Provider, ProviderError};
use crate::bindings::wasmcp::otel_exporter::otel_provider_config::GenericOtlpConfig;

pub struct GenericOtlpProvider;

impl Provider for GenericOtlpProvider {
    type Config = GenericOtlpConfig;

    fn send_trace_data(&self, data: &[u8], config: &Self::Config) -> Result<(), ProviderError> {
        let url = self.build_endpoint_url(&config.endpoint, config);
        let headers = self.build_headers(config);

        println!("[GENERIC-OTLP] Sending {} bytes to: {}", data.len(), url);

        let payload = data.to_vec();
        let headers_clone = headers.clone();

        // Use spin_sdk::http::run for async HTTP requests
        let result = spin_sdk::http::run(async move {
            println!("[GENERIC-OTLP] Building HTTP request...");

            // Create HTTP request using Spin SDK
            let mut builder = spin_sdk::http::Request::builder();
            builder.method(spin_sdk::http::Method::Post);
            builder.uri(&url);
            builder.header("content-type", "application/x-protobuf");

            // Add configured headers
            for (key, value) in &headers_clone {
                println!("[GENERIC-OTLP] Adding header: {} = {}", key, if key.to_lowercase().contains("auth") { "[REDACTED]" } else { value });
                builder.header(key, value);
            }

            let request = builder.body(payload).build();
            println!("[GENERIC-OTLP] Sending HTTP request...");

            // Send the request
            let response: spin_sdk::http::Response = spin_sdk::http::send(request)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to send data to generic OTLP endpoint: {e:?}");
                    println!("[GENERIC-OTLP] ERROR: {}", error_msg);
                    error_msg
                })?;

            println!("[GENERIC-OTLP] SUCCESS: Data sent successfully to generic OTLP endpoint");
            println!("[GENERIC-OTLP] Response status: {}", response.status());

            let body = response.into_body();
            if let Ok(body_str) = String::from_utf8(body) {
                if !body_str.is_empty() {
                    println!("[GENERIC-OTLP] Response body: {}", body_str);
                }
            }
            Ok::<(), String>(())
        });

        result.map_err(|e| ProviderError::NetworkError(e))
    }

    fn build_endpoint_url(&self, base_endpoint: &str, _config: &Self::Config) -> String {
        // Generic OTLP endpoints typically use /v1/traces path
        format!("{}/v1/traces", base_endpoint)
    }

    fn build_headers(&self, config: &Self::Config) -> Vec<(String, String)> {
        // Use all custom headers from configuration
        config.headers.clone()
    }
}

impl GenericOtlpProvider {
    pub fn new() -> Self {
        Self
    }
}