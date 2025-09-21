use std::collections::HashMap;
use super::{Provider, ProviderError};

#[derive(Debug, Clone)]
pub struct GrafanaConfig {
    pub endpoint: String,
    pub api_key: String,
    pub org_id: Option<String>,
    pub service_name: String,
    pub resource_attributes: HashMap<String, String>,
}

pub struct GrafanaProvider;

impl Provider for GrafanaProvider {
    type Config = GrafanaConfig;

    fn send_trace_data(&self, data: &[u8], config: &Self::Config) -> Result<(), ProviderError> {
        let url = self.build_endpoint_url(&config.endpoint, config);
        let headers = self.build_headers(config);

        println!("[GRAFANA] Sending {} bytes to: {}", data.len(), url);

        let payload = data.to_vec();
        let headers_clone = headers.clone();

        // Use spin_sdk::http::run for async HTTP requests
        let result = spin_sdk::http::run(async move {
            println!("[GRAFANA] Building HTTP request...");

            // Create HTTP request using Spin SDK
            let mut builder = spin_sdk::http::Request::builder();
            builder.method(spin_sdk::http::Method::Post);
            builder.uri(&url);
            builder.header("content-type", "application/x-protobuf");

            // Add configured headers (like Authorization)
            for (key, value) in &headers_clone {
                println!("[GRAFANA] Adding header: {} = {}", key, if key.to_lowercase().contains("auth") { "[REDACTED]" } else { value });
                builder.header(key, value);
            }

            let request = builder.body(payload).build();
            println!("[GRAFANA] Sending HTTP request...");

            // Send the request
            let response: spin_sdk::http::Response = spin_sdk::http::send(request)
                .await
                .map_err(|e| {
                    let error_msg = format!("Failed to send OTLP data to Grafana: {e:?}");
                    println!("[GRAFANA] ERROR: {}", error_msg);
                    error_msg
                })?;

            println!("[GRAFANA] SUCCESS: OTLP data sent successfully to Grafana");
            println!("[GRAFANA] Response status: {}", response.status());

            let body = response.into_body();
            if let Ok(body_str) = String::from_utf8(body) {
                if !body_str.is_empty() {
                    println!("[GRAFANA] Response body: {}", body_str);
                } else {
                    println!("[GRAFANA] Empty response body (expected for successful OTLP submission)");
                }
            }
            Ok::<(), String>(())
        });

        result.map_err(|e| ProviderError::NetworkError(e))
    }

    fn build_endpoint_url(&self, base_endpoint: &str, _config: &Self::Config) -> String {
        // Grafana OTLP endpoint requires the /v1/traces path
        format!("{}/v1/traces", base_endpoint)
    }

    fn build_headers(&self, config: &Self::Config) -> Vec<(String, String)> {
        let mut headers = vec![];

        // Add authorization header with API key
        headers.push(("Authorization".to_string(), config.api_key.clone()));

        // Add org ID if provided
        if let Some(ref org_id) = config.org_id {
            headers.push(("X-Scope-OrgID".to_string(), org_id.clone()));
        }

        headers
    }
}

impl GrafanaProvider {
    pub fn new() -> Self {
        Self
    }

    /// Load Grafana configuration from Spin variables
    pub fn load_config_from_spin() -> Result<GrafanaConfig, ProviderError> {
        println!("[GRAFANA] Loading configuration from Spin variables...");

        let endpoint = spin_sdk::variables::get("otel_exporter_otlp_endpoint")
            .map_err(|_| ProviderError::InvalidEndpoint("Missing otel_exporter_otlp_endpoint variable".to_string()))?;

        let api_key = spin_sdk::variables::get("otel_exporter_otlp_headers_authorization")
            .map_err(|_| ProviderError::AuthenticationError("Missing otel_exporter_otlp_headers_authorization variable".to_string()))?;

        let service_name = spin_sdk::variables::get("otel_service_name")
            .unwrap_or_else(|_| "wasmcp-otel-exporter".to_string());

        let org_id = spin_sdk::variables::get("otel_grafana_org_id").ok();

        let resource_attrs_str = spin_sdk::variables::get("otel_resource_attributes")
            .unwrap_or_else(|_| "service.name=wasmcp-otel-exporter,deployment.environment=production".to_string());

        println!("[GRAFANA] Endpoint: {}", endpoint);
        println!("[GRAFANA] Service name: {}", service_name);
        println!("[GRAFANA] Authorization header configured: {}", !api_key.is_empty());
        println!("[GRAFANA] Org ID: {:?}", org_id);
        println!("[GRAFANA] Resource attributes: {}", resource_attrs_str);

        Ok(GrafanaConfig {
            endpoint,
            api_key,
            org_id,
            service_name,
            resource_attributes: parse_resource_attributes(&resource_attrs_str),
        })
    }
}

fn parse_resource_attributes(attrs_str: &str) -> HashMap<String, String> {
    let mut attributes = HashMap::new();
    for pair in attrs_str.split(',') {
        if let Some((key, value)) = pair.split_once('=') {
            attributes.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    attributes
}