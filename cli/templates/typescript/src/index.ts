/**
 * {{project_name}} - MCP Tools Handler
 *
 * This component exports MCP tools using the tools-capability interface.
 * Tools are automatically composed into the MCP server pipeline.
 */

import { z } from 'zod';
import type * as protocol from './generated/interfaces/wasmcp-mcp-protocol';
import type * as capability from './generated/interfaces/wasmcp-mcp-tools-capability';

// Define your tool input schema with Zod
const ExampleToolSchema = z.object({
  input: z.string().describe('Example input parameter'),
});

type ExampleToolInput = z.infer<typeof ExampleToolSchema>;

/**
 * List all tools provided by this handler
 */
export function listTools(
  _request: capability.ListToolsRequest,
  _client: protocol.ClientContext
): protocol.ListToolsResult {
  return {
    tools: [
      {
        name: 'example-tool',
        inputSchema: JSON.stringify(z.toJSONSchema(ExampleToolSchema)),
        options: {
          description: 'An example tool - replace with your own implementation',
          title: 'Example Tool',
        },
      },
    ],
    nextCursor: undefined,
    meta: undefined,
  };
}

/**
 * Execute a tool call
 *
 * Return the result if this handler implements the requested tool,
 * or undefined to delegate to the next handler in the pipeline.
 */
export function callTool(
  request: protocol.CallToolRequest,
  _client: protocol.ClientContext
): protocol.CallToolResult | undefined {
  switch (request.name) {
    case 'example-tool':
      return executeExampleTool(request);
    default:
      // Return undefined to delegate to next handler
      return undefined;
  }
}

/**
 * Execute the example tool
 */
function executeExampleTool(request: protocol.CallToolRequest): protocol.CallToolResult {
  try {
    // Parse and validate input
    if (!request.arguments) {
      return errorResult('Arguments are required');
    }

    const args = JSON.parse(request.arguments);
    const parsedArgs = ExampleToolSchema.parse(args);

    // TODO: Replace with your tool logic
    const result = `Received: ${parsedArgs.input}`;

    return textResult(result);
  } catch (error) {
    if (error instanceof z.ZodError) {
      return errorResult(`Invalid arguments: ${error.message}`);
    }
    return errorResult(error instanceof Error ? error.message : 'Unknown error');
  }
}

/**
 * Create a successful text result
 */
function textResult(text: string): protocol.CallToolResult {
  return {
    content: [
      {
        tag: 'text',
        val: {
          text: { tag: 'text', val: text },
          options: undefined,
        },
      },
    ],
    isError: false,
    structuredContent: undefined,
    meta: undefined,
  };
}

/**
 * Create an error result
 */
function errorResult(message: string): protocol.CallToolResult {
  return {
    content: [
      {
        tag: 'text',
        val: {
          text: { tag: 'text', val: message },
          options: undefined,
        },
      },
    ],
    isError: true,
    structuredContent: undefined,
    meta: undefined,
  };
}

export const toolsCapability = {
  listTools,
  callTool,
};
