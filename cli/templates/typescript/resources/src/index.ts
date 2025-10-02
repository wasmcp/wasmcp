import type { OutputStream } from "./generated/interfaces/wasi-io-streams.js";
import { handle as forwardToNextHandler } from "./generated/interfaces/wasmcp-mcp-incoming-handler.js";
import type { Request } from "./generated/interfaces/wasmcp-mcp-request.js";
import type { Resource } from "./generated/interfaces/wasmcp-mcp-resources-list-result.js";
import { write as writeResourcesList } from "./generated/interfaces/wasmcp-mcp-resources-list-result.js";
import type { Contents } from "./generated/interfaces/wasmcp-mcp-resources-read-result.js";
import { write as writeResourcesRead } from "./generated/interfaces/wasmcp-mcp-resources-read-result.js";
import { write as writeMcpError } from "./generated/interfaces/wasmcp-mcp-error-result.js";
import type { Error as McpError } from "./generated/interfaces/wasmcp-mcp-error.js";
import type { Id } from "./generated/interfaces/wasmcp-mcp-types.js";

export const incomingHandler = {
  async handle(request: Request, output: OutputStream): Promise<void> {
    if (!request.needs({ resources: true })) {
      forwardToNextHandler(request, output);
      return;
    }

    const id = request.id();
    try {
      const params = request.params();
      if (params.tag === "resources-list") {
        handleResourcesList(id, output);
      } else if (params.tag === "resources-read") {
        handleResourcesRead(id, params.val, output);
      }
    } catch (error) {
      writeMcpError(id, output, error as McpError);
    }
  },
};

function handleResourcesList(id: Id, output: OutputStream): void {
  const resources: Resource[] = [
    {
      uri: "file:///example.txt",
      name: "example.txt",
      options: {
        size: undefined,
        title: undefined,
        description: "An example text resource",
        mimeType: "text/plain",
        annotations: undefined,
        meta: undefined,
      },
    },
  ];

  writeResourcesList(id, output, resources, undefined);
}

function handleResourcesRead(id: Id, uri: string, output: OutputStream): void {
  let content: string;
  if (uri === "file:///example.txt") {
    content = readExample();
  } else {
    content = `Unknown resource: ${uri}`;
  }

  const resourceContents: Contents = {
    uri,
    data: new TextEncoder().encode(content),
    options: undefined,
  };

  writeResourcesRead(id, output, resourceContents, undefined);
}

function readExample(): string {
  return "This is the content of example.txt";
}
