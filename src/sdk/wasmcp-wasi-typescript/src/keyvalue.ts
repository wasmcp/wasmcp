/**
 * Key-value storage module for WASI (Spin-specific)
 * 
 * This module provides key-value storage functionality using Spin's WASI KV interfaces.
 * Only available when running in Spin runtime with KV stores configured.
 */

const encoder = new TextEncoder();
const decoder = new TextDecoder();

/**
 * Key-value store interface
 */
export interface Store {
  /**
   * Get a value by key
   * @param key - The key to retrieve
   * @returns The value as Uint8Array, or null if not found
   */
  get(key: string): Uint8Array | null;
  
  /**
   * Get a value as string
   * @param key - The key to retrieve
   * @returns The value as string, or null if not found
   */
  getString(key: string): string | null;
  
  /**
   * Get a value as JSON
   * @param key - The key to retrieve
   * @returns The parsed JSON value, or null if not found
   */
  getJson<T = any>(key: string): T | null;
  
  /**
   * Set a value
   * @param key - The key to set
   * @param value - The value (Uint8Array, string, or object)
   */
  set(key: string, value: Uint8Array | string | object): void;
  
  /**
   * Delete a key
   * @param key - The key to delete
   */
  delete(key: string): void;
  
  /**
   * Check if a key exists
   * @param key - The key to check
   * @returns true if the key exists
   */
  exists(key: string): boolean;
  
  /**
   * Get all keys in the store
   * @returns Array of all keys
   */
  getKeys(): string[];
}

// Cached module reference
let spinKvModule: any = null;
let loadAttempted = false;

/**
 * Try to load the Spin KV module
 * This is done lazily to avoid build-time errors
 */
async function tryLoadSpinKv(): Promise<any> {
  if (loadAttempted) {
    return spinKvModule;
  }
  
  loadAttempted = true;
  
  try {
    // Use eval to prevent static analysis by bundlers
    const importFunc = eval('(m) => import(m)');
    spinKvModule = await importFunc('fermyon:spin/key-value@2.0.0');
  } catch {
    // Not running in Spin or KV not available
    spinKvModule = null;
  }
  
  return spinKvModule;
}

/**
 * Open a key-value store
 * 
 * @param label - The store label (must be configured in spin.toml)
 * @returns A Store instance
 */
export async function open(label: string): Promise<Store> {
  const kv = await tryLoadSpinKv();
  if (!kv || !kv.Store) {
    throw new Error('Key-value storage is not available (not running in Spin or KV not configured)');
  }
  
  let store: any;
  
  try {
    store = kv.Store.open(label);
  } catch (error: any) {
    throw new Error(`Failed to open key-value store '${label}': ${error.message || error}`);
  }
  
  return {
    get(key: string): Uint8Array | null {
      const value = store.get(key);
      return value || null;
    },
    
    getString(key: string): string | null {
      const value = store.get(key);
      return value ? decoder.decode(value) : null;
    },
    
    getJson<T = any>(key: string): T | null {
      const value = store.get(key);
      if (!value) return null;
      const str = decoder.decode(value);
      try {
        return JSON.parse(str) as T;
      } catch {
        throw new Error(`Value for key '${key}' is not valid JSON`);
      }
    },
    
    set(key: string, value: Uint8Array | string | object): void {
      let bytes: Uint8Array;
      
      if (value instanceof Uint8Array) {
        bytes = value;
      } else if (typeof value === 'string') {
        bytes = encoder.encode(value);
      } else if (typeof value === 'object') {
        bytes = encoder.encode(JSON.stringify(value));
      } else {
        throw new Error('Value must be Uint8Array, string, or object');
      }
      
      store.set(key, bytes);
    },
    
    delete(key: string): void {
      store.delete(key);
    },
    
    exists(key: string): boolean {
      return store.exists(key);
    },
    
    getKeys(): string[] {
      return store.getKeys();
    }
  };
}

/**
 * Open the default key-value store
 * 
 * @returns A Store instance for the 'default' store
 */
export async function openDefault(): Promise<Store> {
  return open('default');
}

/**
 * Check if key-value storage is available
 * 
 * @returns true if running in Spin with KV support
 */
export async function isAvailable(): Promise<boolean> {
  const kv = await tryLoadSpinKv();
  return kv !== null && typeof kv.Store !== 'undefined';
}