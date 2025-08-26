import { describe, it, expect } from 'vitest';
import { tools, echoTool } from '../src/index';

describe('{{project-name}} MCP Handler', () => {
    describe('Tools', () => {
        it('should export at least one tool', () => {
            expect(tools).toBeDefined();
            expect(Array.isArray(tools)).toBe(true);
            expect(tools.length).toBeGreaterThan(0);
        });

        it('should have the echo tool', () => {
            const tool = tools.find(t => t.name === 'echo');
            expect(tool).toBeDefined();
            expect(tool).toBe(echoTool);
        });

        it('should have correct tool configuration', () => {
            expect(echoTool.name).toBe('echo');
            expect(echoTool.description).toBe('Echo a message back to the user');
        });

        it('should handle valid input for echo tool', async () => {
            const result = await echoTool.execute({ message: 'test input' });
            expect(result).toBe('Echo: test input');
        });

        it('should validate input schema', () => {
            // Test that the schema requires a message
            const parseResult = echoTool.schema.safeParse({});
            expect(parseResult.success).toBe(false);
            
            const validParseResult = echoTool.schema.safeParse({ message: 'test' });
            expect(validParseResult.success).toBe(true);
        });

        it('should reject empty messages', () => {
            const parseResult = echoTool.schema.safeParse({ message: '' });
            expect(parseResult.success).toBe(false);
        });
    });
});