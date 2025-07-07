// MCP Feature Types

export interface Tool<TArgs = any> {
  name: string;
  description: string;
  inputSchema: object;
  execute: (args: TArgs) => string | Promise<string>;
}

export interface Resource {
  uri: string;
  name: string;
  description?: string;
  mimeType?: string;
  read: () => string | Promise<string>;
}

export interface Prompt<TArgs = any> {
  name: string;
  description?: string;
  arguments?: Array<{
    name: string;
    description?: string;
    required?: boolean;
  }>;
  resolve: (args: TArgs) => PromptMessage[] | Promise<PromptMessage[]>;
}

export interface PromptMessage {
  role: 'user' | 'assistant';
  content: string;
}

// Factory functions

export function createTool<TArgs = any>(config: Tool<TArgs>): Tool<TArgs> {
  return config;
}

export function createResource(config: Resource): Resource {
  return config;
}

export function createPrompt<TArgs = any>(config: Prompt<TArgs>): Prompt<TArgs> {
  return config;
}

// Handler creation

export interface McpFeatures {
  tools?: Tool[];
  resources?: Resource[];
  prompts?: Prompt[];
}

export function createHandler(features: McpFeatures) {
  const tools = features.tools || [];
  const resources = features.resources || [];
  const prompts = features.prompts || [];

  return {
    listTools() {
      return tools.map(tool => ({
        name: tool.name,
        description: tool.description,
        inputSchema: JSON.stringify(tool.inputSchema)
      }));
    },

    async callTool(name: string, argumentsStr: string) {
      let args: any;
      try {
        args = JSON.parse(argumentsStr);
      } catch (e) {
        return {
          tag: 'error',
          val: {
            code: -32602,
            message: `Invalid JSON arguments: ${e}`,
            data: undefined
          }
        };
      }

      const tool = tools.find(t => t.name === name);
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
        const result = await tool.execute(args);
        return {
          tag: 'text',
          val: result
        };
      } catch (e: any) {
        return {
          tag: 'error',
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
      const resource = resources.find(r => r.uri === uri);
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
      return prompts.map(prompt => ({
        name: prompt.name,
        description: prompt.description,
        arguments: prompt.arguments
      }));
    },

    async getPrompt(name: string, argumentsStr: string) {
      const prompt = prompts.find(p => p.name === name);
      if (!prompt) {
        throw {
          code: -32601,
          message: `Prompt not found: ${name}`,
          data: undefined
        };
      }

      let args: any = {};
      if (argumentsStr) {
        try {
          args = JSON.parse(argumentsStr);
        } catch (e) {
          throw {
            code: -32602,
            message: `Invalid JSON arguments: ${e}`,
            data: undefined
          };
        }
      }

      try {
        const messages = await prompt.resolve(args);
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