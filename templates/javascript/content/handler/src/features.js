import { createTool, createResource, createPrompt } from '@fastertools/ftl-sdk';

// Define your tools
export const tools = [
  createTool({
    name: 'echo',
    description: 'Echo a message back to the user',
    inputSchema: {
      type: 'object',
      properties: {
        message: { 
          type: 'string', 
          description: 'Message to echo back' 
        }
      },
      required: ['message']
    },
    execute: async (args) => {
      const message = args.message || 'Hello, world!';
      return `Echo: ${message}`;
    }
  }),
  // Add more tools here
];

// Define your resources
export const resources = [
  // Example:
  // createResource({
  //   uri: 'file:///example.txt',
  //   name: 'Example File',
  //   description: 'An example text file',
  //   mimeType: 'text/plain',
  //   read: async () => {
  //     return 'File contents here';
  //   }
  // }),
];

// Define your prompts
export const prompts = [
  // Example:
  // createPrompt({
  //   name: 'greeting',
  //   description: 'Generate a greeting message',
  //   arguments: [
  //     { name: 'name', description: 'Name to greet', required: true }
  //   ],
  //   resolve: async (args) => {
  //     return [
  //       { role: 'user', content: `Please greet ${args.name}` },
  //       { role: 'assistant', content: `Hello, ${args.name}! How can I help you today?` }
  //     ];
  //   }
  // }),
];