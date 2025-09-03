/**
 * Type declarations for helpers.js
 */

export interface Tool {
    name: string;
    description: string;
    schema: any;
    execute: (args: any) => any;
    annotations?: any;
    getDefinition(): any;
    run(args: any): Promise<any>;
}

export interface HandlerConfig {
    tools?: any[];
    serverInfo?: {
        name: string;
        version: string;
        instructions: string;
    };
    authConfig?: any;
}

export interface Handler {
    coreCapabilities: {
        handleInitialize(request: any): any;
        handleInitialized(): void;
        handlePing(): void;
        handleShutdown(): void;
        getAuthConfig(): any;
    };
    toolsCapabilities: {
        handleListTools(request: any): any;
        handleCallTool(request: any): Promise<any>;
    };
}

export function createTool<T = any>(config: any): Tool;
export function createHandler(config: HandlerConfig): Handler;
export function textResult(text: string): any;
export function errorResult(message: string): any;