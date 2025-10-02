use crate::bindings::exports::wasmcp::mcp::tools_list_result::{
    GuestWriter, Id, Options, OutputStream, StreamError, Tool,
};
use crate::helpers::{tool_to_json, write_to_stream};
use std::cell::RefCell;
use std::collections::VecDeque;

pub struct ListWriter {
    inner: RefCell<ListWriterInner>,
}

struct ListWriterInner {
    id: Id,
    output: OutputStream,
    pending_tools: VecDeque<Tool>,
    written_count: u32,
    closed: bool,
    header_written: bool,
    first_tool_written: bool,
}

impl ListWriter {
    pub fn new(id: Id, output: OutputStream, initial: Vec<Tool>) -> Self {
        Self {
            inner: RefCell::new(ListWriterInner {
                id,
                output,
                pending_tools: initial.into_iter().collect(),
                written_count: 0,
                closed: false,
                header_written: false,
                first_tool_written: false,
            }),
        }
    }

    fn write_header(inner: &mut ListWriterInner) -> Result<(), StreamError> {
        if inner.header_written {
            return Ok(());
        }

        // Write the JSON-RPC envelope opening and start of tools array
        let header = format!(
            r#"{{"jsonrpc":"2.0","id":{},"result":{{"tools":["#,
            match &inner.id {
                Id::Number(n) => n.to_string(),
                Id::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string()),
            }
        );

        write_to_stream(&inner.output, header.as_bytes())?;
        inner.header_written = true;
        Ok(())
    }

    fn write_single_tool(inner: &mut ListWriterInner, tool: &Tool) -> Result<(), StreamError> {
        // Write comma if not the first tool
        if inner.first_tool_written {
            write_to_stream(&inner.output, b",")?;
        }

        // Write the tool JSON
        let tool_json = tool_to_json(tool);
        let tool_str = serde_json::to_string(&tool_json).map_err(|_| StreamError::Closed)?;
        write_to_stream(&inner.output, tool_str.as_bytes())?;

        inner.first_tool_written = true;
        inner.written_count += 1;
        Ok(())
    }

    fn write_footer(
        inner: &mut ListWriterInner,
        options: Option<&Options>,
    ) -> Result<(), StreamError> {
        // Close the tools array
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

        // We can write one tool at a time when streaming
        Ok(if inner.pending_tools.is_empty() { 0 } else { 1 })
    }

    fn write(&self, tool: Tool) -> Result<(), StreamError> {
        let mut inner = self.inner.borrow_mut();

        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Ensure header is written
        Self::write_header(&mut inner)?;

        // Write the tool immediately
        Self::write_single_tool(&mut inner, &tool)?;

        Ok(())
    }

    fn close(&self, options: Option<Options>) -> Result<(), StreamError> {
        let mut inner = self.inner.borrow_mut();
        if inner.closed {
            return Err(StreamError::Closed);
        }

        // Ensure header is written
        Self::write_header(&mut inner)?;

        // Write any remaining tools
        while let Some(tool) = inner.pending_tools.pop_front() {
            Self::write_single_tool(&mut inner, &tool)?;
        }

        // Write the footer
        Self::write_footer(&mut inner, options.as_ref())?;

        inner.closed = true;
        inner.output.flush().map_err(|_| StreamError::Closed)?;
        Ok(())
    }
}

impl crate::bindings::exports::wasmcp::mcp::tools_list_result::Guest for crate::Component {
    type Writer = ListWriter;

    fn write(
        id: Id,
        output: OutputStream,
        tools: Vec<Tool>,
        options: Option<Options>,
    ) -> Result<(), StreamError> {
        let writer = ListWriter::new(id, output, vec![]);
        let mut inner = writer.inner.borrow_mut();

        // For write, we stream everything in one go
        ListWriter::write_header(&mut inner)?;

        // Write all tools
        for tool in &tools {
            ListWriter::write_single_tool(&mut inner, tool)?;
        }

        // Write footer
        ListWriter::write_footer(&mut inner, options.as_ref())?;

        // Flush the stream
        inner.output.flush().map_err(|_| StreamError::Closed)
    }

    fn open(
        id: Id,
        output: OutputStream,
        initial: Vec<Tool>,
    ) -> Result<crate::bindings::exports::wasmcp::mcp::tools_list_result::Writer, StreamError> {
        let writer = ListWriter::new(id, output, initial);
        {
            let mut inner = writer.inner.borrow_mut();

            // Write the header to start streaming
            ListWriter::write_header(&mut inner)?;

            // Write initial tools if any
            while let Some(tool) = inner.pending_tools.pop_front() {
                ListWriter::write_single_tool(&mut inner, &tool)?;

                // Break after writing a few to allow for backpressure
                if inner.written_count >= 5 {
                    break;
                }
            }
        }

        Ok(crate::bindings::exports::wasmcp::mcp::tools_list_result::Writer::new(writer))
    }
}
