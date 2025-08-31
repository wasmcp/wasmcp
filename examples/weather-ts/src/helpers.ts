/**
 * Helper functions for building MCP tools with TypeScript
 */

import { ToolDefinition } from './types.js';

export interface ToolsCapabilities {
    handleListTools: (request: any) => any;
    handleCallTool: (request: any) => any;
}

export interface CreateHandlerOptions {
    tools: ToolDefinition[];
}

/**
 * Create a tool definition
 */
export function createTool(definition: ToolDefinition): ToolDefinition {
    return definition;
}

/**
 * Create the MCP handler that manages tools
 */
export function createHandler(options: CreateHandlerOptions): ToolsCapabilities {
    const toolsMap = new Map<string, ToolDefinition>();
    
    // Build a map of tools by name for quick lookup
    for (const tool of options.tools) {
        toolsMap.set(tool.name, tool);
    }
    
    return {
        handleListTools(request: any) {
            const toolsList = options.tools.map(tool => ({
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
                tools: toolsList,
                nextCursor: undefined,
                meta: undefined,
            };
        },
        
        async handleCallTool(request: any) {
            const tool = toolsMap.get(request.name);
            
            if (!tool) {
                return {
                    content: [{
                        tag: 'text',
                        val: {
                            text: `Unknown tool: ${request.name}`,
                            annotations: undefined,
                            meta: undefined,
                        }
                    }],
                    structuredContent: undefined,
                    isError: true,
                    meta: undefined,
                };
            }
            
            try {
                // Parse arguments if they're a string
                const args = typeof request.arguments === 'string' 
                    ? JSON.parse(request.arguments) 
                    : request.arguments || {};
                
                const result = await tool.execute(args);
                
                return {
                    content: [{
                        tag: 'text',
                        val: {
                            text: result,
                            annotations: undefined,
                            meta: undefined,
                        }
                    }],
                    structuredContent: undefined,
                    isError: false,
                    meta: undefined,
                };
            } catch (error) {
                const errorMessage = error instanceof Error ? error.message : String(error);
                return {
                    content: [{
                        tag: 'text',
                        val: {
                            text: errorMessage,
                            annotations: undefined,
                            meta: undefined,
                        }
                    }],
                    structuredContent: undefined,
                    isError: true,
                    meta: undefined,
                };
            }
        }
    };
}