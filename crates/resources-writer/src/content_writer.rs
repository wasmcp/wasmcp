use crate::bindings::exports::wasmcp::mcp::resources_read_result::{
    Contents, GuestWriter, Id, Options,
};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::helpers::write_to_stream;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::cell::RefCell;

pub struct ContentWriter {
    inner: RefCell<ContentWriterInner>,
}

struct ContentWriterInner {
    id: Id,
    output: OutputStream,
    uri: String,
    mime_type: Option<String>,
    data_buffer: Vec<u8>,
    closed: bool,
    header_written: bool,
}

impl ContentWriter {
    pub fn new(id: Id, output: OutputStream, uri: String) -> Self {
        Self {
            inner: RefCell::new(ContentWriterInner {
                id,
                output,
                uri,
                mime_type: None,
                data_buffer: Vec::new(),
                closed: false,
                header_written: false,
            }),
        }
    }

    fn write_header(inner: &mut ContentWriterInner) -> Result<(), StreamError> {
        if inner.header_written {
            return Ok(());
        }

        // Write the JSON-RPC envelope opening and start of contents
        let mut header = format!(
            r#"{{"jsonrpc":"2.0","id":{},"result":{{"contents":[{{"uri":{}"#,
            match &inner.id {
                Id::Number(n) => n.to_string(),
                Id::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
            },
            serde_json::to_string(&inner.uri).unwrap_or_else(|_| "\"\"".to_string())
        );

        // Add mimeType if present
        if let Some(mime) = &inner.mime_type {
            header.push_str(r#","mimeType":"#);
            header.push_str(&serde_json::to_string(mime).unwrap_or_else(|_| "\"\"".to_string()));
        }

        header.push_str(r#","blob":""#);

        write_to_stream(&inner.output, header.as_bytes())?;
        inner.header_written = true;
        Ok(())
    }

    fn write_footer(inner: &mut ContentWriterInner) -> Result<(), StreamError> {
        // Close the blob string, close the contents object, close the contents array, close result, close JSON-RPC
        // Add newline for stdio protocol
        write_to_stream(&inner.output, b"\"]}}\n")?;
        Ok(())
    }
}

// GuestWriter trait - resource instance methods
impl GuestWriter for ContentWriter {
    fn check_write(&self) -> Result<u64, StreamError> {
        let inner = self.inner.borrow();
        if inner.closed {
            return Err(StreamError::Closed);
        }

        inner.output.check_write().map_err(|_| StreamError::Closed)
    }

    fn write(&self, _contents: Vec<u8>) -> Result<(), StreamError> {
        let mut inner = self.inner.borrow_mut();
        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Ensure header is written
        Self::write_header(&mut inner)?;

        // Add to buffer
        inner.data_buffer.extend_from_slice(&_contents);

        // Encode and write in chunks (base64 encodes 3 bytes -> 4 chars)
        // Write complete 3-byte chunks
        const CHUNK_SIZE: usize = 3072; // Produces 4096 base64 chars
        while inner.data_buffer.len() >= CHUNK_SIZE {
            let chunk_size = CHUNK_SIZE.min(inner.data_buffer.len());
            let encoded = BASE64.encode(&inner.data_buffer[..chunk_size]);
            write_to_stream(&inner.output, encoded.as_bytes())?;
            // Keep remaining bytes for next write
            inner.data_buffer.drain(..chunk_size);
        }

        Ok(())
    }

    fn close(&self, options: Option<Options>) -> Result<(), StreamError> {
        let _ = options; // Currently unused but part of the interface
        let mut inner = self.inner.borrow_mut();
        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Ensure header is written
        Self::write_header(&mut inner)?;

        // Write any remaining buffered data
        if !inner.data_buffer.is_empty() {
            let encoded = BASE64.encode(&inner.data_buffer);
            write_to_stream(&inner.output, encoded.as_bytes())?;
            inner.data_buffer.clear();
        }

        // Write footer
        Self::write_footer(&mut inner)?;

        inner.closed = true;
        inner.output.flush().map_err(|_| StreamError::Closed)?;
        Ok(())
    }
}

// Guest trait - top-level static functions
impl crate::bindings::exports::wasmcp::mcp::resources_read_result::Guest for crate::Component {
    type Writer = ContentWriter;

    fn write(
        id: Id,
        output: OutputStream,
        contents: Contents,
        _options: Option<Options>,
    ) -> Result<(), StreamError> {
        let writer = ContentWriter::new(id, output, contents.uri.clone());
        let mut inner = writer.inner.borrow_mut();

        // Set mime type if present
        if let Some(opts) = &contents.options {
            inner.mime_type = opts.mime_type.clone();
        }

        // Write header
        ContentWriter::write_header(&mut inner)?;

        // Write the data as base64
        let encoded = BASE64.encode(&contents.data);
        write_to_stream(&inner.output, encoded.as_bytes())?;

        // Write footer (options parameter is currently unused in resources-read-result)
        ContentWriter::write_footer(&mut inner)?;

        // Flush
        inner.output.flush().map_err(|_| StreamError::Closed)
    }

    fn open(
        id: Id,
        output: OutputStream,
        initial: Contents,
    ) -> Result<crate::bindings::exports::wasmcp::mcp::resources_read_result::Writer, StreamError>
    {
        let writer = ContentWriter::new(id, output, initial.uri.clone());

        // Set mime type if present
        if let Some(opts) = &initial.options {
            writer.inner.borrow_mut().mime_type = opts.mime_type.clone();
        }

        // Write header
        {
            let mut inner = writer.inner.borrow_mut();
            ContentWriter::write_header(&mut inner)?;
        }

        // Write initial data in chunks
        {
            let inner = writer.inner.borrow_mut();
            const CHUNK_SIZE: usize = 3072; // Produces 4096 base64 chars
            for chunk in initial.data.chunks(CHUNK_SIZE) {
                let encoded = BASE64.encode(chunk);
                write_to_stream(&inner.output, encoded.as_bytes())?;
            }
        }

        Ok(crate::bindings::exports::wasmcp::mcp::resources_read_result::Writer::new(writer))
    }
}
