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
} from 'wasmcp:mcp-v20250618/mcp@{{wasmcp_version}}';
import type { RequestCtx } from 'wasmcp:mcp-v20250618/tools@{{wasmcp_version}}';

// Tool input schemas
const ExampleToolSchema = z.object({
  input: z.string().describe('Example input parameter'),
});

type ExampleToolArgs = z.infer<typeof ExampleToolSchema>;

function listTools(
  _ctx: RequestCtx,
  _request: ListToolsRequest
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
  _ctx: RequestCtx,
  request: CallToolRequest
): Promise<CallToolResult | undefined> {
  switch (request.name) {
    case 'example-tool':
      return await handleExampleTool(request.arguments);
    default:
      return undefined; // We don't handle this tool
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
