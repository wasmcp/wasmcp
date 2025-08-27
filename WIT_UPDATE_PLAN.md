# WIT Update Plan: Full MCP Protocol Support

## Executive Summary

Our current WIT files capture approximately 60% of the MCP protocol. This document outlines specific changes needed to achieve full protocol compliance while maintaining our clean architecture.

## Critical Missing Features

### 1. LLM Sampling (Server → Client)
**Current State**: Not implemented
**Impact**: Servers cannot request AI assistance from clients
**Priority**: HIGH - Core AI agent capability

### 2. Interactive Elicitation (Server → Client)  
**Current State**: Not implemented
**Impact**: Servers cannot request structured user input
**Priority**: HIGH - Essential for configuration flows

### 3. File System Roots (Client → Server)
**Current State**: Not implemented  
**Impact**: Servers cannot access client file systems
**Priority**: MEDIUM - Important for file-based tools

### 4. Autocompletion System
**Current State**: Not implemented
**Impact**: No support for argument completion
**Priority**: LOW - Nice to have for UX

## Type Safety Fixes

### 1. JSON Schema Representation
**Current**:
```wit
type json-schema = json-value;
type json-value = string;
```

**Proposed**:
```wit
record json-schema {
    type: option<string>,
    properties: option<list<tuple<string, json-value>>>,
    required: option<list<string>>,
    description: option<string>,
}

variant json-value {
    null,
    bool(bool),
    integer(s64),
    number(f64),
    string(string),
    array(list<json-value>),
    object(list<tuple<string, json-value>>),
}
```

### 2. Progress Notification Types
**Current**:
```wit
record progress-notification {
    progress-token: progress-token,
    progress: u32,
    total: option<u32>,
}
```

**Proposed**:
```wit
record progress-notification {
    progress-token: progress-token,
    progress: f64,
    total: option<f64>,
    message: option<string>,
}
```

## New Interfaces to Add

### 1. Sampling Interface (mcp.wit)
```wit
/// LLM sampling interface - allows servers to request AI assistance
interface sampling {
    use types.{
        mcp-error,
        sampling-message,
        model-preferences,
        meta-fields
    };

    record create-message-request {
        messages: list<sampling-message>,
        model-preferences: option<model-preferences>,
        system-prompt: option<string>,
        include-context: option<string>,
        temperature: option<f64>,
        max-tokens: s32,
        stop-sequences: option<list<string>>,
        metadata: option<meta-fields>,
    }

    record sampling-message {
        role: message-role,
        content: content,
    }

    enum message-role {
        user,
        assistant,
        system,
    }

    record model-preferences {
        hints: option<list<model-hint>>,
        cost-priority: option<f64>,
        speed-priority: option<f64>,
        intelligence-priority: option<f64>,
    }

    record model-hint {
        name: option<string>,
    }

    record create-message-result {
        role: message-role,
        content: content,
        model: string,
        stop-reason: option<string>,
    }

    /// Request LLM sampling from the client
    create-message: func(request: create-message-request) -> result<create-message-result, mcp-error>;
}
```

### 2. Elicitation Interface (mcp.wit)
```wit
/// Interactive elicitation - request structured input from users
interface elicitation {
    use types.{mcp-error, json-schema, json-value};

    record elicit-request {
        message: string,
        schema: json-schema,
    }

    record elicit-result {
        action: elicit-action,
        data: option<json-value>,
        message: option<string>,
    }

    enum elicit-action {
        accept,
        decline,
        cancel,
    }

    /// Request structured input from the user
    elicit: func(request: elicit-request) -> result<elicit-result, mcp-error>;
}
```

### 3. Roots Interface (mcp.wit)
```wit
/// File system roots - expose directories to servers
interface roots {
    use types.{mcp-error, meta-fields};

    record root {
        uri: string,
        name: option<string>,
    }

    record list-roots-request {
        // Currently empty, reserved for future use
    }

    record list-roots-result {
        roots: list<root>,
        meta: option<meta-fields>,
    }

    /// List available file system roots
    list-roots: func(request: list-roots-request) -> result<list-roots-result, mcp-error>;
}
```

### 4. Completion Interface (mcp.wit)
```wit
/// Argument completion support
interface completion {
    use types.{mcp-error};

    record complete-request {
        ref: completion-reference,
        argument-name: string,
        argument-value: string,
        context: option<completion-context>,
    }

    variant completion-reference {
        prompt(string),
        resource(string),
    }

    record completion-context {
        arguments: option<list<tuple<string, string>>>,
    }

    record complete-result {
        values: list<string>,
        total: option<s64>,
        has-more: option<bool>,
    }

    /// Get completion suggestions for arguments
    complete: func(request: complete-request) -> result<complete-result, mcp-error>;
}
```

## Updates to Existing Interfaces

### 1. Types Interface (types.wit)

**Add Resource Content Variants**:
```wit
// Update content variant to include all types
variant content {
    text(text-content),
    image(image-content),
    audio(audio-content),
    resource(resource-content),
    embedded-resource(embedded-resource),
}

record audio-content {
    data: list<u8>,  // base64 encoded
    mime-type: string,
}

record resource-content {
    uri: string,
    mime-type: option<string>,
}

record embedded-resource {
    uri: string,
    text: option<string>,
    blob: option<list<u8>>,
}
```

**Add Resource Templates**:
```wit
record resource-template {
    uri-template: string,  // RFC 6570 URI template
    name: string,
    description: option<string>,
    mime-type: option<string>,
}
```

### 2. Resources Interface (resources.wit)

**Add Templates and Subscriptions**:
```wit
interface resources {
    // ... existing functions ...

    /// List available resource templates
    list-templates: func(request: list-request) -> result<list-templates-result, mcp-error>;

    /// Subscribe to resource updates
    subscribe: func(uri: string) -> result<empty, mcp-error>;

    /// Unsubscribe from resource updates  
    unsubscribe: func(uri: string) -> result<empty, mcp-error>;
}

record list-templates-result {
    templates: list<resource-template>,
    cursor: option<cursor>,
    next-cursor: option<cursor>,
}
```

### 3. Notifications Interface (notifications.wit)

**Add Missing Notification Types**:
```wit
interface notifications {
    // ... existing ...

    /// Resource updated notification
    resource-updated: func(notification: resource-updated-notification) -> result<empty, mcp-error>;

    /// Roots list changed notification
    roots-list-changed: func(notification: roots-list-changed-notification) -> result<empty, mcp-error>;
}

record resource-updated-notification {
    uri: string,
}

record roots-list-changed-notification {
    // Empty for now
}
```

## Implementation Order

1. **Phase 1: Type Safety Fixes** (IMMEDIATE)
   - Fix progress types (u32 → f64)
   - Add proper JSON value/schema types
   - Update content variants

2. **Phase 2: Core Missing Features** (HIGH PRIORITY)
   - Add sampling interface
   - Add elicitation interface  
   - Add roots interface

3. **Phase 3: Enhanced Features** (MEDIUM PRIORITY)
   - Add completion interface
   - Add resource templates
   - Add resource subscriptions

4. **Phase 4: Full Notification Support** (LOW PRIORITY)
   - Add remaining notification types
   - Complete metadata support

## Breaking Changes

1. **progress field type change**: u32 → f64
2. **json-schema type change**: string → structured record
3. **New required interfaces**: Servers may need to handle new client capabilities

## Migration Strategy

1. Version the WIT package (e.g., `mcp@0.2.0`)
2. Provide adapter components for backwards compatibility
3. Document the new capabilities clearly
4. Update all examples and templates

## Validation

After implementation:
1. Validate against MCP JSON Schema
2. Test with reference MCP implementations
3. Ensure all examples still compile
4. Update documentation

## Notes

- Keep our clean interface separation
- Don't blindly copy the generated WIT structure
- Maintain WIT idioms (option, result, etc.)
- Document each new capability clearly
- Consider feature flags for optional protocol features