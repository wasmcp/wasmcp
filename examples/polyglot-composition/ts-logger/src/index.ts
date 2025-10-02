import type { OutputStream } from "./generated/interfaces/wasi-io-streams.js";
import { handle as forwardToNextHandler } from "./generated/interfaces/wasmcp-mcp-incoming-handler.js";
import type { Request } from "./generated/interfaces/wasmcp-mcp-request.js";

export const incomingHandler = {
  async handle(request: Request, output: OutputStream): Promise<void> {
    // Log the request
    const feature = request.feature();
    const id = request.id();
    const idStr = id.tag === 'number' ? id.val.toString() : id.val;
    console.error(`[Middleware] Request: feature=${feature}, id=${idStr}`);

    // Forward to next handler in the chain
    forwardToNextHandler(request, output);
  },
};
