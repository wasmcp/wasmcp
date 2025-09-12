// Package wasihttp provides HTTP client functionality for WebAssembly components.
//
// Why this package exists:
// TinyGo compiles to WebAssembly core modules which are single-threaded.
// While Go's goroutines work, they provide cooperative concurrency, not parallelism.
// In Wasm, http.Get() blocks the entire module until completion, even in goroutines.
// 
// This package solves the problem by using WASI's poll API, which allows the
// host runtime to handle multiple I/O operations in parallel while the Wasm
// module waits. This achieves true concurrent HTTP requests.
//
// Exports:
//   - Transport: An http.RoundTripper that routes through WASI HTTP
//   - RequestsConcurrently: Execute multiple HTTP requests in parallel
//   - GetConcurrently: Convenience function for concurrent GET requests
//
// Usage:
//
//	// Set up the transport (typically in init())
//	http.DefaultTransport = &wasihttp.Transport{}
//
//	// Use standard Go HTTP client for single requests
//	resp, err := http.Get("https://example.com")
//
//	// For concurrent GET requests
//	responses := wasihttp.GetConcurrently(urls)
//
//	// For concurrent requests with any method
//	requests := []*http.Request{
//	    mustNewRequest("GET", "https://api1.com", nil),
//	    mustNewRequest("POST", "https://api2.com", body),
//	}
//	responses := wasihttp.RequestsConcurrently(requests)
package wasihttp