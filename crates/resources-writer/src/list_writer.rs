use crate::bindings::exports::wasmcp::mcp::resources_list_result::{
    GuestWriter, Id, Options, Resource,
};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::helpers::{resource_to_json, write_to_stream};
use std::cell::RefCell;
use std::collections::VecDeque;

pub struct ListWriter {
    inner: RefCell<ListWriterInner>,
}

struct ListWriterInner {
    id: Id,
    output: OutputStream,
    pending_resources: VecDeque<Resource>,
    written_count: u32,
    closed: bool,
    header_written: bool,
    first_resource_written: bool,
}

impl ListWriter {
    pub fn new(id: Id, output: OutputStream, initial: Vec<Resource>) -> Self {
        Self {
            inner: RefCell::new(ListWriterInner {
                id,
                output,
                pending_resources: initial.into_iter().collect(),
                written_count: 0,
                closed: false,
                header_written: false,
                first_resource_written: false,
            }),
        }
    }

    fn write_header(inner: &mut ListWriterInner) -> Result<(), StreamError> {
        if inner.header_written {
            return Ok(());
        }

        // Write the JSON-RPC envelope opening and start of resources array
        let header = format!(
            r#"{{"jsonrpc":"2.0","id":{},"result":{{"resources":["#,
            match &inner.id {
                Id::Number(n) => n.to_string(),
                Id::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
            }
        );

        write_to_stream(&inner.output, header.as_bytes())?;
        inner.header_written = true;
        Ok(())
    }

    fn write_single_resource(
        inner: &mut ListWriterInner,
        resource: &Resource,
    ) -> Result<(), StreamError> {
        // Write comma if not the first resource
        if inner.first_resource_written {
            write_to_stream(&inner.output, b",")?;
        }

        // Write the resource JSON
        let resource_json = resource_to_json(resource);
        let resource_str =
            serde_json::to_string(&resource_json).map_err(|_| StreamError::Closed)?;
        write_to_stream(&inner.output, resource_str.as_bytes())?;

        inner.first_resource_written = true;
        inner.written_count += 1;
        Ok(())
    }

    fn write_footer(
        inner: &mut ListWriterInner,
        options: Option<&Options>,
    ) -> Result<(), StreamError> {
        // Close the resources array
        write_to_stream(&inner.output, b"]")?;

        // Add nextCursor if present
        if let Some(cursor) = options.and_then(|o| o.next_cursor.as_ref()) {
            let cursor_json = format!(",\"nextCursor\":{}", serde_json::to_string(cursor).unwrap());
            write_to_stream(&inner.output, cursor_json.as_bytes())?;
        } else {
            write_to_stream(&inner.output, b"}")?;
        }

        // Close the JSON-RPC envelope and add newline for stdio protocol
        write_to_stream(&inner.output, b"}\n")?;
        Ok(())
    }
}

// GuestWriter trait - resource instance methods
impl GuestWriter for ListWriter {
    fn check_write(&self) -> Result<u32, StreamError> {
        let inner = self.inner.borrow();
        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Check stream capacity
        let capacity = inner
            .output
            .check_write()
            .map_err(|_| StreamError::Closed)?;
        if capacity == 0 {
            return Ok(0);
        }

        // We can write one resource at a time when streaming
        Ok(if inner.pending_resources.is_empty() {
            0
        } else {
            1
        })
    }

    fn write(&self, resource: Resource) -> Result<(), StreamError> {
        let mut inner = self.inner.borrow_mut();

        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Ensure header is written
        Self::write_header(&mut inner)?;

        // Write the resource immediately
        Self::write_single_resource(&mut inner, &resource)?;

        Ok(())
    }

    fn close(&self, options: Option<Options>) -> Result<(), StreamError> {
        let mut inner = self.inner.borrow_mut();
        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Ensure header is written
        Self::write_header(&mut inner)?;

        // Write any remaining resources
        while let Some(resource) = inner.pending_resources.pop_front() {
            Self::write_single_resource(&mut inner, &resource)?;
        }

        // Write the footer
        Self::write_footer(&mut inner, options.as_ref())?;

        inner.closed = true;
        inner.output.flush().map_err(|_| StreamError::Closed)?;
        Ok(())
    }
}

// Guest trait - top-level static functions
impl crate::bindings::exports::wasmcp::mcp::resources_list_result::Guest for crate::Component {
    type Writer = ListWriter;

    fn write(
        id: Id,
        output: OutputStream,
        resources: Vec<Resource>,
        options: Option<Options>,
    ) -> Result<(), StreamError> {
        let writer = ListWriter::new(id, output, vec![]);
        let mut inner = writer.inner.borrow_mut();

        // For write, we stream everything in one go
        ListWriter::write_header(&mut inner)?;

        // Write all resources
        for resource in &resources {
            ListWriter::write_single_resource(&mut inner, resource)?;
        }

        // Write footer
        ListWriter::write_footer(&mut inner, options.as_ref())?;

        // Flush the stream
        inner.output.flush().map_err(|_| StreamError::Closed)
    }

    fn open(
        id: Id,
        output: OutputStream,
        initial: Resource,
    ) -> Result<crate::bindings::exports::wasmcp::mcp::resources_list_result::Writer, StreamError>
    {
        let writer = ListWriter::new(id, output, vec![initial]);
        {
            let mut inner = writer.inner.borrow_mut();

            // Write the header to start streaming
            ListWriter::write_header(&mut inner)?;

            // Write initial resource if any
            while let Some(resource) = inner.pending_resources.pop_front() {
                ListWriter::write_single_resource(&mut inner, &resource)?;

                // Break after writing a few to allow for backpressure
                if inner.written_count >= 5 {
                    break;
                }
            }
        }

        Ok(crate::bindings::exports::wasmcp::mcp::resources_list_result::Writer::new(writer))
    }
}
