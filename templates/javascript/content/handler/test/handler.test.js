import { describe, it, expect } from 'vitest';
import { tools } from '../src/features.js';

describe('{{project-name}} MCP Handler', () => {
    describe('Tools', () => {
        it('should export at least one tool', () => {
            expect(tools).toBeDefined();
            expect(Array.isArray(tools)).toBe(true);
            expect(tools.length).toBeGreaterThan(0);
        });

        it('should have the echo tool', () => {
            const echoTool = tools.find(t => t.name === 'echo');
            expect(echoTool).toBeDefined();
            expect(echoTool?.description).toBe('Echo a message back to the user');
        });

        it('should handle valid input for echo tool', async () => {
            const echoTool = tools.find(t => t.name === 'echo');
            expect(echoTool).toBeDefined();
            expect(echoTool?.execute).toBeDefined();
            
            const result = await echoTool?.execute({ message: 'test input' });
            expect(result).toBeDefined();
            expect(typeof result).toBe('string');
            expect(result).toContain('test input');
        });

        it('should handle missing message with default', async () => {
            const echoTool = tools.find(t => t.name === 'echo');
            expect(echoTool).toBeDefined();
            
            const result = await echoTool?.execute({});
            expect(result).toBeDefined();
            expect(typeof result).toBe('string');
            expect(result).toContain('Hello, world!');
        });
    });
});