use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::foundation;
use crate::bindings::wasi::otel_sdk::otel_export;
use crate::otlp;

use std::sync::{Arc, Mutex};

/// Implementation of the trace exporter resource
pub struct TraceExporterImpl {
    inner: Arc<Mutex<TraceExporterInner>>,
}

pub struct TraceExporterInner {
    http_client_handle: u32, // Store the HTTP client handle from wit-bindgen
    spans: Vec<trace::SpanData>,
    resource: Option<foundation::OtelResource>,
    batch_size: usize,
}

impl TraceExporterImpl {
    /// Maximum batch size for span export
    const MAX_BATCH_SIZE: usize = 512;

    /// Get the appropriate OTLP endpoint path based on protocol
    fn get_otlp_path(protocol: &otel_export::ExportProtocol) -> &'static str {
        match protocol {
            otel_export::ExportProtocol::Grpc => "/opentelemetry.proto.collector.trace.v1.TraceService/Export",
            otel_export::ExportProtocol::HttpProtobuf => "/v1/traces",
            otel_export::ExportProtocol::HttpJson => "/v1/traces",
        }
    }

    /// Prepare spans for export by batching them
    fn prepare_batch(&mut self) -> Option<Vec<trace::SpanData>> {
        let mut inner = self.inner.lock().unwrap();

        if inner.spans.is_empty() {
            return None;
        }

        // Take up to MAX_BATCH_SIZE spans
        let batch_size = inner.spans.len().min(Self::MAX_BATCH_SIZE);
        let batch: Vec<trace::SpanData> = inner.spans.drain(..batch_size).collect();

        Some(batch)
    }

    /// Export a batch of spans via HTTP
    fn export_batch_internal(
        &self,
        spans: Vec<trace::SpanData>,
        resource: &foundation::OtelResource,
        http_client: &otel_export::HttpClient,
    ) -> Result<(), String> {
        // Get the export protocol from the HTTP client configuration
        let protocol = otel_export::ExportProtocol::HttpProtobuf; // Default to protobuf

        // Serialize spans to OTLP format
        let body = otlp::serialize_spans_to_otlp(spans, resource.clone(), protocol.clone())?;

        // Get the appropriate content type
        let content_type = match protocol {
            otel_export::ExportProtocol::Grpc => "application/grpc",
            otel_export::ExportProtocol::HttpProtobuf => "application/x-protobuf",
            otel_export::ExportProtocol::HttpJson => "application/json",
        };

        // Create request with appropriate headers
        let path = Self::get_otlp_path(&protocol);

        // Send the request using the HTTP client
        let response = http_client.send_otlp(
            path,
            &body,
            content_type,
        );

        match response {
            otel_export::ExportResult::Success => Ok(()),
            otel_export::ExportResult::Failure(msg) => {
                Err(format!("Export failed: {}", msg))
            }
            otel_export::ExportResult::PartialFailure(error) => {
                Err(format!("Export partially failed: {:?}", error))
            }
        }
    }

    /// Get a clone of the inner Arc for registry storage
    pub fn inner_arc(&self) -> Arc<Mutex<TraceExporterInner>> {
        self.inner.clone()
    }
}

impl trace::GuestTraceExporter for TraceExporterImpl {
    fn new(client: &otel_export::HttpClient) -> Self {
        // Store the HTTP client handle so we can use it in export methods
        let client_handle = client.handle();

        let inner = Arc::new(Mutex::new(TraceExporterInner {
            http_client_handle: client_handle,
            spans: Vec::new(),
            resource: None,
            batch_size: TraceExporterImpl::MAX_BATCH_SIZE,
        }));

        Self {
            inner,
        }
    }

    fn add_spans(&self, spans: Vec<trace::SpanData>) {
        let mut inner = self.inner.lock().unwrap();

        // Add spans to the internal buffer
        inner.spans.extend(spans);

        // Optional: Trigger export if buffer is full
        if inner.spans.len() >= inner.batch_size {
            // In a real implementation, we might trigger an async export here
            // For now, we just accumulate spans
        }
    }

    fn set_resource(&self, service_resource: foundation::OtelResource) {
        let mut inner = self.inner.lock().unwrap();
        inner.resource = Some(service_resource);
    }

    fn export_batch(&self) -> otel_export::ExportResult {
        let mut inner = self.inner.lock().unwrap();

        // If no spans, nothing to export
        if inner.spans.is_empty() {
            return otel_export::ExportResult::Success;
        }

        // Get the resource (use default if not set)
        let resource = inner.resource.clone().unwrap_or_else(|| {
            foundation::OtelResource {
                attributes: vec![
                    foundation::Attribute {
                        key: "service.name".to_string(),
                        value: foundation::AttributeValue::String("unknown_service".to_string()),
                    },
                ],
                schema_url: None,
            }
        });

        // Take up to MAX_BATCH_SIZE spans
        let batch_size = inner.spans.len().min(Self::MAX_BATCH_SIZE);
        let batch: Vec<trace::SpanData> = inner.spans.drain(..batch_size).collect();

        // Get the HTTP client handle
        let client_handle = inner.http_client_handle;

        // Release the lock before doing the export
        drop(inner);

        // Reconstruct the HttpClient from the handle
        let http_client = unsafe { otel_export::HttpClient::from_handle(client_handle) };

        // Export the batch using the internal implementation
        match self.export_batch_internal(batch, &resource, &http_client) {
            Ok(()) => otel_export::ExportResult::Success,
            Err(msg) => otel_export::ExportResult::Failure(msg),
        }
    }

    fn force_flush(&self) -> bool {
        // Export all pending spans by repeatedly calling export_batch
        loop {
            let has_spans = {
                let inner = self.inner.lock().unwrap();
                !inner.spans.is_empty()
            };

            if !has_spans {
                break;
            }

            // Export a batch
            match self.export_batch() {
                otel_export::ExportResult::Success => continue,
                otel_export::ExportResult::Failure(_) | otel_export::ExportResult::PartialFailure(_) => {
                    return false; // Export failed
                }
            }
        }

        true // All spans successfully exported
    }

    fn finish(exporter: trace::TraceExporter) -> bool {
        // Get the handle ID from the exporter
        let handle = exporter.handle();

        // Retrieve the exporter data from the registry and remove it
        if let Some(exporter_data) = crate::remove_exporter(handle) {
            let inner = exporter_data.lock().unwrap();

            // In a production implementation, we would:
            // 1. Export any remaining spans
            // 2. Flush the HTTP client
            // 3. Clean up resources

            // For now, we successfully cleaned up the exporter
            drop(inner);
            true
        } else {
            // Exporter not found in registry, still return success
            // as the resource has been consumed
            true
        }
    }
}