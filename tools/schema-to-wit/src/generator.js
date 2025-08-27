/**
 * JSON Schema to WIT Generator
 */

const { TypeMapper } = require('./type-mapper');

class WitGenerator {
  constructor(options = {}) {
    this.options = options;
    this.mapper = new TypeMapper();
    this.interfaces = new Map();
    this.types = [];
    this.inlineTypes = new Map();
  }

  /**
   * Recursively process inline types in a schema
   */
  processInlineTypes(parentName, schema, definitions, types) {
    if (!schema.properties) return;
    
    for (const [propName, propSchema] of Object.entries(schema.properties)) {
      if (propSchema.type === 'object' && propSchema.properties) {
        // Generate a type for this inline object
        const inlineName = `${parentName}-${propName}`;
        const inlineType = this.mapper.generateRecord(inlineName, propSchema, definitions);
        if (inlineType && !this.inlineTypes.has(inlineName)) {
          this.inlineTypes.set(inlineName, inlineType);
          types.push(inlineType);
          // Recursively process nested objects
          this.processInlineTypes(inlineName, propSchema, definitions, types);
        }
      } else if (propSchema.anyOf || propSchema.oneOf) {
        // Generate a variant type for inline anyOf/oneOf
        const inlineName = `${parentName}-${propName}`;
        const inlineType = this.mapper.generateVariant(inlineName, propSchema, definitions);
        if (inlineType && !this.inlineTypes.has(inlineName)) {
          this.inlineTypes.set(inlineName, inlineType);
          types.push(inlineType);
        }
      } else if (propSchema.type === 'array' && propSchema.items) {
        // Check if array items need a type definition
        if (propSchema.items.anyOf || propSchema.items.oneOf) {
          const inlineName = `${parentName}-${propName}-item`;
          const inlineType = this.mapper.generateVariant(inlineName, propSchema.items, definitions);
          if (inlineType && !this.inlineTypes.has(inlineName)) {
            this.inlineTypes.set(inlineName, inlineType);
            types.push(inlineType);
          }
        } else if (propSchema.items.type === 'object' && propSchema.items.properties) {
          const inlineName = `${parentName}-${propName}-item`;
          const inlineType = this.mapper.generateRecord(inlineName, propSchema.items, definitions);
          if (inlineType && !this.inlineTypes.has(inlineName)) {
            this.inlineTypes.set(inlineName, inlineType);
            types.push(inlineType);
            // Recursively process nested objects in array items
            this.processInlineTypes(inlineName, propSchema.items, definitions, types);
          }
        }
      }
    }
  }

  /**
   * Generate WIT from JSON Schema
   */
  generate(schema) {
    const packageName = this.options.package || 'generated:schema@0.1.0';
    
    // Process definitions
    const types = this.generateTypes(schema.definitions || {});
    
    // Process method patterns (for MCP)
    const methods = this.extractMethods(schema);
    
    // Generate interface functions
    const interfaces = this.generateInterfaces(methods, schema.definitions || {});
    
    // Combine everything
    let wit = `// Generated from JSON Schema\n`;
    wit += `// Generator: @wasmcp/schema-to-wit v0.1.0\n\n`;
    wit += `package ${packageName};\n\n`;
    
    // Types interface
    if (types.length > 0) {
      wit += `interface types {\n`;
      
      // Add MCP error type (common to all interfaces)
      wit += `  /// MCP protocol error\n`;
      wit += `  record mcp-error {\n`;
      wit += `    code: s32,\n`;
      wit += `    message: string,\n`;
      wit += `    data: option<string>,\n`;
      wit += `  }\n\n`;
      
      wit += types.join('\n');
      wit += `}\n\n`;
    }
    
    // Method interfaces
    for (const [name, content] of interfaces) {
      wit += content + '\n\n';
    }
    
    return wit;
  }

  /**
   * Generate type definitions
   */
  generateTypes(definitions) {
    const types = [];
    
    // First pass: generate main types
    for (const [name, schema] of Object.entries(definitions)) {
      const type = this.generateType(name, schema, definitions);
      if (type) {
        types.push(type);
      }
      
      // Also generate types for inline properties
      this.processInlineTypes(name, schema, definitions, types);
    }
    
    // Second pass: add any additional inline types (avoiding duplicates)
    for (const [name, type] of this.inlineTypes) {
      const witName = this.mapper.toWitIdentifier(name);
      const isDuplicate = types.some(t => {
        return t.includes(`record ${witName} `) || 
               t.includes(`variant ${witName} `) ||
               t.includes(`enum ${witName} `);
      });
      if (!isDuplicate) {
        types.push(type);
      }
    }
    
    return types;
  }

  /**
   * Generate a single type definition
   */
  generateType(name, schema, definitions) {
    // Handle array type (union type)
    if (Array.isArray(schema.type)) {
      // Generate variant for union types
      const options = schema.type.map(t => ({ type: t }));
      return this.mapper.generateVariant(name, { oneOf: options }, definitions);
    }
    
    // Skip if it's a simple type that doesn't need definition
    if (schema.type === 'string' && !schema.enum && !schema.pattern) {
      return null;
    }
    if (schema.type === 'number' || schema.type === 'integer' || schema.type === 'boolean') {
      return null;
    }
    
    // Generate based on schema type
    if (schema.type === 'object' && schema.properties) {
      return this.mapper.generateRecord(name, schema, definitions);
    }
    
    if (schema.anyOf || schema.oneOf) {
      return this.mapper.generateVariant(name, schema, definitions);
    }
    
    if (schema.enum) {
      return this.mapper.generateEnum(name, schema.enum);
    }
    
    if (schema.type === 'array') {
      // Arrays don't need type definitions, they're inline
      return null;
    }
    
    // Default: generate as record if it has properties
    if (schema.properties) {
      return this.mapper.generateRecord(name, schema, definitions);
    }
    
    return null;
  }

  /**
   * Extract method patterns from schema (MCP specific)
   */
  extractMethods(schema) {
    const methods = new Map();
    
    // Recursively search for method patterns
    function findMethods(obj, path = []) {
      if (!obj || typeof obj !== 'object') return;
      
      // Look for properties that indicate this is a method
      if (obj.properties && obj.properties.method) {
        // Check if method has a const value (specific method name)
        if (obj.properties.method.const) {
          const methodName = obj.properties.method.const;
          methods.set(methodName, {
            schema: obj,
            path: path
          });
        }
      }
      
      // Continue searching
      for (const [key, value] of Object.entries(obj)) {
        if (key !== '$ref') { // Skip refs to avoid infinite loops
          findMethods(value, [...path, key]);
        }
      }
    }
    
    findMethods(schema);
    
    // Also look for method patterns in definitions
    if (schema.definitions) {
      for (const [defName, defSchema] of Object.entries(schema.definitions)) {
        // Check if this looks like a request/response pair
        if (defName.endsWith('Request') || defName.endsWith('Result')) {
          const baseName = defName.replace(/(Request|Result)$/, '');
          
          // Check for method property
          if (defSchema.properties && defSchema.properties.method) {
            if (defSchema.properties.method.const) {
              methods.set(defSchema.properties.method.const, {
                schema: defSchema,
                definitionName: defName
              });
            }
          }
        }
      }
    }
    
    return methods;
  }

  /**
   * Generate interface functions from methods
   */
  generateInterfaces(methods, definitions) {
    const interfaces = new Map();
    
    // Group methods by category
    const categories = new Map();
    
    for (const [methodName, methodInfo] of methods) {
      // Parse method name (e.g., "tools/list" -> category: "tools", action: "list")
      const parts = methodName.split('/');
      if (parts.length === 2) {
        const [category, action] = parts;
        
        if (!categories.has(category)) {
          categories.set(category, []);
        }
        
        categories.get(category).push({
          action,
          methodName,
          ...methodInfo
        });
      }
    }
    
    // Generate interface for each category
    for (const [category, categoryMethods] of categories) {
      let wit = `interface ${category} {\n`;
      wit += `  use types.{mcp-error`;
      
      // Import request/response types from types interface
      const typeImports = new Set();
      for (const method of categoryMethods) {
        const requestType = this.getRequestType(method);
        const responseType = this.getResponseType(method);
        if (requestType) typeImports.add(requestType);
        if (responseType) typeImports.add(responseType);
      }
      
      if (typeImports.size > 0) {
        wit += `, ${Array.from(typeImports).join(', ')}`;
      }
      wit += `};\n\n`;
      
      for (const method of categoryMethods) {
        wit += this.generateFunction(method, definitions);
      }
      
      wit += `}\n`;
      
      interfaces.set(category, wit);
    }
    
    // Also generate handler interfaces (for exports)
    for (const [category, categoryMethods] of categories) {
      let wit = `interface ${category}-handler {\n`;
      wit += `  use types.{mcp-error`;
      
      // Import request/response types from types interface
      const typeImports = new Set();
      for (const method of categoryMethods) {
        const requestType = this.getRequestType(method);
        const responseType = this.getResponseType(method);
        if (requestType) typeImports.add(requestType);
        if (responseType) typeImports.add(responseType);
      }
      
      if (typeImports.size > 0) {
        wit += `, ${Array.from(typeImports).join(', ')}`;
      }
      wit += `};\n\n`;
      
      for (const method of categoryMethods) {
        wit += this.generateHandlerFunction(method, definitions);
      }
      
      wit += `}\n`;
      
      interfaces.set(`${category}-handler`, wit);
    }
    
    return interfaces;
  }

  /**
   * Generate a function signature
   */
  generateFunction(method, definitions) {
    const funcName = this.mapper.toWitIdentifier(method.action);
    const requestType = this.getRequestType(method);
    const responseType = this.getResponseType(method);
    
    let wit = `  /// ${method.methodName}\n`;
    
    if (method.schema && method.schema.description) {
      wit += `  /// ${method.schema.description.replace(/\n/g, '\n  /// ')}\n`;
    }
    
    wit += `  ${funcName}: func(`;
    
    // Parameters
    if (requestType) {
      wit += `request: ${requestType}`;
    }
    
    wit += `) -> result<${responseType || 'string'}, mcp-error>;\n\n`;
    
    return wit;
  }

  /**
   * Generate a handler function signature
   */
  generateHandlerFunction(method, definitions) {
    const funcName = `handle-${this.mapper.toWitIdentifier(method.action)}`;
    const requestType = this.getRequestType(method);
    const responseType = this.getResponseType(method);
    
    let wit = `  /// Handle ${method.methodName}\n`;
    wit += `  ${funcName}: func(`;
    
    // Parameters
    if (requestType) {
      wit += `request: ${requestType}`;
    }
    
    wit += `) -> result<${responseType || 'string'}, mcp-error>;\n\n`;
    
    return wit;
  }

  /**
   * Get request type name for a method
   */
  getRequestType(method) {
    // Look for the actual type in schema definitions
    // Convert method name to PascalCase
    // For tools/call -> CallToolRequest (singular)
    // For tools/list -> ListToolsRequest (plural kept)
    const parts = method.methodName.split('/');
    if (parts.length === 2) {
      const [category, action] = parts;
      // Special handling for different actions
      let pascalName;
      if (action === 'call') {
        // "call" uses singular form: CallTool
        const singularCategory = category.replace(/s$/, ''); // Remove trailing 's'
        pascalName = 'Call' + singularCategory.charAt(0).toUpperCase() + singularCategory.slice(1);
      } else if (action === 'list') {
        // "list" uses plural form: ListTools
        pascalName = 'List' + category.charAt(0).toUpperCase() + category.slice(1);
      } else {
        // Default pattern: action + category
        pascalName = action.charAt(0).toUpperCase() + action.slice(1) + 
                    category.charAt(0).toUpperCase() + category.slice(1);
      }
      return this.mapper.toWitIdentifier(pascalName + 'Request');
    }
    
    // Fallback to generic naming
    const baseName = method.methodName.replace(/\//g, '-');
    return this.mapper.toWitIdentifier(`${baseName}-request`);
  }

  /**
   * Get response type name for a method
   */
  getResponseType(method) {
    // Look for the actual type in schema definitions
    // Same logic as request type but with Result suffix
    const parts = method.methodName.split('/');
    if (parts.length === 2) {
      const [category, action] = parts;
      // Special handling for different actions
      let pascalName;
      if (action === 'call') {
        // "call" uses singular form: CallTool
        const singularCategory = category.replace(/s$/, ''); // Remove trailing 's'
        pascalName = 'Call' + singularCategory.charAt(0).toUpperCase() + singularCategory.slice(1);
      } else if (action === 'list') {
        // "list" uses plural form: ListTools
        pascalName = 'List' + category.charAt(0).toUpperCase() + category.slice(1);
      } else {
        // Default pattern: action + category
        pascalName = action.charAt(0).toUpperCase() + action.slice(1) + 
                    category.charAt(0).toUpperCase() + category.slice(1);
      }
      return this.mapper.toWitIdentifier(pascalName + 'Result');
    }
    
    // Fallback to generic naming
    const baseName = method.methodName.replace(/\//g, '-');
    return this.mapper.toWitIdentifier(`${baseName}-result`);
  }
}

module.exports = { WitGenerator };