use crate::bindings::exports::wasmcp::mcp::tools_call_content::{GuestWriter, Id, Options};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::bindings::wasmcp::mcp::content::ContentBlock;
use crate::helpers::{content_block_to_json, content_block_to_state, write_to_stream};
use crate::types::ContentBlockState;
use serde_json::json;
use std::cell::RefCell;

pub struct ContentWriter {
    inner: RefCell<ContentWriterInner>,
}

struct ContentWriterInner {
    id: Id,
    output: OutputStream,
    current_block: Option<ContentBlockState>,
    header_written: bool,
    first_block_written: bool,
    closed: bool,
}

impl ContentWriter {
    pub fn new(id: Id, output: OutputStream) -> Self {
        Self {
            inner: RefCell::new(ContentWriterInner {
                id,
                output,
                current_block: None,
                header_written: false,
                first_block_written: false,
                closed: false,
            }),
        }
    }

    fn write_header(inner: &mut ContentWriterInner) -> Result<(), StreamError> {
        if inner.header_written {
            return Ok(());
        }

        // Write the JSON-RPC envelope opening and start of content array
        let header = format!(
            r#"{{"jsonrpc":"2.0","id":{},"result":{{"content":["#,
            match &inner.id {
                Id::Number(n) => n.to_string(),
                Id::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
            }
        );

        write_to_stream(&inner.output, header.as_bytes())?;
        inner.header_written = true;
        Ok(())
    }

    fn flush_current_block(inner: &mut ContentWriterInner) -> Result<(), StreamError> {
        if let Some(state) = inner.current_block.take() {
            // Ensure header is written
            Self::write_header(inner)?;

            // Write comma if not the first block
            if inner.first_block_written {
                write_to_stream(&inner.output, b",")?;
            }

            use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
            let block = match state {
                ContentBlockState::Text { text } => json!({
                    "type": "text",
                    "text": text
                }),
                ContentBlockState::Image { data, mime_type } => json!({
                    "type": "image",
                    "data": BASE64.encode(&data),
                    "mimeType": mime_type
                }),
                ContentBlockState::Audio { data, mime_type } => json!({
                    "type": "audio",
                    "data": BASE64.encode(&data),
                    "mimeType": mime_type
                }),
                ContentBlockState::Resource {
                    uri,
                    text,
                    blob,
                    mime_type,
                } => {
                    let mut resource = json!({"uri": uri});
                    if let Some(m) = mime_type {
                        resource["mimeType"] = json!(m);
                    }
                    if let Some(t) = text {
                        resource["text"] = json!(t);
                    }
                    if let Some(b) = blob {
                        resource["blob"] = json!(BASE64.encode(&b));
                    }
                    json!({
                        "type": "resource",
                        "resource": resource
                    })
                }
            };

            // Write the block JSON
            let block_str = serde_json::to_string(&block).map_err(|_| StreamError::Closed)?;
            write_to_stream(&inner.output, block_str.as_bytes())?;

            inner.first_block_written = true;
        }
        Ok(())
    }

    fn write_footer(
        inner: &mut ContentWriterInner,
        options: Option<&Options>,
    ) -> Result<(), StreamError> {
        // Ensure header is written
        Self::write_header(inner)?;

        // Close the content array
        write_to_stream(&inner.output, b"]")?;

        // Add isError if present
        if let Some(opts) = options {
            if opts.is_error {
                write_to_stream(&inner.output, b",\"isError\":true")?;
            }
        }

        // Close the result and JSON-RPC envelope, add newline for stdio protocol
        write_to_stream(&inner.output, b"}}\n")?;
        Ok(())
    }
}

// Guest trait - top-level static functions
impl crate::bindings::exports::wasmcp::mcp::tools_call_content::Guest for crate::Component {
    type Writer = ContentWriter;

    fn write_text(
        id: Id,
        output: OutputStream,
        text: String,
        options: Option<Options>,
    ) -> Result<(), StreamError> {
        let writer = ContentWriter::new(id, output);
        let mut inner = writer.inner.borrow_mut();

        // Write header
        ContentWriter::write_header(&mut inner)?;

        // Write the text block directly
        let block = json!({
            "type": "text",
            "text": text
        });
        let block_str = serde_json::to_string(&block).map_err(|_| StreamError::Closed)?;
        write_to_stream(&inner.output, block_str.as_bytes())?;

        // Write footer
        ContentWriter::write_footer(&mut inner, options.as_ref())?;

        // Flush
        inner.output.flush().map_err(|_| StreamError::Closed)
    }

    fn write_error(id: Id, output: OutputStream, reason: String) -> Result<(), StreamError> {
        let writer = ContentWriter::new(id, output);
        let mut inner = writer.inner.borrow_mut();

        // Write header
        ContentWriter::write_header(&mut inner)?;

        // Write the error text block
        let block = json!({
            "type": "text",
            "text": reason
        });
        let block_str = serde_json::to_string(&block).map_err(|_| StreamError::Closed)?;
        write_to_stream(&inner.output, block_str.as_bytes())?;

        // Write footer with error flag
        ContentWriter::write_footer(
            &mut inner,
            Some(&Options {
                is_error: true,
                meta: None,
            }),
        )?;

        // Flush
        inner.output.flush().map_err(|_| StreamError::Closed)
    }

    fn write(
        id: Id,
        output: OutputStream,
        content: Vec<ContentBlock>,
        options: Option<Options>,
    ) -> Result<(), StreamError> {
        let writer = ContentWriter::new(id, output);
        let mut inner = writer.inner.borrow_mut();

        // Write header
        ContentWriter::write_header(&mut inner)?;

        // Write all content blocks
        for (i, block) in content.iter().enumerate() {
            if i > 0 {
                write_to_stream(&inner.output, b",")?;
            }
            let block_json = content_block_to_json(block);
            let block_str = serde_json::to_string(&block_json).map_err(|_| StreamError::Closed)?;
            write_to_stream(&inner.output, block_str.as_bytes())?;
        }

        // Write footer
        ContentWriter::write_footer(&mut inner, options.as_ref())?;

        // Flush
        inner.output.flush().map_err(|_| StreamError::Closed)
    }

    fn open(
        id: Id,
        output: OutputStream,
        initial: ContentBlock,
    ) -> Result<crate::bindings::exports::wasmcp::mcp::tools_call_content::Writer, StreamError>
    {
        let writer = ContentWriter::new(id, output);

        // Set up initial block for streaming
        {
            let mut inner = writer.inner.borrow_mut();
            inner.current_block = Some(content_block_to_state(&initial)?);
        }

        Ok(crate::bindings::exports::wasmcp::mcp::tools_call_content::Writer::new(writer))
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

    fn write(&self, contents: Vec<u8>) -> Result<(), StreamError> {
        let mut inner = self.inner.borrow_mut();
        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Append to current block if it's text
        match &mut inner.current_block {
            Some(ContentBlockState::Text { text }) => {
                let new_text = String::from_utf8(contents).map_err(|_| StreamError::Closed)?;
                text.push_str(&new_text);
            }
            Some(ContentBlockState::Image { data, .. })
            | Some(ContentBlockState::Audio { data, .. }) => {
                data.extend_from_slice(&contents);
            }
            Some(ContentBlockState::Resource { blob: Some(b), .. }) => {
                b.extend_from_slice(&contents);
            }
            _ => return Err(StreamError::Closed),
        }

        Ok(())
    }

    fn next(&self, content: ContentBlock) -> Result<(), StreamError> {
        let mut inner = self.inner.borrow_mut();
        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Flush current block and start new one
        Self::flush_current_block(&mut inner)?;
        inner.current_block = Some(content_block_to_state(&content)?);
        Ok(())
    }

    fn close(&self, options: Option<Options>) -> Result<(), StreamError> {
        let mut inner = self.inner.borrow_mut();
        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Flush any current block
        Self::flush_current_block(&mut inner)?;

        // Write footer
        Self::write_footer(&mut inner, options.as_ref())?;

        inner.closed = true;
        inner.output.flush().map_err(|_| StreamError::Closed)?;
        Ok(())
    }
}
