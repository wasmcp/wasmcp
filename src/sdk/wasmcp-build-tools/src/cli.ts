#!/usr/bin/env node

import yargs from 'yargs';
import { hideBin } from 'yargs/helpers';
import { componentize } from './index.js';
import { readFile, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import path from 'node:path';

interface CliArgs {
  input: string;
  output: string;
  world?: string;
  'wit-path'?: string;
  'enable-kv'?: boolean;
  debug?: boolean;
  aot?: boolean;
}

async function main() {
  const args = yargs(hideBin(process.argv))
    .option('input', {
      alias: 'i',
      describe: 'Path to the input JavaScript file',
      demandOption: true,
      type: 'string'
    })
    .option('output', {
      alias: 'o',
      describe: 'Path to the output WASM file',
      default: 'component.wasm',
      type: 'string'
    })
    .option('world', {
      alias: 'w',
      describe: 'World to use (mcp-handler by default)',
      default: 'mcp-handler',
      type: 'string'
    })
    .option('wit-path', {
      describe: 'Path to WIT files',
      type: 'string'
    })
    .option('enable-kv', {
      describe: 'Enable key-value storage (Spin only)',
      type: 'boolean',
      default: false
    })
    .option('debug', {
      alias: 'd',
      describe: 'Enable debugging',
      type: 'boolean'
    })
    .option('aot', {
      describe: 'Enable Ahead of Time compilation',
      type: 'boolean'
    })
    .argv as CliArgs;

  try {
    const { input, output, world, debug, aot } = args;
    const enableKv = args['enable-kv'];
    let witPath = args['wit-path'];
    
    // Auto-detect WIT path if not provided
    if (!witPath) {
      // Look for wit directory in common locations
      const possiblePaths = [
        path.join(process.cwd(), 'wit'),
        path.join(process.cwd(), 'handler', 'wit'),
        path.join(path.dirname(input), '..', 'wit'),
        path.join(path.dirname(input), 'wit')
      ];
      
      for (const p of possiblePaths) {
        if (existsSync(p)) {
          witPath = p;
          console.log(`Using WIT directory: ${witPath}`);
          break;
        }
      }
      
      if (!witPath) {
        throw new Error('Could not find WIT directory. Please specify --wit-path');
      }
    }
    
    // Determine which world to use
    const targetWorld = world || 'mcp-handler';
    
    console.log(`Componentizing ${input}...`);
    console.log(`Target world: ${targetWorld}`);
    if (enableKv) {
      console.log('Key-value storage enabled (Spin only)');
    }
    
    const component = await componentize({
      sourcePath: input,
      witPath,
      worldName: targetWorld,
      enableDebug: debug,
      enableAot: aot,
      disableFeatures: []  // We want all features for MCP
    });
    
    await writeFile(output, component);
    console.log(`Component written to ${output}`);
    
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

main();