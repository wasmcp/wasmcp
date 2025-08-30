/**
 * Helper library for building MCP tool handlers in JavaScript
 * Provides a clean API for defining and registering tools
 */

/**
 * Base class for MCP tools
 */
export class Tool {
    constructor(config) {
        this.name = config.name;
        this.description = config.description;
        this.schema = config.schema;
        this.execute = config.execute;
        this.annotations = config.annotations;
    }

    /**
     * Get the tool definition for MCP
     */
    getDefinition() {
        return {
            base: {
                name: this.name,
                title: this.name
            },
            description: this.description,
            inputSchema: typeof this.schema === 'string' 
                ? this.schema 
                : JSON.stringify(this.schema),
            outputSchema: null,
            annotations: this.annotations || null,
            meta: null
        };
    }

    /**
     * Execute the tool with parsed arguments
     */
    async run(args) {
        const result = await this.execute(args);
        
        // If the result is a string, wrap it in the MCP format
        if (typeof result === 'string') {
            return textResult(result);
        }
        
        // If it's already in the correct format, return as-is
        if (result.content && Array.isArray(result.content)) {
            return result;
        }
        
        // If it's an error, format appropriately
        if (result instanceof Error) {
            return errorResult(result.message);
        }
        
        // Otherwise, try to convert to text
        return textResult(String(result));
    }
}

/**
 * Factory function to create a tool
 */
export function createTool(config) {
    return new Tool(config);
}

/**
 * Create a text result in MCP format
 */
export function textResult(text) {
    return {
        content: [{
            tag: 'text',
            val: {
                text: text,
                annotations: null,
                meta: null
            }
        }],
        structuredContent: null,
        isError: false,
        meta: null
    };
}

/**
 * Create an error result in MCP format
 */
export function errorResult(message) {
    return {
        content: [{
            tag: 'text',
            val: {
                text: message,
                annotations: null,
                meta: null
            }
        }],
        structuredContent: null,
        isError: true,
        meta: null
    };
}

/**
 * Create a handler that implements the MCP tool-handler interface
 */
export function createHandler(config) {
    const tools = config.tools || [];
    
    // Build a map of tools by name for quick lookup
    const toolMap = new Map();
    for (const tool of tools) {
        if (tool instanceof Tool) {
            toolMap.set(tool.name, tool);
        } else {
            // Allow plain objects to be converted to Tool instances
            const toolInstance = new Tool(tool);
            toolMap.set(toolInstance.name, toolInstance);
        }
    }
    
    return {
        /**
         * Handle list-tools request
         */
        handleListTools(request) {
            const toolDefinitions = Array.from(toolMap.values()).map(tool => 
                tool.getDefinition()
            );
            
            return {
                tools: toolDefinitions,
                nextCursor: null,
                meta: null
            };
        },
        
        /**
         * Handle call-tool request
         */
        async handleCallTool(request) {
            const tool = toolMap.get(request.name);
            
            if (!tool) {
                return errorResult(`Unknown tool: ${request.name}`);
            }
            
            try {
                // Parse arguments if they're a string
                const args = request.arguments 
                    ? (typeof request.arguments === 'string' 
                        ? JSON.parse(request.arguments) 
                        : request.arguments)
                    : {};
                
                // Execute the tool
                return await tool.run(args);
            } catch (error) {
                return errorResult(`Error executing ${request.name}: ${error.message}`);
            }
        }
    };
}

/**
 * Helper function to register multiple tools at once
 * Returns the handler export expected by jco
 */
export function registerTools(...tools) {
    const handler = createHandler({ tools });
    
    // Export as the name expected by the WIT interface
    return {
        toolHandler: handler
    };
}