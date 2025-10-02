use crate::bindings::exports::wasmcp::mcp::resource_templates_list_result::{
    GuestWriter, Id, Options, Template,
};
use crate::bindings::wasi::io::streams::{OutputStream, StreamError};
use crate::helpers::{resource_template_to_json, write_to_stream};
use std::cell::RefCell;
use std::collections::VecDeque;

pub struct TemplatesListWriter {
    inner: RefCell<TemplatesListWriterInner>,
}

struct TemplatesListWriterInner {
    id: Id,
    output: OutputStream,
    pending_templates: VecDeque<Template>,
    written_count: u32,
    closed: bool,
    header_written: bool,
    first_template_written: bool,
}

impl TemplatesListWriter {
    pub fn new(id: Id, output: OutputStream, initial: Vec<Template>) -> Self {
        Self {
            inner: RefCell::new(TemplatesListWriterInner {
                id,
                output,
                pending_templates: initial.into_iter().collect(),
                written_count: 0,
                closed: false,
                header_written: false,
                first_template_written: false,
            }),
        }
    }

    fn write_header(inner: &mut TemplatesListWriterInner) -> Result<(), StreamError> {
        if inner.header_written {
            return Ok(());
        }

        // Write the JSON-RPC envelope opening and start of resourceTemplates array
        let header = format!(
            r#"{{"jsonrpc":"2.0","id":{},"result":{{"resourceTemplates":["#,
            match &inner.id {
                Id::Number(n) => n.to_string(),
                Id::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
            }
        );

        write_to_stream(&inner.output, header.as_bytes())?;
        inner.header_written = true;
        Ok(())
    }

    fn write_single_template(
        inner: &mut TemplatesListWriterInner,
        template: &Template,
    ) -> Result<(), StreamError> {
        // Write comma if not the first template
        if inner.first_template_written {
            write_to_stream(&inner.output, b",")?;
        }

        // Write the template JSON
        let template_json = resource_template_to_json(template);
        let template_str =
            serde_json::to_string(&template_json).map_err(|_| StreamError::Closed)?;
        write_to_stream(&inner.output, template_str.as_bytes())?;

        inner.first_template_written = true;
        inner.written_count += 1;
        Ok(())
    }

    fn write_footer(
        inner: &mut TemplatesListWriterInner,
        options: Option<&Options>,
    ) -> Result<(), StreamError> {
        // Close the resourceTemplates array
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
impl GuestWriter for TemplatesListWriter {
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

        // We can write one template at a time when streaming
        Ok(if inner.pending_templates.is_empty() {
            0
        } else {
            1
        })
    }

    fn write(&self, template: Template) -> Result<(), StreamError> {
        let mut inner = self.inner.borrow_mut();

        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Ensure header is written
        Self::write_header(&mut inner)?;

        // Write the template immediately
        Self::write_single_template(&mut inner, &template)?;

        Ok(())
    }

    fn close(&self, options: Option<Options>) -> Result<(), StreamError> {
        let mut inner = self.inner.borrow_mut();
        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Ensure header is written
        Self::write_header(&mut inner)?;

        // Write any remaining templates
        while let Some(template) = inner.pending_templates.pop_front() {
            Self::write_single_template(&mut inner, &template)?;
        }

        // Write the footer
        Self::write_footer(&mut inner, options.as_ref())?;

        inner.closed = true;
        inner.output.flush().map_err(|_| StreamError::Closed)?;
        Ok(())
    }
}

// Guest trait - top-level static functions
impl crate::bindings::exports::wasmcp::mcp::resource_templates_list_result::Guest
    for crate::Component
{
    type Writer = TemplatesListWriter;

    fn write(
        id: Id,
        output: OutputStream,
        templates: Vec<Template>,
        options: Option<Options>,
    ) -> Result<(), StreamError> {
        let writer = TemplatesListWriter::new(id, output, vec![]);
        let mut inner = writer.inner.borrow_mut();

        // For write, we stream everything in one go
        TemplatesListWriter::write_header(&mut inner)?;

        // Write all templates
        for template in &templates {
            TemplatesListWriter::write_single_template(&mut inner, template)?;
        }

        // Write footer
        TemplatesListWriter::write_footer(&mut inner, options.as_ref())?;

        // Flush the stream
        inner.output.flush().map_err(|_| StreamError::Closed)
    }

    fn open(
        id: Id,
        output: OutputStream,
    ) -> Result<
        crate::bindings::exports::wasmcp::mcp::resource_templates_list_result::Writer,
        StreamError,
    > {
        // NOTE: resource-templates open() has NO initial parameter - different from others
        let writer = TemplatesListWriter::new(id, output, vec![]);
        {
            let mut inner = writer.inner.borrow_mut();

            // Write the header to start streaming
            TemplatesListWriter::write_header(&mut inner)?;
        }

        Ok(
            crate::bindings::exports::wasmcp::mcp::resource_templates_list_result::Writer::new(
                writer,
            ),
        )
    }
}
