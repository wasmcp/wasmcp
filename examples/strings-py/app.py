"""String Tools Capability Provider

A tools capability that provides string manipulation operations with notifications.
"""

import json
from typing import Optional
from wit_world import exports
from wit_world.imports import mcp, server_handler, streams, notifications


class StringsTools(exports.Tools):
    def list_tools(
        self,
        ctx: server_handler.RequestCtx,
        request: mcp.ListToolsRequest,
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
                    name="slice",
                    input_schema=json.dumps({
                        "type": "object",
                        "properties": {
                            "text": {"type": "string", "description": "Text to slice"},
                            "start": {"type": "integer", "description": "Start index (inclusive)"},
                            "end": {"type": "integer", "description": "End index (exclusive, optional)"}
                        },
                        "required": ["text", "start"]
                    }),
                    options=mcp.ToolOptions(
                        meta=None,
                        annotations=None,
                        description="Extract substring by start/end indices (Python slicing)",
                        output_schema=None,
                        title="Slice",
                    ),
                ),
            ],
            meta=None,
            next_cursor=None,
        )

    def call_tool(
        self,
        ctx: server_handler.RequestCtx,
        request: mcp.CallToolRequest,
    ) -> Optional[mcp.CallToolResult]:
        if not request.arguments:
            return error_result("Missing tool arguments")

        try:
            args = json.loads(request.arguments)
        except json.JSONDecodeError as e:
            return error_result(f"Invalid JSON arguments: {e}")

        if request.name == "reverse":
            return reverse_string(args.get("text"))
        elif request.name == "slice":
            return slice_string(
                text=args.get("text"),
                start=args.get("start"),
                end=args.get("end"),
                message_stream=ctx.message_stream,
            )
        else:
            return None  # We don't handle this tool


def reverse_string(text: str) -> mcp.CallToolResult:
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")

    return success_result(text[::-1])


def slice_string(text: str, start: int, end: Optional[int], message_stream: Optional[streams.OutputStream] = None) -> mcp.CallToolResult:
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")
    if not isinstance(start, int):
        return error_result("Missing or invalid parameter 'start'")
    if end is not None and not isinstance(end, int):
        return error_result("Invalid parameter 'end'")

    # Send notification about the slicing operation
    if message_stream:
        end_str = str(end) if end is not None else "end"
        msg = f"Slicing text from index {start} to {end_str}"
        notifications.log(message_stream, msg, mcp.LogLevel.DEBUG, "slice")

    result = text[start:end] if end is not None else text[start:]
    return success_result(result)


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
