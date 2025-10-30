"""{{project_name}} Resources Capability Provider

A resources capability that provides simple text resources.
"""

from typing import Optional
from wit_world import exports
from wit_world.imports import mcp, server_handler


class TextResources(exports.Resources):
    def list_resources(
        self,
        ctx: server_handler.RequestCtx,
        request: mcp.ListResourcesRequest,
    ) -> mcp.ListResourcesResult:
        return mcp.ListResourcesResult(
            resources=[
                mcp.McpResource(
                    uri="text://greeting",
                    name="Greeting",
                    options=mcp.ResourceOptions(
                        size=None,
                        title=None,
                        description="A friendly greeting message",
                        mime_type="text/plain",
                        annotations=None,
                        meta=None,
                    ),
                ),
                mcp.McpResource(
                    uri="text://info",
                    name="Info",
                    options=mcp.ResourceOptions(
                        size=None,
                        title=None,
                        description="Information about this resource provider",
                        mime_type="text/plain",
                        annotations=None,
                        meta=None,
                    ),
                ),
            ],
            meta=None,
            next_cursor=None,
        )

    def read_resource(
        self,
        ctx: server_handler.RequestCtx,
        request: mcp.ReadResourceRequest,
    ) -> Optional[mcp.ReadResourceResult]:
        if request.uri == "text://greeting":
            return success_result("Hello from wasmcp resources!")
        elif request.uri == "text://info":
            return success_result(
                "This is a simple resources capability component. "
                "It provides static text content via custom URIs."
            )
        else:
            return None  # We don't handle this URI

    def list_resource_templates(
        self,
        ctx: server_handler.RequestCtx,
        request: mcp.ListResourceTemplatesRequest,
    ) -> mcp.ListResourceTemplatesResult:
        # No templates for static resources
        return mcp.ListResourceTemplatesResult(
            resource_templates=[],
            meta=None,
            next_cursor=None,
        )


def success_result(text: str) -> mcp.ReadResourceResult:
    return mcp.ReadResourceResult(
        meta=None,
        contents=[mcp.ResourceContents_Text(
            mcp.TextResourceContents(
                uri="",  # URI is provided in request
                text=mcp.TextData_Text(text),
                options=mcp.EmbeddedResourceOptions(
                    mime_type="text/plain",
                    meta=None,
                ),
            )
        )],
    )


# Export the Resources implementation
Resources = TextResources
