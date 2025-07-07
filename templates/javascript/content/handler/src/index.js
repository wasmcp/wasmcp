import { createTool, createHandler, z } from 'wasmcp';

// Use factory functions to create tools
const echoTool = createTool({
  name: 'echo',
  description: 'Echo a message back to the user',
  schema: z.object({
    message: z.string().min(1).describe('Message to echo back')
  }),
  execute: (args) => {
    return `Echo: ${args.message}`;
  }
});

// Add more tools here...

// Export the handler implementation for componentize-js
export const handler = createHandler({
  tools: [echoTool]
});