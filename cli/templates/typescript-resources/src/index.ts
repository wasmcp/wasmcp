/**
 * {{project_name}} Resources Capability
 *
 * A resources capability that provides simple text resources.
 */

import type {
  ListResourcesRequest,
  ListResourcesResult,
  ReadResourceRequest,
  ReadResourceResult,
  ListResourceTemplatesRequest,
  ListResourceTemplatesResult,
  McpResource,
} from './generated/interfaces/wasmcp-protocol-mcp.js';
import type { Context } from './generated/interfaces/wasmcp-protocol-server-messages.js';
import type { OutputStream } from './generated/interfaces/wasi-io-streams.js';

function listResources(
  _ctx: Context,
  _request: ListResourcesRequest,
  _clientStream: OutputStream | null
): ListResourcesResult {
  const resources: McpResource[] = [
    {
      uri: 'text://greeting',
      name: 'Greeting',
      options: {
        mimeType: 'text/plain',
        description: 'A friendly greeting message',
      },
    },
    {
      uri: 'text://info',
      name: 'Info',
      options: {
        mimeType: 'text/plain',
        description: 'Information about this resource provider',
      },
    },
  ];

  return { resources };
}

async function readResource(
  _ctx: Context,
  request: ReadResourceRequest,
  _clientStream: OutputStream | null
): Promise<ReadResourceResult | null> {
  switch (request.uri) {
    case 'text://greeting':
      return textResource('Hello from wasmcp resources!');
    case 'text://info':
      return textResource(
        'This is a simple resources capability component. ' +
        'It provides static text content via custom URIs.'
      );
    default:
      return null; // We don't handle this URI
  }
}

async function listResourceTemplates(
  _ctx: Context,
  _request: ListResourceTemplatesRequest,
  _clientStream: OutputStream | null
): Promise<ListResourceTemplatesResult> {
  // No templates for static resources
  return { resourceTemplates: [] };
}

function textResource(text: string): ReadResourceResult {
  return {
    contents: [{
      tag: 'text',
      val: {
        uri: '', // URI is provided in request
        text: {
          tag: 'text',
          val: text,
        },
        options: {
          mimeType: 'text/plain',
        },
      },
    }],
  };
}

export const resources = {
  listResources,
  readResource,
  listResourceTemplates,
};
