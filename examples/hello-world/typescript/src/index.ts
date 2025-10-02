import type { OutputStream } from "./generated/interfaces/wasi-io-streams.js";
import { handle as forwardToNextHandler } from "./generated/interfaces/wasmcp-mcp-incoming-handler.js";
import type { Request } from "./generated/interfaces/wasmcp-mcp-request.js";
import type { Tool } from "./generated/interfaces/wasmcp-mcp-tools-list-result.js";
import { write as writeToolsList } from "./generated/interfaces/wasmcp-mcp-tools-list-result.js";
import { writeText, writeError as writeToolError } from "./generated/interfaces/wasmcp-mcp-tools-call-content.js";
import { write as writeMcpError } from "./generated/interfaces/wasmcp-mcp-error-result.js";
import type { Error as McpError } from "./generated/interfaces/wasmcp-mcp-error.js";
import type { Id } from "./generated/interfaces/wasmcp-mcp-types.js";

export const incomingHandler = {
  async handle(request: Request, output: OutputStream): Promise<void> {
    if (!request.needs({ tools: true })) {
      forwardToNextHandler(request, output);
      return;
    }

    const id = request.id();
    try {
      const params = request.params();
      if (params.tag === "tools-list") {
        handleToolsList(id, output);
      } else if (params.tag === "tools-call") {
        await handleToolsCall(id, params.val.name, params.val.arguments, output);
      }
    } catch (error) {
      writeMcpError(id, output, error as McpError);
    }
  },
};

function handleToolsList(id: Id, output: OutputStream): void {
  const tools: Tool[] = [
    {
      name: "echo",
      inputSchema: JSON.stringify({
        type: "object",
        properties: {
          message: {
            type: "string",
            description: "The message to echo",
          },
        },
        required: ["message"],
      }),
      options: {
        description: "Echo a message back",
        title: "Echo",
      },
    },
  ];

  writeToolsList(id, output, tools, undefined);
}

async function handleToolsCall(
  id: Id,
  name: string,
  argumentsJson: string | undefined,
  output: OutputStream,
): Promise<void> {
  let args: Record<string, any> = {};
  if (argumentsJson) {
    try {
      args = JSON.parse(argumentsJson);
    } catch (e) {
      writeToolError(id, output, `Invalid JSON arguments: ${e}`);
      return;
    }
  }

  try {
    let resultText: string;

    switch (name) {
      case "echo":
        resultText = handleEcho(args);
        break;
      default:
        writeToolError(id, output, `Unknown tool: ${name}`);
        return;
    }

    writeText(id, output, resultText, undefined);
  } catch (error) {
    writeToolError(
      id,
      output,
      `Tool execution failed: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
}

function handleEcho(args: Record<string, any>): string {
  const message = args.message || "";
  return `Echo: ${message}`;
}
