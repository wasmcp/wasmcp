"""{{ handler_type_capitalized }} handler for MCP."""

import json
from typing import Optional

from wit_world import exports
from wit_world.imports import (
    request,
    streams,
    incoming_handler as next_handler,
    tools_list_result,
    tools_call_content,
    error_result,
    types,
)


class IncomingHandler(exports.IncomingHandler):
    """Implementation of the MCP incoming handler interface."""

    def handle(self, req: request.Request, output: streams.OutputStream) -> None:
        """Handle an incoming MCP request."""
        if not req.needs(types.ServerCapabilities.TOOLS):
            next_handler.handle(req, output)
            return

        req_id = req.id()
        try:
            params = req.params()
            if isinstance(params, request.Params_ToolsList):
                self.handle_tools_list(req_id, output)
            elif isinstance(params, request.Params_ToolsCall):
                args = params.value
                self.handle_tools_call(req_id, args.name, args.arguments, output)
        except request.McpError as error:
            error_result.write(req_id, output, error)

    def handle_tools_list(
        self,
        req_id: types.Id,
        output: streams.OutputStream,
    ) -> None:
        """Handle tools/list request."""
        tools = [
            tools_list_result.Tool(
                name="echo",
                input_schema=json.dumps(
                    {
                        "type": "object",
                        "properties": {
                            "message": {
                                "type": "string",
                                "description": "The message to echo",
                            }
                        },
                        "required": ["message"],
                    }
                ),
                options=tools_list_result.ToolOptions(
                    meta=None,
                    annotations=None,
                    description="Echo a message back",
                    output_schema=None,
                    title="Echo",
                ),
            ),
        ]

        tools_list_result.write(req_id, output, tools, None)

    def handle_tools_call(
        self,
        req_id: types.Id,
        name: str,
        arguments: Optional[str],
        output: streams.OutputStream,
    ) -> None:
        """Handle tools/call request."""
        try:
            if name == "echo":
                result = self.handle_echo(arguments)
            else:
                tools_call_content.write_error(
                    req_id, output, f"Unknown tool: {name}"
                )
                return

            tools_call_content.write_text(req_id, output, result, None)
        except Exception as e:
            tools_call_content.write_error(
                req_id, output, f"Tool execution failed: {str(e)}"
            )

    def handle_echo(self, arguments: Optional[str]) -> str:
        """Handle the echo tool."""
        if not arguments:
            raise ValueError("Missing arguments")

        args = json.loads(arguments)
        message = args.get("message", "")
        return f"Echo: {message}"
