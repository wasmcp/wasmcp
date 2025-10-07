use crate::bindings::exports::wasi::otel_sdk::trace;
use crate::bindings::wasi::otel_sdk::common;
use crate::bindings::wasi::otel_sdk::transport;
use crate::otlp;

use std::sync::{Arc, Mutex};

/// Implementation of the trace exporter resource
pub struct TraceExporterImpl {
    inner: Arc<Mutex<TraceExporterInner>>,
}

pub struct TraceExporterInner {
    spans: Vec<trace::SpanData>,
    resource: Option<common::OtelResource>,
    max_batch_size: usize,
}

impl TraceExporterImpl {
    /// Default batch size for span export
    const DEFAULT_BATCH_SIZE: usize = 512;
    /// Maximum allowed batch size (safety limit)
    const MAX_ALLOWED_BATCH_SIZE: usize = 10_000;

    /// Export a batch of spans via abstract transport
    fn export_batch_internal(
        &self,
        spans: Vec<trace::SpanData>,
        resource: &common::OtelResource,
        exporter_transport: &transport::ExporterTransport,
    ) -> Result<(), String> {
        // Serialize spans to OTLP format based on content type
        let body = otlp::serialize_spans_to_otlp(spans, resource.clone())
            .map_err(|e| e.to_string())?;

        // Send via transport using the new API
        let response = exporter_transport.send(
            transport::SignalType::Traces,
            &body,
            transport::ContentType::Protobuf,
        );

        match response {
            transport::ExportResult::Success => Ok(()),
            transport::ExportResult::Failure(msg) => {
                Err(format!("Export failed: {}", msg))
            }
            transport::ExportResult::PartialFailure(error) => {
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
    fn new(batch_size: Option<u32>) -> Self {
        // Use provided batch size, default to 512, cap at 10,000
        let max_batch_size = batch_size
            .map(|size| (size as usize).min(Self::MAX_ALLOWED_BATCH_SIZE))
            .unwrap_or(Self::DEFAULT_BATCH_SIZE);

        let inner = Arc::new(Mutex::new(TraceExporterInner {
            spans: Vec::new(),
            resource: None,
            max_batch_size,
        }));

        Self {
            inner,
        }
    }

    fn add_spans(&self, spans: Vec<trace::SpanData>) {
        let mut inner = self.inner.lock().unwrap();

        // Add spans to the internal buffer
        inner.spans.extend(spans);
    }

    fn set_resource(&self, service_resource: common::OtelResource) {
        let mut inner = self.inner.lock().unwrap();
        inner.resource = Some(service_resource);
    }

    fn export_batch(&self, exporter_transport: &transport::ExporterTransport) -> transport::ExportResult {
        let mut inner = self.inner.lock().unwrap();

        // If no spans, nothing to export
        if inner.spans.is_empty() {
            return transport::ExportResult::Success;
        }

        // Get the resource (use default if not set)
        let resource = inner.resource.clone().unwrap_or_else(|| {
            common::OtelResource {
                attributes: vec![
                    common::Attribute {
                        key: "service.name".to_string(),
                        value: common::AttributeValue::String("unknown_service".to_string()),
                    },
                ],
                schema_url: None,
            }
        });

        // Take up to max_batch_size spans
        let batch_size = inner.spans.len().min(inner.max_batch_size);
        let batch: Vec<trace::SpanData> = inner.spans.drain(..batch_size).collect();

        // Release the lock before doing the export
        drop(inner);

        // Export the batch using the internal implementation
        match self.export_batch_internal(batch, &resource, exporter_transport) {
            Ok(()) => transport::ExportResult::Success,
            Err(msg) => transport::ExportResult::Failure(msg),
        }
    }

    fn force_flush(&self, exporter_transport: &transport::ExporterTransport) -> bool {
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
            match self.export_batch(exporter_transport) {
                transport::ExportResult::Success => continue,
                transport::ExportResult::Failure(_) | transport::ExportResult::PartialFailure(_) => {
                    return false; // Export failed
                }
            }
        }

        true // All spans successfully exported
    }

    fn shutdown(exporter: trace::TraceExporter, exporter_transport: &transport::ExporterTransport) -> bool {
        // Get the handle ID from the exporter
        let handle = exporter.handle();

        // Retrieve the exporter data from the registry and remove it
        if let Some(exporter_data) = crate::remove_exporter(handle) {
            // Export any remaining spans before cleanup
            let exporter_impl = TraceExporterImpl { inner: exporter_data };
            let flush_result = exporter_impl.force_flush(exporter_transport);

            // Clean up resources
            drop(exporter_impl);

            flush_result
        } else {
            // Exporter not found in registry, still return success
            // as the resource has been consumed
            true
        }
    }
}