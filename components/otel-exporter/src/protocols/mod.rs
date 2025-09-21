pub mod otlp_http;

use crate::SpanImpl;

#[derive(Debug)]
pub enum ProtocolError {
    SerializationError(String),
    UnsupportedFeature(String),
    InvalidData(String),
}

pub trait Protocol {
    type Config;

    fn serialize_span(&self, span: &SpanImpl, config: &Self::Config) -> Result<Vec<u8>, ProtocolError>;
    fn content_type(&self) -> &'static str;
    fn supports_compression(&self) -> bool;
}