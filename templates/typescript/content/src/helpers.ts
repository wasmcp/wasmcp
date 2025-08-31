/**
 * Helper functions for building MCP tools with strict TypeScript types
 */

import type { ToolDefinition } from './types.js';
import type {
  ToolsCapabilities,
  ListToolsRequest,
  ListToolsResponse,
  CallToolRequest,
  ToolResult,
  Tool,
  ContentBlock,
  JsonValue,
} from './mcp-types.js';

export interface CreateHandlerOptions {
  tools: readonly ToolDefinition[];
}

/**
 * Create a tool definition with type safety
 */
export function createTool<TArgs extends Record<string, unknown>>(
  definition: ToolDefinition<TArgs>,
): ToolDefinition<TArgs> {
  return definition;
}

/**
 * Create a success result
 */
function createSuccessResult(text: string): ToolResult {
  const contentBlock: ContentBlock = {
    tag: 'text',
    val: {
      text,
      annotations: undefined,
      meta: undefined,
    },
  };

  return {
    content: [contentBlock],
    structuredContent: undefined,
    isError: false,
    meta: undefined,
  };
}

/**
 * Create an error result
 */
function createErrorResult(text: string): ToolResult {
  const contentBlock: ContentBlock = {
    tag: 'text',
    val: {
      text,
      annotations: undefined,
      meta: undefined,
    },
  };

  return {
    content: [contentBlock],
    structuredContent: undefined,
    isError: true,
    meta: undefined,
  };
}

/**
 * Parse and validate arguments from the request
 */
function parseArguments(argumentsValue: JsonValue | undefined): Record<string, unknown> {
  if (argumentsValue === undefined || argumentsValue === null) {
    return {};
  }

  if (typeof argumentsValue === 'string') {
    try {
      const parsed = JSON.parse(argumentsValue) as unknown;
      if (typeof parsed === 'object' && parsed !== null && !Array.isArray(parsed)) {
        return parsed as Record<string, unknown>;
      }
      return {};
    } catch {
      return {};
    }
  }

  if (typeof argumentsValue === 'object' && !Array.isArray(argumentsValue)) {
    return argumentsValue as Record<string, unknown>;
  }

  return {};
}

/**
 * Create the MCP handler that manages tools with strict typing
 */
export function createHandler(options: CreateHandlerOptions): ToolsCapabilities {
  const toolsMap = new Map<string, ToolDefinition>();

  // Build a map of tools by name for quick lookup
  for (const tool of options.tools) {
    toolsMap.set(tool.name, tool);
  }

  return {
    handleListTools(_request: ListToolsRequest): ListToolsResponse {
      const tools: Tool[] = Array.from(options.tools).map((tool) => ({
        base: {
          name: tool.name,
          title: tool.name,
        },
        description: tool.description,
        inputSchema: JSON.stringify(tool.schema),
        outputSchema: undefined,
        annotations: undefined,
        meta: undefined,
      }));

      return {
        tools,
        nextCursor: undefined,
        meta: undefined,
      };
    },

    async handleCallTool(request: CallToolRequest): Promise<ToolResult> {
      const tool = toolsMap.get(request.name);

      if (!tool) {
        return createErrorResult(`Unknown tool: ${request.name}`);
      }

      try {
        const args = parseArguments(request.arguments);
        const result = await tool.execute(args);
        return createSuccessResult(result);
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : String(error);
        return createErrorResult(errorMessage);
      }
    },
  };
}
