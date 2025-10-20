"""{{project_name}} Resources Capability Provider

A resources capability that provides simple text resources.
"""

from typing import Optional
from wit_world import exports
from wit_world.imports import mcp, server_messages, streams


class TextResources(exports.Resources):
    def list_resources(
        self,
        ctx: server_messages.Context,
        request: mcp.ListResourcesRequest,
        client_stream: Optional[streams.OutputStream],
    ) -> mcp.ListResourcesResult:
        return mcp.ListResourcesResult(
            resources=[
                mcp.Resource(
                    uri="text://greeting",
                    name="Greeting",
                    mime_type="text/plain",
                    options=mcp.ResourceOptions(
                        annotations=None,
                        description="A friendly greeting message",
                        meta=None,
                    ),
                ),
                mcp.Resource(
                    uri="text://info",
                    name="Info",
                    mime_type="text/plain",
                    options=mcp.ResourceOptions(
                        annotations=None,
                        description="Information about this resource provider",
                        meta=None,
                    ),
                ),
            ],
            meta=None,
            next_cursor=None,
        )

    def read_resource(
        self,
        ctx: server_messages.Context,
        request: mcp.ReadResourceRequest,
        client_stream: Optional[streams.OutputStream],
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
        ctx: server_messages.Context,
        request: mcp.ListResourceTemplatesRequest,
        client_stream: Optional[streams.OutputStream],
    ) -> mcp.ListResourceTemplatesResult:
        # No templates for static resources
        return mcp.ListResourceTemplatesResult(
            resource_templates=[],
            meta=None,
            next_cursor=None,
        )


def success_result(text: str) -> mcp.ReadResourceResult:
    return mcp.ReadResourceResult(
        contents=[mcp.ResourceContents(
            uri="",  # URI is provided in request
            mime_type="text/plain",
            text=text,
            options=None,
            blob=None,
        )],
        meta=None,
    )


# Export the Resources implementation
Resources = TextResources
