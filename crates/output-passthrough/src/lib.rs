//! Simple passthrough output implementation for testing
//!
//! Writes messages directly to stdout with newline delimiters.

wit_bindgen::generate!({
    path: "wit",
    world: "output-passthrough",
    generate_all,
});

struct Component;

export!(Component);

impl exports::wasmcp::mcp::output::Guest for Component {
    fn start_message() -> Result<(), exports::wasmcp::mcp::output::IoError> {
        // Nothing to do - just start accumulating
        Ok(())
    }

    fn write_message_contents(
        data: Vec<u8>,
    ) -> Result<(), exports::wasmcp::mcp::output::IoError> {
        use wasi::cli::stdout;

        let out = stdout::get_stdout();

        // Write data in chunks to respect blocking_write_and_flush 4KB limit
        const CHUNK_SIZE: usize = 4096;
        for chunk in data.chunks(CHUNK_SIZE) {
            out.blocking_write_and_flush(chunk)
                .map_err(exports::wasmcp::mcp::output::IoError::Stream)?;
        }

        Ok(())
    }

    fn finish_message() -> Result<(), exports::wasmcp::mcp::output::IoError> {
        use wasi::cli::stdout;

        let out = stdout::get_stdout();

        // Write newline delimiter and flush
        out.blocking_write_and_flush(b"\n")
            .map_err(exports::wasmcp::mcp::output::IoError::Stream)?;

        Ok(())
    }
}
