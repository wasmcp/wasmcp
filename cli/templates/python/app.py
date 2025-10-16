"""{{project_name}} Tools Capability Provider

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
                    name="uppercase",
                    input_schema=json.dumps({
                        "type": "object",
                        "properties": {
                            "text": {"type": "string", "description": "Text to convert to uppercase"}
                        },
                        "required": ["text"]
                    }),
                    options=ToolOptions(
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
        elif request.name == "uppercase":
            return uppercase_string(args.get("text"))
        else:
            return None  # We don't handle this tool


def reverse_string(text: str) -> CallToolResult:
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")

    return success_result(text[::-1])


def uppercase_string(text: str) -> CallToolResult:
    if not isinstance(text, str):
        return error_result("Missing or invalid parameter 'text'")

    return success_result(text.upper())


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
