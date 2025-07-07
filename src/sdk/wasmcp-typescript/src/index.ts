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
  tools?: Array<Tool<z.ZodType>>;
  resources?: Array<Resource>;
  prompts?: Array<Prompt<z.ZodType>>;
}

export interface ToolInfo {
  name: string;
  description: string;
  inputSchema?: string;
}

export interface ResourceInfo {
  uri: string;
  name: string;
  description?: string;
  mimeType?: string;
}

export interface PromptInfo {
  name: string;
  description?: string;
  arguments?: Array<{
    name: string;
    description?: string;
    required: boolean;
  }>;
}

export interface ErrorResponse {
  tag: 'error';
  val: {
    code: number;
    message: string;
    data?: unknown;
  };
}

export interface SuccessResponse<T> {
  tag: 'ok';
  val: T;
}

export type HandlerResponse<T> = ErrorResponse | SuccessResponse<T>;

export interface Handler {
  listTools(): ToolInfo[];
  listResources(): ResourceInfo[];
  listPrompts(): PromptInfo[];
  callTool(name: string, argumentsStr: string): Promise<HandlerResponse<string>>;
  readResource(uri: string): Promise<HandlerResponse<string>>;
  getPrompt(name: string, argumentsStr: string): Promise<HandlerResponse<PromptMessage[]>>;
}

export function createHandler(config: HandlerConfig): Handler {
  const tools = config.tools ?? [];
  const resources = config.resources ?? [];
  const prompts = config.prompts ?? [];
  
  const toolMap = new Map(tools.map(t => [t.name, t]));
  const resourceMap = new Map(resources.map(r => [r.uri, r]));
  const promptMap = new Map(prompts.map(p => [p.name, p]));

  return {
    listTools(): ToolInfo[] {
      return tools.map(tool => ({
        name: tool.name,
        description: tool.description,
        inputSchema: JSON.stringify(z.toJSONSchema(tool.schema))
      }));
    },

    async callTool(name: string, argumentsStr: string): Promise<HandlerResponse<string>> {
      const tool = toolMap.get(name);
      if (!tool) {
        return {
          tag: 'error',
          val: {
            code: -32601,
            message: `Unknown tool: ${name}`,
            data: undefined
          }
        };
      }

      try {
        // Parse the arguments
        const args = JSON.parse(argumentsStr) as unknown;
        
        // Validate using the tool's schema
        if (tool.schema !== undefined) {
          const parsed = tool.schema.safeParse(args);
          if (!parsed.success) {
            return {
              tag: 'error',
              val: {
                code: -32602,
                message: formatZodError(parsed.error),
                data: parsed.error.format()
              }
            };
          }
          
          // Execute with validated arguments
          const result = await tool.execute(parsed.data);
          return { tag: 'ok', val: result };
        } else {
          // No schema, pass through
          const result = await tool.execute(args as z.infer<typeof tool.schema>);
          return { tag: 'ok', val: result };
        }
      } catch (e) {
        const error = e as Error;
        return {
          tag: 'error',
          val: {
            code: -32603,
            message: error.message ?? String(error),
            data: undefined
          }
        };
      }
    },

    listResources(): ResourceInfo[] {
      return resources.map(resource => ({
        uri: resource.uri,
        name: resource.name,
        description: resource.description,
        mimeType: resource.mimeType
      }));
    },

    async readResource(uri: string): Promise<HandlerResponse<string>> {
      const resource = resourceMap.get(uri);
      if (!resource) {
        return {
          tag: 'error',
          val: {
            code: -32002,
            message: `Resource not found: ${uri}`,
            data: undefined
          }
        };
      }

      try {
        const content = await resource.read();
        return { tag: 'ok', val: content };
      } catch (e) {
        const error = e as Error;
        return {
          tag: 'error',
          val: {
            code: -32603,
            message: error.message ?? String(error),
            data: undefined
          }
        };
      }
    },

    listPrompts(): PromptInfo[] {
      return prompts.map(prompt => {
        let args: PromptInfo['arguments'];
        
        if (prompt.schema !== undefined && 'shape' in prompt.schema && prompt.schema.shape !== undefined) {
          const shape = prompt.schema.shape as Record<string, z.ZodType>;
          const schemaDef = prompt.schema._def as { required?: string[] } | undefined;
          const requiredFields = schemaDef?.required ?? [];
          
          args = Object.entries(shape).map(([name, field]) => ({
            name,
            description: (field as z.ZodType & { description?: string }).description,
            required: Array.isArray(requiredFields) ? requiredFields.includes(name) : false
          }));
        }
        
        return {
          name: prompt.name,
          description: prompt.description,
          arguments: args
        };
      });
    },

    async getPrompt(name: string, argumentsStr: string): Promise<HandlerResponse<PromptMessage[]>> {
      const prompt = promptMap.get(name);
      if (!prompt) {
        return {
          tag: 'error',
          val: {
            code: -32002,
            message: `Prompt not found: ${name}`,
            data: undefined
          }
        };
      }

      try {
        const args = JSON.parse(argumentsStr) as unknown;
        
        if (prompt.schema !== undefined) {
          const parsed = prompt.schema.safeParse(args);
          if (!parsed.success) {
            return {
              tag: 'error',
              val: {
                code: -32602,
                message: formatZodError(parsed.error),
                data: parsed.error.format()
              }
            };
          }
          const messages = await prompt.resolve(parsed.data);
          return { tag: 'ok', val: messages };
        } else {
          const messages = await prompt.resolve(args as z.infer<NonNullable<typeof prompt.schema>>);
          return { tag: 'ok', val: messages };
        }
      } catch (e) {
        const error = e as Error;
        return {
          tag: 'error',
          val: {
            code: -32603,
            message: error.message ?? String(error),
            data: undefined
          }
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
}): Tool<TSchema> {
  return new (class extends Tool<TSchema> {
    readonly name = config.name;
    readonly description = config.description;
    readonly schema = config.schema;
    execute = config.execute;
  })();
}

export function createResource(config: {
  uri: string;
  name: string;
  description?: string;
  mimeType?: string;
  read: () => string | Promise<string>;
}): Resource {
  return new (class extends Resource {
    readonly uri = config.uri;
    readonly name = config.name;
    readonly description = config.description;
    readonly mimeType = config.mimeType;
    read = config.read;
  })();
}

export function createPrompt<TSchema extends z.ZodType>(config: {
  name: string;
  description?: string;
  schema?: TSchema;
  resolve: (args: z.infer<TSchema>) => PromptMessage[] | Promise<PromptMessage[]>;
}): Prompt<TSchema> {
  return new (class extends Prompt<TSchema> {
    readonly name = config.name;
    readonly description = config.description;
    readonly schema = config.schema;
    resolve = config.resolve;
  })();
}

// Helper to format Zod errors nicely
function formatZodError(error: z.ZodError): string {
  const issues = error.issues.map(issue => {
    const path = issue.path.join('.');
    return path ? `${path}: ${issue.message}` : issue.message;
  });
  return `Validation error: ${issues.join(', ')}`;
}

// Re-export zod for convenience
export { z };