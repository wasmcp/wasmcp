/**
 * {{project-name | kebab_case}} MCP Provider
 * 
 * TypeScript implementation of an MCP provider using the WebAssembly Component Model.
 */

import * as lifecycle from './capabilities/lifecycle.js';
import * as authorization from './capabilities/authorization.js';
import * as tools from './capabilities/tools.js';

// jco requires exports to be grouped by interface name
export { lifecycle, authorization, tools };