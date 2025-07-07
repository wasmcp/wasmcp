import { createHandler } from '@fastertools/ftl-sdk';
import { tools, resources, prompts } from './features.js';

// Export the handler implementation for componentize-js
export const handler = createHandler({
    tools,
    resources,
    prompts
});