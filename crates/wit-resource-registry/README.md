# wit-resource-registry

Generic resource registry for managing WIT resource lifecycles in WebAssembly components.

## Overview

This crate provides a reusable solution for managing WebAssembly Interface Types (WIT) resources that need to be accessed in static methods after consumption.

### The Problem

WIT resources with static methods like `finish: static func(this: T) -> R` consume the resource handle, and wit-bindgen doesn't provide a way to access the underlying implementation once consumed.

### The Solution

`ResourceRegistry<T>` stores resource data in `Arc<Mutex<T>>` and maintains a mapping from handle IDs to resource data, allowing static methods to retrieve the underlying implementation.

## Usage

```rust
use std::sync::{Arc, Mutex};
use wit_resource_registry::ResourceRegistry;

// Your resource implementation
pub struct SpanInner {
    name: String,
    // ... other fields
}

// Global registry instance
static SPAN_REGISTRY: Mutex<Option<ResourceRegistry<SpanInner>>> = Mutex::new(None);

// Register resource with handle ID from wit-bindgen
pub fn register_span(handle: u32, span_data: Arc<Mutex<SpanInner>>) {
    let mut registry = SPAN_REGISTRY.lock().unwrap();
    if registry.is_none() {
        *registry = Some(ResourceRegistry::new());
    }
    registry.as_mut().unwrap().insert(handle, span_data);
}

// Retrieve resource in static methods
pub fn get_span(handle: u32) -> Option<Arc<Mutex<SpanInner>>> {
    let registry = SPAN_REGISTRY.lock().unwrap();
    registry.as_ref().and_then(|r| r.get(handle))
}
```

## Features

- **Generic**: Works with any resource type `T`
- **Thread-safe**: Uses `Arc<Mutex<T>>` for safe shared access
- **Zero-cost abstraction**: No runtime overhead beyond standard library types
- **Signal-agnostic**: Reusable across OpenTelemetry signals (traces, logs, metrics)

## Use Cases

This pattern is particularly useful for:

- OpenTelemetry trace spans and exporters
- OpenTelemetry log records and exporters
- OpenTelemetry metrics and exporters
- Any WIT resources requiring lifecycle management beyond wit-bindgen's built-in support

## License

MIT OR Apache-2.0
