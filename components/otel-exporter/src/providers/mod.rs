pub mod grafana;
pub mod jaeger;
pub mod generic;

#[derive(Debug)]
pub enum ProviderError {
    NetworkError(String),
    AuthenticationError(String),
    InvalidEndpoint(String),
    SerializationError(String),
}

pub trait Provider {
    type Config;

    fn send_trace_data(&self, data: &[u8], config: &Self::Config) -> Result<(), ProviderError>;
    fn build_endpoint_url(&self, base_endpoint: &str, config: &Self::Config) -> String;
    fn build_headers(&self, config: &Self::Config) -> Vec<(String, String)>;
}