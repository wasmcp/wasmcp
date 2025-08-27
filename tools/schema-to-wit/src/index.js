/**
 * JSON Schema to WIT Converter
 * 
 * Main entry point for the library
 */

const { WitGenerator } = require('./generator');
const { TypeMapper } = require('./type-mapper');

module.exports = {
  WitGenerator,
  TypeMapper,
  
  /**
   * Convert JSON Schema to WIT
   * @param {object} schema - JSON Schema object
   * @param {object} options - Options for generation
   * @returns {string} Generated WIT
   */
  generateWit(schema, options = {}) {
    const generator = new WitGenerator(options);
    return generator.generate(schema);
  }
};