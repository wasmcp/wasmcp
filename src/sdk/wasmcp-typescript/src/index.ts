import * as z from 'zod/v4';

// Core types
export interface PromptMessage {
  role: 'user' | 'assistant';
  content: string;
}

// Base classes for MCP features with Zod integration
export abstract class Tool<TSchema extends z.ZodType = z.ZodAny> {
  abstract readonly name: string;
  abstract readonly description: string;
  abstract readonly schema: TSchema;
  
  // The execute method uses the inferred type from the schema
  abstract execute(args: z.infer<TSchema>): string | Promise<string>;
}

export abstract class Resource {
  abstract readonly uri: string;
  abstract readonly name: string;
  readonly description?: string;
  readonly mimeType?: string;
  abstract read(): string | Promise<string>;
}

export abstract class Prompt<TSchema extends z.ZodType = z.ZodAny> {
  abstract readonly name: string;
  readonly description?: string;
  readonly schema?: TSchema;
  
  abstract resolve(args: z.infer<TSchema>): PromptMessage[] | Promise<PromptMessage[]>;
}

// Handler creation with automatic validation
export interface HandlerConfig {
  tools?: Array<new () => Tool<any>>;
  resources?: Array<new () => Resource>;
  prompts?: Array<new () => Prompt<any>>;
}

export function createHandler(config: HandlerConfig) {
  const tools = config.tools?.map(T => new T()) || [];
  const resources = config.resources?.map(R => new R()) || [];
  const prompts = config.prompts?.map(P => new P()) || [];
  
  const toolMap = new Map(tools.map(t => [t.name, t]));
  const resourceMap = new Map(resources.map(r => [r.uri, r]));
  const promptMap = new Map(prompts.map(p => [p.name, p]));

  return {
    listTools() {
      return tools.map(tool => ({
        name: tool.name,
        description: tool.description,
        inputSchema: JSON.stringify(z.toJSONSchema(tool.schema))
      }));
    },

    async callTool(name: string, argumentsStr: string) {
      const tool = toolMap.get(name);
      if (!tool) {
        return {
          tag: 'error' as const,
          val: {
            code: -32601,
            message: `Unknown tool: ${name}`,
            data: undefined
          }
        };
      }

      let args: unknown;
      try {
        args = JSON.parse(argumentsStr);
      } catch (e) {
        return {
          tag: 'error' as const,
          val: {
            code: -32602,
            message: `Invalid JSON: ${e}`,
            data: undefined
          }
        };
      }

      // Validate args with Zod
      const parsed = tool.schema.safeParse(args);
      if (!parsed.success) {
        return {
          tag: 'error' as const,
          val: {
            code: -32602,
            message: `Invalid arguments: ${z.prettifyError(parsed.error)}`,
            data: JSON.stringify(parsed.error.issues)
          }
        };
      }

      try {
        const result = await tool.execute(parsed.data);
        return {
          tag: 'text' as const,
          val: result
        };
      } catch (e: any) {
        return {
          tag: 'error' as const,
          val: {
            code: -32603,
            message: e.message || String(e),
            data: undefined
          }
        };
      }
    },

    listResources() {
      return resources.map(resource => ({
        uri: resource.uri,
        name: resource.name,
        description: resource.description,
        mimeType: resource.mimeType
      }));
    },

    async readResource(uri: string) {
      const resource = resourceMap.get(uri);
      if (!resource) {
        throw {
          code: -32601,
          message: `Resource not found: ${uri}`,
          data: undefined
        };
      }

      try {
        const contents = await resource.read();
        return {
          contents,
          mimeType: resource.mimeType
        };
      } catch (e: any) {
        throw {
          code: -32603,
          message: e.message || String(e),
          data: undefined
        };
      }
    },

    listPrompts() {
      return prompts.map(prompt => {
        // Convert Zod schema to prompt arguments
        const args = prompt.schema && z.toJSONSchema(prompt.schema);
        const arguments_ = [];
        
        if (args && typeof args === 'object' && 'properties' in args) {
          const properties = args.properties as Record<string, any>;
          const required = Array.isArray(args.required) ? args.required : [];
          
          for (const [key, value] of Object.entries(properties)) {
            arguments_.push({
              name: key,
              description: value.description,
              required: required.includes(key)
            });
          }
        }

        return {
          name: prompt.name,
          description: prompt.description,
          arguments: arguments_
        };
      });
    },

    async getPrompt(name: string, argumentsStr: string) {
      const prompt = promptMap.get(name);
      if (!prompt) {
        throw {
          code: -32601,
          message: `Prompt not found: ${name}`,
          data: undefined
        };
      }

      let args: unknown = {};
      if (argumentsStr) {
        try {
          args = JSON.parse(argumentsStr);
        } catch (e) {
          throw {
            code: -32602,
            message: `Invalid JSON: ${e}`,
            data: undefined
          };
        }
      }

      if (prompt.schema) {
        const parsed = prompt.schema.safeParse(args);
        if (!parsed.success) {
          throw {
            code: -32602,
            message: `Invalid arguments: ${z.prettifyError(parsed.error)}`,
            data: JSON.stringify(parsed.error.issues)
          };
        }
        args = parsed.data;
      }

      try {
        const messages = await prompt.resolve(args as any);
        return messages;
      } catch (e: any) {
        throw {
          code: -32603,
          message: e.message || String(e),
          data: undefined
        };
      }
    }
  };
}

// Factory functions for simpler API
export function createTool<TSchema extends z.ZodType>(config: {
  name: string;
  description: string;
  schema: TSchema;
  execute: (args: z.infer<TSchema>) => string | Promise<string>;
}): new () => Tool<TSchema> {
  return class extends Tool<TSchema> {
    readonly name = config.name;
    readonly description = config.description;
    readonly schema = config.schema;
    execute = config.execute;
  };
}

export function createResource(config: {
  uri: string;
  name: string;
  description?: string;
  mimeType?: string;
  read: () => string | Promise<string>;
}): new () => Resource {
  return class extends Resource {
    readonly uri = config.uri;
    readonly name = config.name;
    readonly description = config.description;
    readonly mimeType = config.mimeType;
    read = config.read;
  };
}

export function createPrompt<TSchema extends z.ZodType>(config: {
  name: string;
  description?: string;
  schema?: TSchema;
  resolve: (args: z.infer<TSchema>) => PromptMessage[] | Promise<PromptMessage[]>;
}): new () => Prompt<TSchema> {
  return class extends Prompt<TSchema> {
    readonly name = config.name;
    readonly description = config.description;
    readonly schema = config.schema;
    resolve = config.resolve;
  };
}

// Re-export zod for convenience
export { z } from 'zod/v4';