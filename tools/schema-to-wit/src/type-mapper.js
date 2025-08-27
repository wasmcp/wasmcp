/**
 * Type mapping from JSON Schema to WIT
 */

class TypeMapper {
  constructor() {
    // Track generated types to avoid duplicates
    this.generatedTypes = new Map();
    // Track types being processed to handle circular references
    this.processingTypes = new Set();
  }

  /**
   * Map a JSON Schema type to WIT type
   */
  mapType(schema, typeName = null, definitions = {}) {
    if (!schema) return 'string';

    // Handle references
    if (schema.$ref) {
      const refName = schema.$ref.split('/').pop();
      return this.toWitIdentifier(refName);
    }

    // Handle combined schemas
    if (schema.anyOf || schema.oneOf) {
      return this.mapVariant(schema, typeName, definitions);
    }

    if (schema.allOf) {
      // For allOf, we need to merge schemas - simplified for now
      return this.mapType(schema.allOf[0], typeName, definitions);
    }

    // Handle basic types
    switch (schema.type) {
      case 'string':
        return this.mapString(schema);
      
      case 'number':
        return this.mapNumber(schema);
      
      case 'integer':
        return this.mapInteger(schema);
      
      case 'boolean':
        return 'bool';
      
      case 'array':
        return this.mapArray(schema, typeName, definitions);
      
      case 'object':
        return this.mapObject(schema, typeName, definitions);
      
      case 'null':
        return 'option<string>'; // WIT doesn't have null, use option
      
      default:
        // If no type specified, check for other schema keywords
        if (schema.properties) {
          return this.mapObject(schema, typeName, definitions);
        }
        if (schema.enum) {
          return this.mapEnum(schema, typeName);
        }
        return 'string'; // fallback
    }
  }

  mapString(schema) {
    // Handle string formats
    if (schema.format === 'byte') {
      return 'list<u8>'; // base64 encoded bytes
    }
    if (schema.format === 'date-time' || schema.format === 'date') {
      return 'string'; // WIT doesn't have date types
    }
    if (schema.enum) {
      return this.mapEnum(schema);
    }
    return 'string';
  }

  mapNumber(schema) {
    // Check constraints to determine precision
    if (schema.minimum !== undefined || schema.maximum !== undefined) {
      // Has constraints, use f64 for precision
      return 'f64';
    }
    return 'f64'; // Default to f64
  }

  mapInteger(schema) {
    // Determine signed/unsigned and size based on constraints
    const min = schema.minimum;
    const max = schema.maximum;
    
    if (min !== undefined && min >= 0) {
      // Unsigned
      if (max !== undefined && max <= 255) return 'u8';
      if (max !== undefined && max <= 65535) return 'u16';
      if (max !== undefined && max <= 4294967295) return 'u32';
      return 'u64';
    } else {
      // Signed
      if (min !== undefined && min >= -128 && max !== undefined && max <= 127) return 's8';
      if (min !== undefined && min >= -32768 && max !== undefined && max <= 32767) return 's16';
      if (min !== undefined && min >= -2147483648 && max !== undefined && max <= 2147483647) return 's32';
      return 's64';
    }
  }

  mapArray(schema, typeName, definitions) {
    if (!schema.items) {
      return 'list<string>'; // Default item type
    }
    
    const itemType = this.mapType(schema.items, typeName ? `${typeName}-item` : null, definitions);
    return `list<${itemType}>`;
  }

  mapObject(schema, typeName, definitions) {
    // If it has specific properties, it's a record
    if (schema.properties && Object.keys(schema.properties).length > 0) {
      if (!typeName) {
        // Need to generate inline type or use string
        return 'string'; // For now, fallback to JSON string
      }
      // This would be a named record type
      return this.toWitIdentifier(typeName);
    }
    
    // If additionalProperties is specified, it's like a map
    if (schema.additionalProperties) {
      if (typeof schema.additionalProperties === 'object') {
        const valueType = this.mapType(schema.additionalProperties, null, definitions);
        // WIT doesn't have built-in map type, could use list<tuple<string, T>>
        return `list<tuple<string, ${valueType}>>`;
      }
    }
    
    // Generic object, use JSON string
    return 'string';
  }

  mapVariant(schema, typeName, definitions) {
    const options = schema.anyOf || schema.oneOf || [];
    
    // Simple case: all options have const values (enum-like)
    const allConst = options.every(opt => opt.const !== undefined);
    if (allConst) {
      return this.mapEnum({ enum: options.map(opt => opt.const) }, typeName);
    }
    
    // Complex variant - would need a named type
    if (!typeName) {
      return 'string'; // Fallback for inline variants
    }
    
    return this.toWitIdentifier(typeName);
  }

  mapEnum(schema, typeName) {
    // For simple enums, we might want to generate an enum type
    // For now, treat as string
    if (!typeName) {
      return 'string';
    }
    return this.toWitIdentifier(typeName);
  }

  /**
   * Generate a record type from an object schema
   */
  generateRecord(name, schema, definitions = {}) {
    const witName = this.toWitIdentifier(name);
    
    // Prevent infinite recursion
    if (this.processingTypes.has(name)) {
      return `// Circular reference: ${witName}`;
    }
    
    this.processingTypes.add(name);
    
    let wit = '';
    
    // Add description as comment
    if (schema.description) {
      wit += `  /// ${schema.description.replace(/\n/g, '\n  /// ')}\n`;
    }
    
    wit += `  record ${witName} {\n`;
    
    const required = schema.required || [];
    
    if (schema.properties) {
      for (const [propName, propSchema] of Object.entries(schema.properties)) {
        const fieldName = this.toWitIdentifier(propName);
        const fieldType = this.mapType(propSchema, `${name}-${propName}`, definitions);
        const isOptional = !required.includes(propName);
        
        // Add field description
        if (propSchema.description) {
          wit += `    /// ${propSchema.description.replace(/\n/g, '\n    /// ')}\n`;
        }
        
        if (isOptional) {
          wit += `    ${fieldName}: option<${fieldType}>,\n`;
        } else {
          wit += `    ${fieldName}: ${fieldType},\n`;
        }
      }
    }
    
    wit += `  }\n`;
    
    this.processingTypes.delete(name);
    
    return wit;
  }

  /**
   * Generate a variant type from anyOf/oneOf schema
   */
  generateVariant(name, schema, definitions = {}) {
    const witName = this.toWitIdentifier(name);
    const options = schema.anyOf || schema.oneOf || [];
    
    let wit = '';
    
    if (schema.description) {
      wit += `  /// ${schema.description.replace(/\n/g, '\n  /// ')}\n`;
    }
    
    wit += `  variant ${witName} {\n`;
    
    for (const option of options) {
      if (option.const !== undefined) {
        // Simple enum value
        wit += `    ${this.toWitIdentifier(String(option.const))},\n`;
      } else if (option.$ref) {
        // Reference to another type
        const refName = option.$ref.split('/').pop();
        const witRef = this.toWitIdentifier(refName);
        wit += `    ${witRef}(${witRef}),\n`;
      } else if (option.type) {
        // Inline type - generate a name for it
        const optionType = this.mapType(option, null, definitions);
        const caseName = this.toWitIdentifier(option.type);
        wit += `    ${caseName}(${optionType}),\n`;
      }
    }
    
    wit += `  }\n`;
    
    return wit;
  }

  /**
   * Generate an enum type
   */
  generateEnum(name, values) {
    const witName = this.toWitIdentifier(name);
    
    let wit = `  enum ${witName} {\n`;
    
    for (const value of values) {
      wit += `    ${this.toWitIdentifier(String(value))},\n`;
    }
    
    wit += `  }\n`;
    
    return wit;
  }

  /**
   * Convert string to valid WIT identifier (kebab-case)
   */
  toWitIdentifier(str) {
    if (!str) return 'unknown';
    
    // WIT reserved keywords that need escaping
    const reserved = new Set([
      'type', 'record', 'variant', 'enum', 'flags', 'resource',
      'func', 'use', 'interface', 'world', 'export', 'import',
      'package', 'include', 'as', 'from', 'static', 'constructor',
      'method', 'u8', 'u16', 'u32', 'u64', 's8', 's16', 's32', 's64',
      'f32', 'f64', 'bool', 'string', 'char', 'list', 'option',
      'result', 'tuple', 'future', 'stream'
    ]);
    
    let identifier = str
      // Handle URIs and special formats
      .replace(/^https?:\/\//, '')
      .replace(/[\/\.:]/g, '-')
      // Convert to kebab-case
      .replace(/([a-z])([A-Z])/g, '$1-$2')
      .replace(/[\s_]+/g, '-')
      .replace(/[^a-zA-Z0-9-]/g, '')
      .toLowerCase()
      // Clean up multiple dashes
      .replace(/-+/g, '-')
      .replace(/^-|-$/g, '')
      // Ensure valid identifier (can't start with number)
      .replace(/^(\d)/, 'n$1') || 'unknown';
    
    // Escape reserved keywords by prefixing with underscore
    if (reserved.has(identifier)) {
      identifier = `${identifier}-field`;
    }
    
    return identifier;
  }
}

module.exports = { TypeMapper };