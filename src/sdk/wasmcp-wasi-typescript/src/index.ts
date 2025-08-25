/**
 * WASI SDK for MCP (Model Context Protocol) WebAssembly components
 * 
 * This SDK provides WASI-compatible implementations for:
 * - HTTP client (via fetch)
 * - Key-value storage (Spin-specific)
 * - Configuration access
 */

// Re-export all modules
export * as http from './http.js';
export * as keyvalue from './keyvalue.js';
export * as config from './config.js';

// Export types
export type { Store } from './keyvalue.js';
export type { HttpResponse, RequestOptions } from './http.js';