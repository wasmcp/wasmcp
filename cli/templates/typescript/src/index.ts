/**
 * {{project_name}} Tools Capability
 *
 * A tools capability that provides example operations.
 */

import * as z from 'zod';
import type {
  ListToolsRequest,
  ListToolsResult,
  CallToolRequest,
  CallToolResult,
  Tool,
} from './generated/interfaces/wasmcp-protocol-mcp.js';
import type { Context } from './generated/interfaces/wasmcp-protocol-server-messages.js';
import type { OutputStream } from './generated/interfaces/wasi-io-streams.js';

// Tool input schemas
const ExampleToolSchema = z.object({
  input: z.string().describe('Example input parameter'),
});

type ExampleToolArgs = z.infer<typeof ExampleToolSchema>;

function listTools(
  _ctx: Context,
  _request: ListToolsRequest,
  _clientStream: OutputStream | null
): ListToolsResult {
  const tools: Tool[] = [
    {
      name: 'example-tool',
      inputSchema: JSON.stringify(z.toJSONSchema(ExampleToolSchema)),
      options: {
        title: 'Example Tool',
        description: 'An example tool - replace with your own implementation',
      },
    },
  ];

  return { tools };
}

async function callTool(
  _ctx: Context,
  request: CallToolRequest,
  _clientStream: OutputStream | null
): Promise<CallToolResult | null> {
  switch (request.name) {
    case 'example-tool':
      return await handleExampleTool(request.arguments);
    default:
      return null; // We don't handle this tool
  }
}

async function handleExampleTool(args?: string): Promise<CallToolResult> {
  try {
    if (!args) {
      return errorResult('Arguments are required');
    }

    const parsed: ExampleToolArgs = ExampleToolSchema.parse(JSON.parse(args));

    // TODO: Replace with your tool logic
    const result = `Received: ${parsed.input}`;

    return textResult(result);
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResult(`Invalid arguments: ${error.message}`);
    }
    return errorResult(
      `Error processing request: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

function textResult(text: string): CallToolResult {
  return {
    content: [{
      tag: 'text',
      val: {
        text: { tag: 'text', val: text },
      },
    }],
    isError: false,
  };
}

function errorResult(message: string): CallToolResult {
  return {
    content: [{
      tag: 'text',
      val: {
        text: { tag: 'text', val: message },
      },
    }],
    isError: true,
  };
}

export const tools = {
  listTools,
  callTool,
};
