/**
 * {{project_name}} Prompts Capability
 *
 * A prompts capability that provides example prompt templates.
 */

import type {
  ListPromptsRequest,
  ListPromptsResult,
  GetPromptRequest,
  GetPromptResult,
  Prompt,
  PromptOptions,
  PromptArgument,
  PromptMessage,
  Role,
  ContentBlock,
  TextData,
} from './generated/interfaces/wasmcp-protocol-mcp.js';
import type { Context } from './generated/interfaces/wasmcp-protocol-server-messages.js';
import type { OutputStream } from './generated/interfaces/wasi-io-streams.js';

function listPrompts(
  _ctx: Context,
  _request: ListPromptsRequest,
  _clientStream: OutputStream | null
): ListPromptsResult {
  const prompts: Prompt[] = [
    {
      name: 'code-review',
      options: {
        meta: undefined,
        arguments: [
          {
            name: 'language',
            description: 'Programming language (e.g., python, rust, typescript)',
            required: true,
            title: 'Language',
          },
          {
            name: 'code',
            description: 'Code to review',
            required: true,
            title: 'Code',
          },
        ],
        description: 'Review code for best practices and potential issues',
        title: 'Code Review',
      },
    },
    {
      name: 'greeting',
      options: {
        meta: undefined,
        arguments: [
          {
            name: 'name',
            description: 'Name to greet',
            required: false,
            title: 'Name',
          },
        ],
        description: 'Generate a friendly greeting',
        title: 'Greeting',
      },
    },
  ];

  return {
    prompts,
    nextCursor: undefined,
    meta: undefined,
  };
}

async function getPrompt(
  _ctx: Context,
  request: GetPromptRequest,
  _clientStream: OutputStream | null
): Promise<GetPromptResult | null> {
  if (request.name === 'code-review') {
    // Parse arguments
    const args = request.arguments ? JSON.parse(request.arguments) : {};
    const language = args.language || 'unknown';
    const code = args.code || '';

    return {
      meta: undefined,
      description: `Code review for ${language}`,
      messages: [
        {
          role: 'user' as Role,
          content: {
            tag: 'text',
            val: {
              text: {
                tag: 'text',
                val: `Please review this ${language} code for best practices, potential bugs, and suggest improvements:\n\n${code}`,
              },
              options: undefined,
            },
          } as ContentBlock,
        } as PromptMessage,
      ],
    };
  } else if (request.name === 'greeting') {
    const args = request.arguments ? JSON.parse(request.arguments) : {};
    const name = args.name || 'there';

    return {
      meta: undefined,
      description: 'A friendly greeting',
      messages: [
        {
          role: 'user' as Role,
          content: {
            tag: 'text',
            val: {
              text: {
                tag: 'text',
                val: `Greet ${name} in a friendly and welcoming way.`,
              },
              options: undefined,
            },
          } as ContentBlock,
        } as PromptMessage,
      ],
    };
  }

  return null; // We don't handle this prompt
}

export const prompts = {
  listPrompts,
  getPrompt,
};
