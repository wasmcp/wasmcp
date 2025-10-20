"""{{project_name}} Prompts Capability Provider

A prompts capability that provides example prompt templates.
"""

from typing import Optional
from wit_world import exports
from wit_world.imports import mcp, server_messages, streams


class ExamplePrompts(exports.Prompts):
    def list_prompts(
        self,
        ctx: server_messages.Context,
        request: mcp.ListPromptsRequest,
        client_stream: Optional[streams.OutputStream],
    ) -> mcp.ListPromptsResult:
        return mcp.ListPromptsResult(
            prompts=[
                mcp.Prompt(
                    name="code-review",
                    options=mcp.PromptOptions(
                        meta=None,
                        arguments=[
                            mcp.PromptArgument(
                                name="language",
                                description="Programming language (e.g., python, rust, typescript)",
                                required=True,
                                title="Language",
                            ),
                            mcp.PromptArgument(
                                name="code",
                                description="Code to review",
                                required=True,
                                title="Code",
                            ),
                        ],
                        description="Review code for best practices and potential issues",
                        title="Code Review",
                    ),
                ),
                mcp.Prompt(
                    name="greeting",
                    options=mcp.PromptOptions(
                        meta=None,
                        arguments=[
                            mcp.PromptArgument(
                                name="name",
                                description="Name to greet",
                                required=False,
                                title="Name",
                            ),
                        ],
                        description="Generate a friendly greeting",
                        title="Greeting",
                    ),
                ),
            ],
            meta=None,
            next_cursor=None,
        )

    def get_prompt(
        self,
        ctx: server_messages.Context,
        request: mcp.GetPromptRequest,
        client_stream: Optional[streams.OutputStream],
    ) -> Optional[mcp.GetPromptResult]:
        if request.name == "code-review":
            # Parse arguments (would be JSON in real implementation)
            import json
            args = json.loads(request.arguments) if request.arguments else {}
            language = args.get("language", "unknown")
            code = args.get("code", "")

            return mcp.GetPromptResult(
                meta=None,
                description=f"Code review for {language}",
                messages=[
                    mcp.PromptMessage(
                        role=mcp.Role.USER,
                        content=mcp.ContentBlock(
                            text=mcp.TextContent(
                                text=mcp.TextData(
                                    text=f"Please review this {language} code for best practices, "
                                    f"potential bugs, and suggest improvements:\n\n{code}",
                                    text_stream=None,
                                ),
                                options=None,
                            ),
                            image=None,
                            embedded_resource=None,
                            resource=None,
                        ),
                    ),
                ],
            )
        elif request.name == "greeting":
            import json
            args = json.loads(request.arguments) if request.arguments else {}
            name = args.get("name", "there")

            return mcp.GetPromptResult(
                meta=None,
                description="A friendly greeting",
                messages=[
                    mcp.PromptMessage(
                        role=mcp.Role.USER,
                        content=mcp.ContentBlock(
                            text=mcp.TextContent(
                                text=mcp.TextData(
                                    text=f"Greet {name} in a friendly and welcoming way.",
                                    text_stream=None,
                                ),
                                options=None,
                            ),
                            image=None,
                            embedded_resource=None,
                            resource=None,
                        ),
                    ),
                ],
            )
        else:
            return None  # We don't handle this prompt


# Export the Prompts implementation
Prompts = ExamplePrompts
