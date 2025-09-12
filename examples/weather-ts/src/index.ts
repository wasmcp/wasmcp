/**
 * weather-ts MCP Provider
 * 
 * A TypeScript implementation of an MCP provider using the WebAssembly Component Model.
 * This file exports the capability implementations in the pattern jco expects.
 * While we need to group by interface name, we can do it more concisely.
 */

// Direct imports and re-exports - more idiomatic TypeScript
import * as lifecycle from './capabilities/lifecycle.js';
import * as authorization from './capabilities/authorization.js';
import * as tools from './capabilities/tools.js';

// jco requires exports to be grouped by interface name
export { lifecycle, authorization, tools };

/**
 * Component Model Integration with jco
 * 
 * Key advantages of TypeScript/JavaScript with jco:
 * 
 * 1. **Native async support** - Just use async/await and fetch()
 * 2. **Natural concurrency** - Promise.all() works as expected
 * 3. **Type safety** - Full TypeScript types from WIT
 * 4. **Familiar patterns** - Feels like normal JavaScript development
 * 
 * The Component Model's constraints:
 * - Stateless exports (no global state between calls)
 * - WIT type system (strings for JSON)
 * - WebAssembly sandbox (no direct filesystem/network except through WASI)
 * 
 * But jco makes these constraints feel natural by:
 * - Transparently handling async-to-sync bridging
 * - Providing native fetch() that works through WASI HTTP
 * - Generating idiomatic TypeScript types
 */