import { componentize as componentizeJs } from '@bytecodealliance/componentize-js';
import { readFile } from 'node:fs/promises';
import path from 'node:path';

export interface ComponentizeOptions {
  sourcePath: string;
  witPath: string;
  worldName?: string;
  enableDebug?: boolean;
  enableAot?: boolean;
  disableFeatures?: string[];
}

/**
 * Componentize a JavaScript module to a WebAssembly component
 */
export async function componentize(options: ComponentizeOptions): Promise<Uint8Array> {
  const {
    sourcePath,
    witPath,
    worldName = 'mcp-handler',
    enableDebug = false,
    enableAot = false,
    disableFeatures = []
  } = options;
  
  // Read the source file
  const source = await readFile(sourcePath, 'utf-8');
  
  // Prepare componentize options
  const componentizeOptions = {
    sourcePath,
    witPath,
    worldName,
    enableAot,
    disableFeatures: disableFeatures as any,
    runtimeArgs: enableDebug ? '--enable-script-debugging' : undefined
  };
  
  console.log('Running componentize with options:', {
    ...componentizeOptions,
    source: '[omitted]'
  });
  
  // Run componentize
  const { component } = await componentizeJs(componentizeOptions);
  
  return component;
}

/**
 * Get the version of componentize-js
 */
export function getVersion(): string {
  // Import version from componentize-js package.json
  return '0.18.1'; // TODO: Read from actual package
}