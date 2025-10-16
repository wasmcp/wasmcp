"""{{project_name}} - Tools Capability Provider

A clean tools capability component that provides basic string manipulation operations.
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
    """String manipulation tools capability."""

    def list_tools(
        self,
        request: ListToolsRequest,
        client: ClientContext,
    ) -> ListToolsResult:
        return ListToolsResult(
            tools=[
                self._create_reverse_tool(),
                self._create_uppercase_tool(),
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
        elif tool_name == "uppercase":
            return self._execute_uppercase(request)
        else:
            # We don't handle this tool
            return None

    # Tool Definitions
    # ----------------

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

    def _create_uppercase_tool(self) -> Tool:
        return Tool(
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
        )

    # Tool Execution
    # --------------

    def _execute_reverse(self, req: CallToolRequest) -> CallToolResult:
        args = self._parse_json_args(req.arguments)
        text = args.get("text")

        if not isinstance(text, str):
            return self._error_result("Parameter 'text' must be a string")

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

    def _execute_uppercase(self, req: CallToolRequest) -> CallToolResult:
        args = self._parse_json_args(req.arguments)
        text = args.get("text")

        if not isinstance(text, str):
            return self._error_result("Parameter 'text' must be a string")

        result = text.upper()

        return CallToolResult(
            content=[ContentBlock_Text(TextContent(
                text=TextData_Text(result),
                options=None,
            ))],
            is_error=None,
            meta=None,
            structured_content=None,
        )

    # Helper Functions
    # ----------------

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
