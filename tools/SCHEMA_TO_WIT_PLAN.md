# JSON Schema to WIT Generator: Project Plan

## Executive Summary

We've discovered a significant gap in the WebAssembly tooling ecosystem: there is no existing tool to convert JSON Schema (or OpenAPI/Protobuf) specifications to WIT (WebAssembly Interface Types). This presents an opportunity to create the first such tool, which would benefit not only our MCP implementation but the entire WebAssembly community.

## Problem Statement

1. **MCP Specification**: The Model Context Protocol has a complete JSON Schema specification (`schema.json`) that defines all types, methods, and capabilities
2. **WIT Interfaces**: We need WIT interfaces to implement MCP in WebAssembly components
3. **Manual Translation**: Currently, WIT interfaces must be manually written from specs, leading to:
   - Potential inconsistencies
   - Maintenance burden when specs change
   - Duplicate effort across projects

## Discovery Findings

### What We Searched
- "JSON Schema to WIT converter" - **No tools found**
- "OpenAPI to WIT generator" - **No tools found**
- "Protobuf to WIT converter" - **No tools found**

### What the Community Says
- WebAssembly Component Model community explicitly wants "an automated tool that translates interface definitions to WIT"
- Multiple IDL formats (OpenAPI, Protobuf, JSON Schema) all need similar converters
- This is recognized as a gap but no one has built it yet

## Technical Approach

### Input: JSON Schema
```json
{
  "definitions": {
    "Tool": {
      "type": "object",
      "properties": {
        "name": { "type": "string" },
        "description": { "type": "string" },
        "inputSchema": { "type": "object" }
      },
      "required": ["name"]
    }
  }
}
```

### Output: WIT Interface
```wit
record tool {
  name: string,
  description: option<string>,
  input-schema: string, // JSON schema as string
}
```

### Type Mapping Strategy

| JSON Schema Type | WIT Type | Notes |
|-----------------|----------|-------|
| `string` | `string` | Direct mapping |
| `number` | `f64` | Or `f32` based on constraints |
| `integer` | `s32`/`s64` | Based on min/max |
| `boolean` | `bool` | Direct mapping |
| `array` | `list<T>` | Recursive type resolution |
| `object` (with properties) | `record` | Named fields |
| `object` (arbitrary) | `string` | JSON string fallback |
| `null` | `option<T>` | Nullable types |
| `anyOf`/`oneOf` | `variant` | Union types |
| `$ref` | Reference to type | Cross-reference resolution |
| `format: "byte"` | `list<u8>` | Base64 → bytes |
| `format: "uri"` | `string` | No special URI type in WIT |

## Implementation Plan

### Phase 1: Core Generator (Week 1)
- [x] Proof of concept (basic type conversion)
- [ ] Complete type mapping system
- [ ] Handle nested objects and arrays
- [ ] Reference resolution (`$ref`)
- [ ] Generate valid WIT syntax

### Phase 2: MCP-Specific Features (Week 2)
- [ ] Parse MCP method definitions
- [ ] Generate interface functions from methods
- [ ] Handle request/response patterns
- [ ] Generate separate WIT files per interface (types, tools, resources, etc.)

### Phase 3: Production Ready (Week 3)
- [ ] CLI tool with options
- [ ] NPM package
- [ ] Validation and error reporting
- [ ] Documentation
- [ ] Test suite with MCP schema

### Phase 4: Community Release
- [ ] Open source on GitHub
- [ ] Support for OpenAPI (stretch goal)
- [ ] Support for Protobuf (stretch goal)
- [ ] Community feedback incorporation

## Benefits

### For wasmcp Project
1. **Automatic WIT Generation**: Keep WIT interfaces in sync with MCP spec
2. **Version Management**: Easy updates when MCP spec changes
3. **Correctness**: No manual translation errors
4. **Documentation**: WIT comments from schema descriptions

### For WebAssembly Community
1. **First Tool of Its Kind**: Fills a recognized gap
2. **Enables Integration**: JSON APIs → WebAssembly components
3. **Broader Application**: Any JSON Schema-based API can target WebAssembly
4. **Foundation for More Tools**: Pattern for OpenAPI/Protobuf converters

## Success Metrics

1. Successfully generates WIT from complete MCP schema
2. Generated WIT compiles with `wasm-tools`
3. Components built from generated WIT pass MCP protocol tests
4. Community adoption (GitHub stars, forks, contributions)

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| WIT doesn't support all JSON Schema features | Use string fallback for complex types, document limitations |
| Schema changes break generator | Version the generator with schema versions |
| Complex schemas produce invalid WIT | Extensive validation and testing |

## Code Architecture

```
schema-to-wit/
├── src/
│   ├── parser.js        # JSON Schema parsing
│   ├── mapper.js        # Type mapping logic
│   ├── generator.js     # WIT generation
│   ├── validator.js     # Output validation
│   └── cli.js          # CLI interface
├── templates/
│   └── wit.hbs         # WIT templates
├── test/
│   ├── fixtures/       # Test schemas
│   └── specs/          # Test cases
└── package.json
```

## Next Steps

1. **Immediate**: Enhance proof-of-concept to handle full MCP schema
2. **This Week**: Build core generator with complete type mapping
3. **Next Week**: Test with real MCP implementation
4. **Two Weeks**: Package and release as open source tool

## Conclusion

This is a legitimate opportunity to create a foundational tool for the WebAssembly ecosystem. We have:
- **Clear need**: No existing tools
- **Concrete use case**: MCP implementation
- **Technical feasibility**: Proof of concept works
- **Community value**: Recognized gap by WA Component Model community

Let's build the first JSON Schema → WIT generator!