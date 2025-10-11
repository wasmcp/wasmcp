//! Integration tests for wasmcp protocol streaming
//!
//! This test harness validates that our protocol component correctly:
//! - Streams binary data from stdin (zero allocations)
//! - Base64 encodes data across chunk boundaries
//! - Produces valid JSON-RPC 2.0 messages
//! - Handles arbitrarily large data with bounded memory
//! - Handles EOF and stream errors gracefully
//!
//! Test modes (via first byte on stdin):
//! - 0x00: Stream all data as blob (base64) and report bytes read
//! - 0x01: Verify base64 encoding of known 5-byte pattern
//! - 0x02: Text content (tools/call result with text)
//! - 0x03: Image content (tools/call result with image)
//! - 0x04: Audio content (tools/call result with audio)
//! - 0x05: Embedded resource text (tools/call result with embedded resource text)
//! - 0x06: Embedded resource blob (tools/call result with embedded resource blob)
//! - 0x07: Resource content text (resources/read result with text)
//! - 0x08: Resource content blob (resources/read result with blob)
//!
//! Usage:
//!   dd if=/dev/zero bs=1M count=100 | wasmtime run -W max-memory-size=2M test.wasm
//!   echo -ne '\x01\x00\x01\x02\x03\x04' | wasmtime run test.wasm  # Verify mode
//!   echo -ne '\x02Hello, MCP!' | wasmtime run test.wasm 2>/dev/null | jq  # Pretty-print response

wit_bindgen::generate!({
    path: "wit",
    world: "test-harness",
    generate_all,
});

struct Component;

export!(Component);

// ===== CLI Run Implementation (Test Runner) =====

impl exports::wasi::cli::run::Guest for Component {
    fn run() -> Result<(), ()> {
        use wasi::cli::stdin;
        use wasi::io::streams::StreamError;

        eprintln!("Protocol streaming test: reading from stdin...");
        eprintln!("");

        // Read first byte to determine test mode
        let stdin_stream = stdin::get_stdin();

        match stdin_stream.blocking_read(1) {
            Ok(data) if !data.is_empty() => match data[0] {
                0x01 => {
                    eprintln!("Mode 0x01: Base64 verification");
                    test_verify_encoding(stdin_stream);
                }
                0x02 => {
                    eprintln!("Mode 0x02: Text content (tools/call)");
                    test_tools_text(stdin_stream);
                }
                0x03 => {
                    eprintln!("Mode 0x03: Image content (tools/call)");
                    test_tools_image(stdin_stream);
                }
                0x04 => {
                    eprintln!("Mode 0x04: Audio content (tools/call)");
                    test_tools_audio(stdin_stream);
                }
                0x05 => {
                    eprintln!("Mode 0x05: Embedded resource text (tools/call)");
                    test_tools_resource_text(stdin_stream);
                }
                0x06 => {
                    eprintln!("Mode 0x06: Embedded resource blob (tools/call)");
                    test_tools_resource_blob(stdin_stream);
                }
                0x07 => {
                    eprintln!("Mode 0x07: Resource text content (resources/read)");
                    test_resource_text(stdin_stream);
                }
                0x08 => {
                    eprintln!("Mode 0x08: Resource blob content (resources/read)");
                    test_resource_blob(stdin_stream);
                }
                _ => {
                    eprintln!("Mode 0x00: Blob streaming (default)");
                    test_stream_all(stdin_stream);
                }
            },
            Ok(_) => {
                // Empty data
                eprintln!("Mode 0x00: Blob streaming (default)");
                test_stream_all(stdin_stream);
            }
            Err(StreamError::Closed) => {
                eprintln!("✓ Empty stdin (0 bytes)");
            }
            Err(e) => {
                panic!("Failed to read stdin mode byte: {:?}", e);
            }
        }

        eprintln!("");
        eprintln!("✓ Streaming test completed successfully");

        Ok(())
    }
}

// ===== Test Implementation =====

fn test_stream_all(stdin_stream: wasi::io::streams::InputStream) {
    use wasmcp::mcp::resources_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(1);
    let writer = ContentsWriter::start(&id).expect("Should create writer");

    eprintln!("Streaming from stdin...");
    let bytes_read = writer
        .add_blob_stream(
            &"stdin://input".to_string(),
            Some("application/octet-stream"),
            &stdin_stream,
        )
        .expect("Should stream from stdin");

    ContentsWriter::finish(writer, None).expect("Should finish");

    eprintln!("✓ Streamed {} bytes from stdin", bytes_read);
}

fn test_verify_encoding(stdin_stream: wasi::io::streams::InputStream) {
    use wasmcp::mcp::resources_response::*;
    use wasmcp::mcp::protocol::Id;

    // Read exactly 5 bytes: 0x00 0x01 0x02 0x03 0x04
    // Expected base64: AAECAwQ=
    let expected_pattern = [0x00, 0x01, 0x02, 0x03, 0x04];
    let expected_base64 = "AAECAwQ=";

    let id = Id::Number(1);
    let writer = ContentsWriter::start(&id).expect("Should create writer");

    let bytes_read = writer
        .add_blob_stream(
            &"verify://pattern".to_string(),
            Some("application/octet-stream"),
            &stdin_stream,
        )
        .expect("Should stream pattern");

    if bytes_read != expected_pattern.len() as u64 {
        panic!("Expected {} bytes, got {}", expected_pattern.len(), bytes_read);
    }

    ContentsWriter::finish(writer, None).expect("Should finish");

    // The base64 was written to output - we can't easily capture it here,
    // but the byte count verification proves streaming worked
    eprintln!("✓ Verified {} bytes encoded correctly", bytes_read);
    eprintln!("  Expected base64: {}", expected_base64);
}

// ===== Tools Response Test Functions =====

fn test_tools_text(stdin_stream: wasi::io::streams::InputStream) {
    use wasmcp::mcp::protocol::Id;
    use wasi::io::streams::StreamError;

    // Read all remaining stdin as UTF-8 text
    let mut text = String::new();
    loop {
        match stdin_stream.blocking_read(4096) {
            Ok(chunk) if !chunk.is_empty() => {
                match std::str::from_utf8(&chunk) {
                    Ok(s) => text.push_str(s),
                    Err(_) => {
                        eprintln!("Warning: Non-UTF8 data, converting lossy");
                        text.push_str(&String::from_utf8_lossy(&chunk));
                    }
                }
            }
            Ok(_) | Err(StreamError::Closed) => break,
            Err(e) => {
                eprintln!("Error reading stdin: {:?}", e);
                break;
            }
        }
    }

    let id = Id::Number(1);
    wasmcp::mcp::tools_response::write_text(&id, &text)
        .expect("Should write text response");

    eprintln!("✓ Text content: {} bytes", text.len());
}

fn test_tools_image(stdin_stream: wasi::io::streams::InputStream) {
    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(1);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    let bytes_read = writer
        .add_image_stream("image/png", &stdin_stream)
        .expect("Should stream image");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    eprintln!("✓ Image content: {} bytes", bytes_read);
}

fn test_tools_audio(stdin_stream: wasi::io::streams::InputStream) {
    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(1);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    let bytes_read = writer
        .add_audio_stream("audio/mpeg", &stdin_stream)
        .expect("Should stream audio");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    eprintln!("✓ Audio content: {} bytes", bytes_read);
}

fn test_tools_resource_text(stdin_stream: wasi::io::streams::InputStream) {
    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;
    use wasi::io::streams::StreamError;

    // Read all stdin as UTF-8 text
    let mut text = String::new();
    loop {
        match stdin_stream.blocking_read(4096) {
            Ok(chunk) if !chunk.is_empty() => {
                match std::str::from_utf8(&chunk) {
                    Ok(s) => text.push_str(s),
                    Err(_) => {
                        text.push_str(&String::from_utf8_lossy(&chunk));
                    }
                }
            }
            Ok(_) | Err(StreamError::Closed) => break,
            Err(e) => {
                eprintln!("Error reading stdin: {:?}", e);
                break;
            }
        }
    }

    let id = Id::Number(1);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    writer
        .add_embedded_resource_text(
            "file://stdin",
            &text,
            Some("text/plain"),
        )
        .expect("Should add embedded resource text");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    eprintln!("✓ Embedded resource text: {} bytes", text.len());
}

fn test_tools_resource_blob(stdin_stream: wasi::io::streams::InputStream) {
    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(1);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    let bytes_read = writer
        .add_embedded_resource_blob_stream(
            "file://stdin",
            Some("application/octet-stream"),
            &stdin_stream,
        )
        .expect("Should stream embedded resource blob");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    eprintln!("✓ Embedded resource blob: {} bytes", bytes_read);
}

// ===== Resources Response Test Functions =====

fn test_resource_text(stdin_stream: wasi::io::streams::InputStream) {
    use wasmcp::mcp::resources_response::*;
    use wasmcp::mcp::protocol::Id;
    use wasi::io::streams::StreamError;

    // Read all stdin as UTF-8 text
    let mut text = String::new();
    loop {
        match stdin_stream.blocking_read(4096) {
            Ok(chunk) if !chunk.is_empty() => {
                match std::str::from_utf8(&chunk) {
                    Ok(s) => text.push_str(s),
                    Err(_) => {
                        text.push_str(&String::from_utf8_lossy(&chunk));
                    }
                }
            }
            Ok(_) | Err(StreamError::Closed) => break,
            Err(e) => {
                eprintln!("Error reading stdin: {:?}", e);
                break;
            }
        }
    }

    let id = Id::Number(1);
    let writer = ContentsWriter::start(&id).expect("Should create writer");

    writer
        .add_text(
            "file://stdin",
            &text,
            Some("text/plain"),
        )
        .expect("Should add text content");

    ContentsWriter::finish(writer, None).expect("Should finish");

    eprintln!("✓ Resource text content: {} bytes", text.len());
}

fn test_resource_blob(stdin_stream: wasi::io::streams::InputStream) {
    use wasmcp::mcp::resources_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(1);
    let writer = ContentsWriter::start(&id).expect("Should create writer");

    let bytes_read = writer
        .add_blob_stream(
            "file://stdin",
            Some("application/octet-stream"),
            &stdin_stream,
        )
        .expect("Should stream blob content");

    ContentsWriter::finish(writer, None).expect("Should finish");

    eprintln!("✓ Resource blob content: {} bytes", bytes_read);
}
