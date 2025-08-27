#!/usr/bin/env node

/**
 * Proof-of-concept: Generate WIT interfaces from MCP JSON Schema
 * 
 * This demonstrates how we could automatically generate WIT from the MCP spec,
 * ensuring our interfaces stay in sync with the protocol specification.
 */

const fs = require('fs');
const path = require('path');

// Map JSON Schema types to WIT types
function jsonSchemaTypeToWit(schema, typeName = '') {
  if (!schema) return 'string'; // fallback
  
  // Handle references
  if (schema.$ref) {
    const refName = schema.$ref.split('/').pop();
    return kebabCase(refName);
  }
  
  // Handle basic types
  switch (schema.type) {
    case 'string':
      if (schema.format === 'uri') return 'string'; // WIT doesn't have URI type
      if (schema.format === 'byte') return 'list<u8>'; // base64 -> bytes
      return 'string';
    
    case 'number':
    case 'integer':
      return 'f64'; // or 's32/s64' for integers
    
    case 'boolean':
      return 'bool';
    
    case 'array':
      const itemType = jsonSchemaTypeToWit(schema.items);
      return `list<${itemType}>`;
    
    case 'object':
      // If it has specific properties, it's a record
      if (schema.properties) {
        return typeName || 'record';
      }
      // Otherwise it's a generic map
      return 'string'; // WIT doesn't have generic objects, use JSON string
    
    case undefined:
      // Handle anyOf/oneOf as variants
      if (schema.anyOf || schema.oneOf) {
        return `variant-${typeName}`;
      }
      return 'string';
    
    default:
      return 'string';
  }
}

// Convert property to WIT record field
function propertyToWitField(name, schema, required = []) {
  const witType = jsonSchemaTypeToWit(schema, name);
  const isOptional = !required.includes(name);
  const fieldName = kebabCase(name);
  
  if (isOptional) {
    return `  ${fieldName}: option<${witType}>`;
  }
  return `  ${fieldName}: ${witType}`;
}

// Generate WIT record from JSON Schema object
function generateWitRecord(name, schema) {
  const witName = kebabCase(name);
  const required = schema.required || [];
  
  let wit = `record ${witName} {\n`;
  
  if (schema.properties) {
    for (const [propName, propSchema] of Object.entries(schema.properties)) {
      wit += propertyToWitField(propName, propSchema, required) + ',\n';
    }
  }
  
  wit += '}\n';
  return wit;
}

// Generate WIT variant from anyOf/oneOf
function generateWitVariant(name, schema) {
  const witName = kebabCase(name);
  let wit = `variant ${witName} {\n`;
  
  const options = schema.anyOf || schema.oneOf || [];
  for (const option of options) {
    if (option.const) {
      wit += `  ${kebabCase(option.const)},\n`;
    } else if (option.$ref) {
      const refName = option.$ref.split('/').pop();
      wit += `  ${kebabCase(refName)}(${kebabCase(refName)}),\n`;
    }
  }
  
  wit += '}\n';
  return wit;
}

// Convert camelCase to kebab-case
function kebabCase(str) {
  return str
    .replace(/([a-z])([A-Z])/g, '$1-$2')
    .replace(/[\s_]+/g, '-')
    .toLowerCase();
}

// Generate WIT interface for MCP methods
function generateMcpInterface(schema) {
  let wit = 'interface mcp-protocol {\n';
  
  // Extract method patterns from the schema
  const methods = new Set();
  
  // Look for method constants in the schema
  function findMethods(obj, path = '') {
    if (typeof obj !== 'object' || !obj) return;
    
    if (obj.const && typeof obj.const === 'string' && obj.const.includes('/')) {
      methods.add(obj.const);
    }
    
    for (const value of Object.values(obj)) {
      findMethods(value);
    }
  }
  
  findMethods(schema);
  
  // Generate functions for each method
  for (const method of methods) {
    const [category, action] = method.split('/');
    const funcName = `${action}-${category}`;
    
    wit += `\n  // ${method}\n`;
    wit += `  ${funcName}: func(request: ${kebabCase(method.replace('/', '-'))}-request) -> result<${kebabCase(method.replace('/', '-'))}-response, mcp-error>;\n`;
  }
  
  wit += '}\n';
  return wit;
}

// Main generator
function generateWitFromSchema(schemaPath) {
  const schema = JSON.parse(fs.readFileSync(schemaPath, 'utf8'));
  
  let wit = `// Generated from MCP JSON Schema\n`;
  wit += `// Source: ${path.basename(schemaPath)}\n\n`;
  wit += `package fastertools:mcp@0.1.1;\n\n`;
  
  // Generate types from definitions
  wit += `interface types {\n`;
  
  if (schema.definitions) {
    for (const [name, definition] of Object.entries(schema.definitions)) {
      wit += `  // ${definition.description || name}\n`;
      
      if (definition.type === 'object' && definition.properties) {
        wit += generateWitRecord(name, definition);
      } else if (definition.anyOf || definition.oneOf) {
        wit += generateWitVariant(name, definition);
      }
      
      wit += '\n';
    }
  }
  
  wit += '}\n\n';
  
  // Generate interface
  wit += generateMcpInterface(schema);
  
  return wit;
}

// Run if called directly
if (require.main === module) {
  const schemaPath = process.argv[2] || './tmp/modelcontextprotocol/schema/2025-06-18/schema.json';
  
  try {
    const wit = generateWitFromSchema(schemaPath);
    console.log(wit);
    
    // Optionally write to file
    if (process.argv[3]) {
      fs.writeFileSync(process.argv[3], wit);
      console.log(`\nWIT generated and saved to ${process.argv[3]}`);
    }
  } catch (error) {
    console.error('Error generating WIT:', error);
    process.exit(1);
  }
}

module.exports = { generateWitFromSchema };