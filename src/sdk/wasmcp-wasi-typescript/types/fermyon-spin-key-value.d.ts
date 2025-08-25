declare module 'fermyon:spin/key-value@2.0.0' {
  /**
   * The set of errors which may be raised by functions in this interface
   */
  export type Error =
    | ErrorStoreTableFull
    | ErrorNoSuchStore
    | ErrorAccessDenied
    | ErrorOther;
  
  /**
   * Too many stores have been opened simultaneously. Closing one or more
   * stores prior to retrying may address this.
   */
  export interface ErrorStoreTableFull {
    tag: 'store-table-full';
  }
  
  /**
   * The host does not recognize the store label requested.
   */
  export interface ErrorNoSuchStore {
    tag: 'no-such-store';
  }
  
  /**
   * The requesting component does not have access to the specified store
   * (which may or may not exist).
   */
  export interface ErrorAccessDenied {
    tag: 'access-denied';
  }
  
  /**
   * Some implementation-specific error has occurred (e.g. I/O)
   */
  export interface ErrorOther {
    tag: 'other';
    val: string;
  }

  export class Store {
    /**
     * Open the store with the specified label.
     *
     * `label` must refer to a store allowed in the spin.toml manifest.
     *
     * `error::no-such-store` will be raised if the `label` is not recognized.
     */
    static open(label: string): Store;
    
    /**
     * Get the value associated with the specified `key`
     *
     * Returns `ok(none)` if the key does not exist.
     */
    get(key: string): Uint8Array | undefined;
    
    /**
     * Set the `value` associated with the specified `key` overwriting any existing value.
     */
    set(key: string, value: Uint8Array): void;
    
    /**
     * Delete the tuple with the specified `key`
     *
     * No error is raised if a tuple did not previously exist for `key`.
     */
    delete(key: string): void;
    
    /**
     * Return whether a tuple exists for the specified `key`
     */
    exists(key: string): boolean;
    
    /**
     * Return a list of all the keys
     */
    getKeys(): Array<string>;
  }
}