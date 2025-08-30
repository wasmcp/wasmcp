# MCP WIT Definitions

The Wasm Interface Type ([WIT](https://component-model.bytecodealliance.org/design/wit.html)) package in this directory aim to capture a complete representation of the MCP specification. It currently reflects the 2025-06-18 version of the spec, with some additional elements from the latest draft.

The WIT package is published as Wasm at https://github.com/orgs/fastertools/packages/container/package/mcp. It can be fetched with `wkg wit fetch` when included as a dependency in a component's world:

```
/// world.wit
package weather-js:handler;

/// MCP tools for An MCP server written in JavaScript
world weather-js {
    export fastertools:mcp/tool-handler@0.1.9;
}
```
