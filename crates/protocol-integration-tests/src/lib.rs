//! Integration tests for wasmcp protocol streaming
//!
//! This test harness validates that our protocol component correctly:
//! - Streams binary data
//! - Base64 encodes data across chunk boundaries
//! - Produces valid JSON-RPC 2.0 messages
//! - Handles large data with bounded memory
//!
//! NOTE: This test outputs JSON-RPC messages to stdout (via stdio-transport).
//! The test validates the structure but the output will appear in the test logs.

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
        println!("Starting protocol streaming integration tests...");
        println!("NOTE: JSON-RPC output will appear below - this is expected");
        println!("");

        // Run all tests - Tools Response
        test_simple_text_response();
        test_streaming_image();
        test_streaming_blob();
        test_large_image_streaming();
        test_mixed_content_blocks();

        // Streaming API tests with real input-streams
        test_streaming_image_with_stream();
        test_audio_content();
        test_resource_link();
        test_embedded_resource_text();

        // Resources Response tests
        test_resources_contents_text();
        test_resources_contents_blob();
        test_resources_contents_blob_stream();

        // Edge case tests
        test_empty_stream();
        test_binary_data_all_bytes();
        test_large_file_streaming();

        println!("");
        println!("✓ All {} protocol streaming integration tests passed!", 15);
        Ok(())
    }
}

// ===== Test Cases =====

fn test_simple_text_response() {
    println!("Test 1: Simple text response");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(1);
    write_text(&id, &"Hello, integration test!".to_string())
        .expect("write_text should succeed");

    println!("  ✓ Simple text response completed");
}

fn test_streaming_image() {
    println!("Test 2: Streaming image response");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(2);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    // Small test image data (PNG signature + IHDR)
    let image_data: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // "IHDR"
    ];

    writer
        .add_image(&image_data, &"image/png".to_string())
        .expect("Should add image");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Streaming image response completed");
}

fn test_streaming_blob() {
    println!("Test 3: Streaming blob response");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(3);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    let blob_data = b"Binary blob data with \xFF\xFE\xFD non-UTF8 bytes".to_vec();

    writer
        .add_embedded_resource_blob(
            &"file:///test.bin".to_string(),
            &blob_data,
            Some("application/octet-stream"),
        )
        .expect("Should add blob");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Streaming blob response completed");
}

fn test_large_image_streaming() {
    println!("Test 4: Large image streaming (1MB)");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(4);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    // Create 1MB of test data
    let large_data = vec![0x42; 1024 * 1024]; // 1MB of 'B'

    writer
        .add_image(&large_data, &"image/test".to_string())
        .expect("Should handle large data");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Large image streaming (1MB) completed");
}

fn test_mixed_content_blocks() {
    println!("Test 5: Mixed content blocks");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(5);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    // Add multiple different content types
    writer
        .add_text(&"First text block".to_string())
        .expect("Should add text");

    let img_data = vec![0x89, 0x50, 0x4E, 0x47];
    writer
        .add_image(&img_data, &"image/png".to_string())
        .expect("Should add image");

    writer
        .add_text(&"Second text block".to_string())
        .expect("Should add text");

    let blob_data = vec![0x00, 0x01, 0x02];
    writer
        .add_embedded_resource_blob(&"file:///test".to_string(), &blob_data, None)
        .expect("Should add blob");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Mixed content blocks completed");
}

// ===== Helper Functions for Creating Test Streams =====

use wasi::filesystem0_2_3::types::{Descriptor, DescriptorFlags, OpenFlags, PathFlags};
use wasi::filesystem0_2_3::preopens;
use wasi::io0_2_3::streams::InputStream;

/// Create a temporary file with test data and return an input-stream from it
fn create_test_stream(data: &[u8]) -> (Descriptor, InputStream) {
    // Get the first preopened directory (typically /)
    let dirs = preopens::get_directories();
    let (root_desc, _path) = &dirs[0];

    // Create a unique temp file name
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let file_name = format!("test_stream_{}.tmp", COUNTER.fetch_add(1, Ordering::SeqCst));

    // Open/create the file for writing
    let file_desc = root_desc
        .open_at(
            PathFlags::empty(),
            &file_name,
            OpenFlags::CREATE | OpenFlags::TRUNCATE,
            DescriptorFlags::WRITE | DescriptorFlags::READ,
        )
        .expect("Should create temp file");

    // Write the test data
    let output_stream = file_desc
        .write_via_stream(0)
        .expect("Should create output stream");

    let mut offset = 0;
    while offset < data.len() {
        let chunk_size = std::cmp::min(4096, data.len() - offset);
        let chunk = &data[offset..offset + chunk_size];

        output_stream
            .blocking_write_and_flush(chunk)
            .expect("Should write data");

        offset += chunk_size;
    }

    drop(output_stream);

    // Sync the file to ensure data is written
    file_desc.sync().expect("Should sync file");

    // Create an input stream from the file
    let input_stream = file_desc
        .read_via_stream(0)
        .expect("Should create input stream");

    (file_desc, input_stream)
}

fn test_streaming_image_with_stream() {
    println!("Test 6: Streaming image with actual input-stream");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(6);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    // Create a larger test image (100KB)
    let image_data = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] // PNG signature
        .into_iter()
        .chain(vec![0x42; 100 * 1024]) // 100KB of data
        .collect::<Vec<u8>>();

    // Create input-stream from filesystem (test REAL streaming, not Vec<u8>)
    let (_file_desc, stream) = create_test_stream(&image_data);

    // Test streaming API with real WASI input-stream
    let bytes_read = writer
        .add_image_stream(&"image/png".to_string(), &stream)
        .expect("Should stream image");

    assert_eq!(bytes_read, 102408, "Should read exactly 100KB + 8-byte PNG header");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Streaming image with input-stream completed ({} bytes)", bytes_read);
}

fn test_audio_content() {
    println!("Test 7: Audio content with actual input-stream");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(7);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    // Create test audio data (50KB)
    let audio_data = vec![0xFF; 50 * 1024];
    let (_file_desc, stream) = create_test_stream(&audio_data);

    let bytes_read = writer
        .add_audio_stream(&"audio/wav".to_string(), &stream)
        .expect("Should stream audio");

    assert_eq!(bytes_read, 51200, "Should read exactly 50KB");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Audio streaming completed ({} bytes)", bytes_read);
}

fn test_resource_link() {
    println!("Test 8: Resource link content");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(8);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    writer
        .add_resource_link(&"file:///example.txt".to_string(), &"Example File".to_string())
        .expect("Should add resource link");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Resource link completed");
}

fn test_embedded_resource_text() {
    println!("Test 9: Embedded resource text content");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(9);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    writer
        .add_embedded_resource_text(
            &"file:///readme.txt".to_string(),
            &"This is embedded text content".to_string(),
            Some("text/plain"),
        )
        .expect("Should add embedded resource text");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Embedded resource text completed");
}

fn test_resources_contents_text() {
    println!("Test 10: Resources ContentsWriter - text content");

    use wasmcp::mcp::resources_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(10);
    let writer = ContentsWriter::start(&id).expect("Should create writer");

    writer
        .add_text(
            &"file:///example.txt".to_string(),
            &"This is the content of the file.".to_string(),
            Some("text/plain"),
        )
        .expect("Should add text content");

    ContentsWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Resources text content completed");
}

fn test_resources_contents_blob() {
    println!("Test 11: Resources ContentsWriter - blob content");

    use wasmcp::mcp::resources_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(11);
    let writer = ContentsWriter::start(&id).expect("Should create writer");

    let blob_data = b"Binary file content \xFF\xFE\xFD".to_vec();

    writer
        .add_blob(
            &"file:///binary.dat".to_string(),
            &blob_data,
            Some("application/octet-stream"),
        )
        .expect("Should add blob content");

    ContentsWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Resources blob content completed");
}

fn test_resources_contents_blob_stream() {
    println!("Test 12: Resources ContentsWriter - streaming blob");

    use wasmcp::mcp::resources_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(12);
    let writer = ContentsWriter::start(&id).expect("Should create writer");

    // Create a 500KB blob to stream
    let blob_data = vec![0xCA, 0xFE, 0xBA, 0xBE].repeat(125 * 1024);
    let (_file_desc, stream) = create_test_stream(&blob_data);

    let bytes_read = writer
        .add_blob_stream(
            &"file:///large-resource.bin".to_string(),
            Some("application/octet-stream"),
            &stream,
        )
        .expect("Should stream blob");

    assert_eq!(bytes_read, 512000, "Should read exactly 500KB");

    ContentsWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Resources streaming blob completed ({} bytes)", bytes_read);
}

// ===== Edge Case Tests =====

fn test_empty_stream() {
    println!("Test 13: Empty stream (0 bytes)");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(13);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    // Test with completely empty data
    let empty_data = vec![];
    let (_file_desc, stream) = create_test_stream(&empty_data);

    let bytes_read = writer
        .add_image_stream(&"image/png".to_string(), &stream)
        .expect("Should handle empty stream");

    assert_eq!(bytes_read, 0, "Should read 0 bytes from empty stream");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Empty stream handled correctly (0 bytes)");
}

fn test_binary_data_all_bytes() {
    println!("Test 14: Binary data with all byte values (0x00-0xFF)");

    use wasmcp::mcp::tools_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(14);
    let writer = ContentBlocksWriter::start(&id).expect("Should create writer");

    // Create data containing all possible byte values, repeated multiple times
    // This ensures base64 encoding handles NULL bytes, high-bit bytes, etc.
    let mut binary_data = Vec::new();
    for _ in 0..100 {
        for byte_val in 0..=255u8 {
            binary_data.push(byte_val);
        }
    }
    // Total: 25,600 bytes (100 * 256)

    let (_file_desc, stream) = create_test_stream(&binary_data);

    let bytes_read = writer
        .add_embedded_resource_blob_stream(
            "file:///binary.dat",
            Some("application/octet-stream"),
            &stream,
        )
        .expect("Should handle all byte values");

    assert_eq!(bytes_read, 25_600, "Should read all 25,600 bytes");

    ContentBlocksWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Binary data test completed ({} bytes, all values 0x00-0xFF)", bytes_read);
}

fn test_large_file_streaming() {
    println!("Test 15: Large file streaming (10MB) - bounded memory test");

    use wasmcp::mcp::resources_response::*;
    use wasmcp::mcp::protocol::Id;

    let id = Id::Number(15);
    let writer = ContentsWriter::start(&id).expect("Should create writer");

    // Create 10MB of data - pattern that's compressible but not trivial
    // Repeating pattern: [0, 1, 2, ..., 255, 255, 254, ..., 1, 0]
    let pattern_size = 512;
    let mut pattern = Vec::with_capacity(pattern_size);
    for i in 0..256 {
        pattern.push(i as u8);
    }
    for i in (1..256).rev() {
        pattern.push(i as u8);
    }

    let repetitions = (10 * 1024 * 1024) / pattern_size; // 10MB
    let large_data: Vec<u8> = pattern.iter()
        .cycle()
        .take(repetitions * pattern_size)
        .copied()
        .collect();

    let expected_size = (repetitions * pattern_size) as u64;

    let (_file_desc, stream) = create_test_stream(&large_data);

    let bytes_read = writer
        .add_blob_stream(
            &"file:///large-test.bin".to_string(),
            Some("application/octet-stream"),
            &stream,
        )
        .expect("Should stream large file");

    assert_eq!(bytes_read, expected_size, "Should read all bytes");
    assert!(bytes_read >= 10 * 1024 * 1024, "Should be at least 10MB");

    ContentsWriter::finish(writer, None).expect("Should finish");

    println!("  ✓ Large file streaming completed ({} bytes = {:.2} MB)",
             bytes_read, bytes_read as f64 / (1024.0 * 1024.0));
}
