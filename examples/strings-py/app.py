"""String Tools Capability Provider

A clean tools capability that provides string manipulation operations.
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
        request: ListToolsRequest,
        client: ClientContext,
    ) -> ListToolsResult:
        return ListToolsResult(
            tools=[
                self._create_reverse_tool(),
                self._create_slice_tool(),
                self._create_contains_tool(),
                self._create_split_tool(),
            ],
            next_cursor=None,
            meta=None,
        )

    def call_tool(
        self,
        request: CallToolRequest,
        client: ClientContext,
    ) -> Optional[CallToolResult]:
        tool_name = request.name

        if tool_name == "reverse":
            return self._execute_reverse(request)
        elif tool_name == "slice":
            return self._execute_slice(request)
        elif tool_name == "contains":
            return self._execute_contains(request)
        elif tool_name == "split":
            return self._execute_split(request)
        else:
            # We don't handle this tool
            return None

    # Tool definitions

    def _create_reverse_tool(self) -> Tool:
        return Tool(
            name="reverse",
            input_schema=json.dumps({
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "Text to reverse"}
                },
                "required": ["text"]
            }),
            options=ToolOptions(
                meta=None,
                annotations=None,
                description="Reverse a string",
                output_schema=None,
                title="Reverse",
            ),
        )

    def _create_slice_tool(self) -> Tool:
        return Tool(
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
        )

    def _create_contains_tool(self) -> Tool:
        return Tool(
            name="contains",
            input_schema=json.dumps({
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "Text to search in"},
                    "substring": {"type": "string", "description": "Substring to search for"}
                },
                "required": ["text", "substring"]
            }),
            options=ToolOptions(
                meta=None,
                annotations=None,
                description="Check if text contains substring",
                output_schema=None,
                title="Contains",
            ),
        )

    def _create_split_tool(self) -> Tool:
        return Tool(
            name="split",
            input_schema=json.dumps({
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "Text to split"},
                    "delimiter": {"type": "string", "description": "Delimiter to split by (optional, defaults to whitespace)"}
                },
                "required": ["text"]
            }),
            options=ToolOptions(
                meta=None,
                annotations=None,
                description="Split string by delimiter into array of parts",
                output_schema=None,
                title="Split",
            ),
        )

    # Tool execution

    def _execute_reverse(self, req: CallToolRequest) -> CallToolResult:
        text = self._get_string_arg(req.arguments, "text")
        result = text[::-1]
        return CallToolResult(
            content=[ContentBlock_Text(TextContent(
                text=TextData_Text(result),
                options=None,
            ))],
            is_error=None,
            meta=None,
            structured_content=None,
        )

    def _execute_slice(self, req: CallToolRequest) -> CallToolResult:
        args = self._parse_json_args(req.arguments)
        text = args.get("text")
        start = args.get("start")
        end = args.get("end")

        if not isinstance(text, str):
            return self._error_result("Missing or invalid parameter 'text'")
        if not isinstance(start, int):
            return self._error_result("Missing or invalid parameter 'start'")

        if end is None:
            result = text[start:]
        else:
            if not isinstance(end, int):
                return self._error_result("Invalid parameter 'end'")
            result = text[start:end]

        return CallToolResult(
            content=[ContentBlock_Text(TextContent(
                text=TextData_Text(result),
                options=None,
            ))],
            is_error=None,
            meta=None,
            structured_content=None,
        )

    def _execute_contains(self, req: CallToolRequest) -> CallToolResult:
        text = self._get_string_arg(req.arguments, "text")
        substring = self._get_string_arg(req.arguments, "substring")
        result = "true" if substring in text else "false"

        return CallToolResult(
            content=[ContentBlock_Text(TextContent(
                text=TextData_Text(result),
                options=None,
            ))],
            is_error=None,
            meta=None,
            structured_content=None,
        )

    def _execute_split(self, req: CallToolRequest) -> CallToolResult:
        args = self._parse_json_args(req.arguments)
        text = args.get("text")
        delimiter = args.get("delimiter")

        if not isinstance(text, str):
            return self._error_result("Missing or invalid parameter 'text'")

        if delimiter is None:
            parts = text.split()  # Split on whitespace
        else:
            if not isinstance(delimiter, str):
                return self._error_result("Invalid parameter 'delimiter'")
            parts = text.split(delimiter)

        # Return as JSON array
        result = json.dumps(parts)
        return CallToolResult(
            content=[ContentBlock_Text(TextContent(
                text=TextData_Text(result),
                options=None,
            ))],
            is_error=None,
            meta=None,
            structured_content=None,
        )

    # Helper functions

    def _parse_json_args(self, arguments: Optional[str]) -> dict:
        if arguments is None:
            return {}

        try:
            parsed = json.loads(arguments)
            if isinstance(parsed, dict):
                return parsed
            return {}
        except json.JSONDecodeError:
            return {}

    def _get_string_arg(self, arguments: Optional[str], name: str) -> str:
        args = self._parse_json_args(arguments)
        value = args.get(name)
        if not isinstance(value, str):
            return ""
        return value

    def _error_result(self, message: str) -> CallToolResult:
        return CallToolResult(
            content=[ContentBlock_Text(TextContent(
                text=TextData_Text(message),
                options=None,
            ))],
            is_error=True,
            meta=None,
            structured_content=None,
        )
