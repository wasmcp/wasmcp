/**
 * Configuration module for WASI
 * 
 * This module provides access to configuration values.
 * Note: Implementation depends on WASI runtime support.
 */

/**
 * Get a configuration value by key
 * 
 * @param key - The configuration key
 * @returns The configuration value, or undefined if not found
 */
export function get(key: string): string | undefined {
  // For now, fallback to environment variables
  // In the future, this could use wasi:config if supported
  if (typeof process !== 'undefined' && process.env) {
    return process.env[key];
  }
  
  // In WASM context, we might not have process.env
  // ComponentizeJS may provide a different mechanism
  return undefined;
}

/**
 * Get all configuration values
 * 
 * @returns Object containing all configuration key-value pairs
 */
export function getAll(): Record<string, string> {
  // For now, return environment variables
  if (typeof process !== 'undefined' && process.env) {
    const result: Record<string, string> = {};
    for (const [key, value] of Object.entries(process.env)) {
      if (value !== undefined) {
        result[key] = value;
      }
    }
    return result;
  }
  
  return {};
}

/**
 * Check if a configuration key exists
 * 
 * @param key - The configuration key
 * @returns true if the key exists
 */
export function has(key: string): boolean {
  return get(key) !== undefined;
}