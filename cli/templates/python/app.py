"""{{project_name}} Tools Capability Provider

A tools capability that provides string manipulation operations.
"""

import json
from typing import Optional
from wit_world import exports
from wit_world.imports import mcp, server_messages, streams


class StringsTools(exports.Tools):
    def list_tools(
        self,
        ctx: server_messages.Context,
        request: mcp.ListToolsRequest,
        client_stream: Optional[streams.OutputStream],
    ) -> mcp.ListToolsResult:
        return mcp.ListToolsResult(
            tools=[
                mcp.Tool(
                    name="reverse",
                    input_schema=json.dumps({
                        "type": "object",
                        "properties": {
                            "text": {"type": "string", "description": "Text to reverse"}
                        },
                        "required": ["text"]
                    }),
                    options=None,
                ),
                mcp.Tool(
                    name="uppercase",
                    input_schema=json.dumps({
                        "type": "object",
                        "properties": {
                            "text": {"type": "string", "description": "Text to convert to uppercase"}
                        },
                        "required": ["text"]
                    }),
                    options=mcp.ToolOptions(
                        meta=None,
                        annotations=None,
                        description="Convert text to uppercase",
                        output_schema=None,
                        title="Uppercase",
                    ),
                ),
            ],
            meta=None,
            next_cursor=None,
        )

    def call_tool(
        self,
        ctx: server_messages.Context,
        request: mcp.CallToolRequest,
        client_stream: Optional[streams.OutputStream],
    ) -> Optional[mcp.CallToolResult]:
        if not request.arguments:
            return error_result("Missing tool arguments")

        try:
            args = json.loads(request.arguments)
        except json.JSONDecodeError as e:
            return error_result(f"Invalid JSON arguments: {e}")

        if request.name == "reverse":
            return reverse_string(args.get("text"))
        elif request.name == "uppercase":
            return uppercase_string(args.get("text"))
        else:
            return None  # We don't handle this tool


def reverse_string(text: str) -> mcp.CallToolResult:
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")

    return success_result(text[::-1])


def uppercase_string(text: str) -> mcp.CallToolResult:
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")

    return success_result(text.upper())


def success_result(text: str) -> mcp.CallToolResult:
    return mcp.CallToolResult(
        content=[mcp.ContentBlock_Text(mcp.TextContent(
            text=mcp.TextData_Text(text),
            options=None,
        ))],
        is_error=None,
        meta=None,
        structured_content=None,
    )


def error_result(message: str) -> mcp.CallToolResult:
    return mcp.CallToolResult(
        content=[mcp.ContentBlock_Text(mcp.TextContent(
            text=mcp.TextData_Text(message),
            options=None,
        ))],
        is_error=True,
        meta=None,
        structured_content=None,
    )


# Export the Tools implementation
Tools = StringsTools
