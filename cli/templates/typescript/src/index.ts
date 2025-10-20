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
  ClientContext,
  Tool,
} from './generated/interfaces/wasmcp-protocol-features.js';

// Tool input schemas
const ExampleToolSchema = z.object({
  input: z.string().describe('Example input parameter'),
});

type ExampleToolArgs = z.infer<typeof ExampleToolSchema>;

function listTools(
  _request: ListToolsRequest,
  _client: ClientContext
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

function callTool(
  request: CallToolRequest,
  _client: ClientContext
): CallToolResult | undefined {
  switch (request.name) {
    case 'example-tool':
      return handleExampleTool(request.arguments);
    default:
      return undefined; // We don't handle this tool
  }
}

function handleExampleTool(args?: string): CallToolResult {
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

export const toolsCapability = {
  listTools,
  callTool,
};
