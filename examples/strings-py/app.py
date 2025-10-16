"""String Tools Capability Provider

A tools capability that provides string manipulation operations.
"""

import json
from typing import Optional
from wit_world import exports
from wit_world.imports.protocol import (
    ClientContext,
    ListToolsRequest,
    ListToolsResult,
    CallToolRequest,
    CallToolResult,
    Tool,
    ToolOptions,
    ContentBlock_Text,
    TextContent,
    TextData_Text,
)


class ToolsCapability(exports.ToolsCapability):
    def list_tools(
        self,
        _request: ListToolsRequest,
        _client: ClientContext,
    ) -> ListToolsResult:
        return ListToolsResult(
            tools=[
                Tool(
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
                Tool(
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
                    options=ToolOptions(
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
        request: CallToolRequest,
        _client: ClientContext,
    ) -> Optional[CallToolResult]:
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
            )
        else:
            return None  # We don't handle this tool


def reverse_string(text: str) -> CallToolResult:
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")

    return success_result(text[::-1])


def slice_string(text: str, start: int, end: Optional[int]) -> CallToolResult:
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")
    if not isinstance(start, int):
        return error_result("Missing or invalid parameter 'start'")
    if end is not None and not isinstance(end, int):
        return error_result("Invalid parameter 'end'")

    result = text[start:end] if end is not None else text[start:]
    return success_result(result)


def success_result(text: str) -> CallToolResult:
    return CallToolResult(
        content=[ContentBlock_Text(TextContent(
            text=TextData_Text(text),
            options=None,
        ))],
        is_error=None,
        meta=None,
        structured_content=None,
    )


def error_result(message: str) -> CallToolResult:
    return CallToolResult(
        content=[ContentBlock_Text(TextContent(
            text=TextData_Text(message),
            options=None,
        ))],
        is_error=True,
        meta=None,
        structured_content=None,
    )
