/**
 * Weather-ts MCP Provider
 * 
 * A TypeScript implementation of an MCP provider using the WebAssembly Component Model.
 * This file re-exports the capability implementations for jco to use when building
 * the component.
 * 
 * TypeScript/JavaScript with jco uses a different pattern than Rust or Python:
 * - Rust uses Guest traits implemented on a Component struct
 * - Python uses classes that are instantiated by componentize-py
 * - TypeScript uses plain function exports that jco wires up
 * 
 * The Component Model's stateless nature means each function call is independent,
 * with no shared state between calls.
 */

// jco expects exports to be grouped by interface name.
// Each WIT interface needs to be exported as an object with its methods.

import * as lifecycleImpl from './capabilities/lifecycle.js';
import * as authorizationImpl from './capabilities/authorization.js';
import * as toolsImpl from './capabilities/tools.js';

// Export lifecycle interface
export const lifecycle = {
  initialize: lifecycleImpl.initialize,
  clientInitialized: lifecycleImpl.clientInitialized,
  shutdown: lifecycleImpl.shutdown,
};

// Export authorization interface
export const authorization = {
  getAuthConfig: authorizationImpl.getAuthConfig,
  jwksCacheGet: authorizationImpl.jwksCacheGet,
  jwksCacheSet: authorizationImpl.jwksCacheSet,
};

// Export tools interface
export const tools = {
  listTools: toolsImpl.listTools,
  callTool: toolsImpl.callTool,
};

/**
 * Component Model Integration Notes:
 * 
 * jco (JavaScript Component Objects) is the TypeScript/JavaScript toolchain for
 * WebAssembly Components, similar to:
 * - cargo-component for Rust
 * - componentize-py for Python
 * - wit-bindgen-go for Go
 * 
 * Key differences in TypeScript:
 * 1. Type generation: jco generates .d.ts files from WIT
 * 2. Async handling: Component Model exports are synchronous, requiring workarounds
 * 3. Variant types: Represented as discriminated unions with { tag, val }
 * 4. Result types: Handled transparently (return success, throw errors)
 * 5. Option types: Native undefined/null mapping
 * 
 * Build process:
 * 1. Bundle TypeScript to JavaScript (webpack/esbuild)
 * 2. Use jco componentize to create WebAssembly component
 * 3. Compose with transport using wac
 */