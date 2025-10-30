"""{{project_name}} Prompts Capability Provider

A prompts capability that provides example prompt templates.
"""

import json
from typing import Optional
from wit_world import exports
from wit_world.imports import mcp, server_handler


class ExamplePrompts(exports.Prompts):
    def list_prompts(
        self,
        ctx: server_handler.RequestCtx,
        request: mcp.ListPromptsRequest,
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
        ctx: server_handler.RequestCtx,
        request: mcp.GetPromptRequest,
    ) -> Optional[mcp.GetPromptResult]:
        if request.name == "code-review":
            # Parse arguments (would be JSON in real implementation)
            args = json.loads(request.arguments) if request.arguments else {}
            language = args.get("language", "unknown")
            code = args.get("code", "")

            return mcp.GetPromptResult(
                meta=None,
                description=f"Code review for {language}",
                messages=[
                    mcp.PromptMessage(
                        role=mcp.Role.USER,
                        content=mcp.ContentBlock_Text(
                            mcp.TextContent(
                                text=mcp.TextData_Text(
                                    f"Please review this {language} code for best practices, "
                                    f"potential bugs, and suggest improvements:\n\n{code}"
                                ),
                                options=None,
                            )
                        ),
                    ),
                ],
            )
        elif request.name == "greeting":
            args = json.loads(request.arguments) if request.arguments else {}
            name = args.get("name", "there")

            return mcp.GetPromptResult(
                meta=None,
                description="A friendly greeting",
                messages=[
                    mcp.PromptMessage(
                        role=mcp.Role.USER,
                        content=mcp.ContentBlock_Text(
                            mcp.TextContent(
                                text=mcp.TextData_Text(
                                    f"Greet {name} in a friendly and welcoming way."
                                ),
                                options=None,
                            )
                        ),
                    ),
                ],
            )
        else:
            return None  # We don't handle this prompt


# Export the Prompts implementation
Prompts = ExamplePrompts
