"""Resources handler for MCP."""

from typing import Optional

from wit_world import exports
from wit_world.imports import (
    request,
    streams,
    incoming_handler as next_handler,
    resources_list_result,
    resources_read_result,
    error_result,
    types,
)


class IncomingHandler(exports.IncomingHandler):
    """Implementation of the MCP incoming handler interface."""

    def handle(self, req: request.Request, output: streams.OutputStream) -> None:
        """Handle an incoming MCP request."""
        if not req.needs(types.ServerCapabilities.RESOURCES):
            next_handler.handle(req, output)
            return

        req_id = req.id()
        try:
            params = req.params()
            if isinstance(params, request.Params_ResourcesList):
                self.handle_resources_list(req_id, output)
            elif isinstance(params, request.Params_ResourcesRead):
                uri = params.value
                self.handle_resources_read(req_id, uri, output)
        except request.McpError as error:
            error_result.write(req_id, output, error)

    def handle_resources_list(
        self,
        req_id: types.Id,
        output: streams.OutputStream,
    ) -> None:
        """Handle resources/list request."""
        resources = [
            resources_list_result.Resource(
                uri="file:///example.txt",
                name="example.txt",
                options=resources_list_result.ResourceOptions(
                    size=None,
                    title=None,
                    description="An example text resource",
                    mime_type="text/plain",
                    annotations=None,
                    meta=None,
                ),
            ),
        ]

        resources_list_result.write(req_id, output, resources, None)

    def handle_resources_read(
        self,
        req_id: types.Id,
        uri: str,
        output: streams.OutputStream,
    ) -> None:
        """Handle resources/read request."""
        try:
            if uri == "file:///example.txt":
                text_content = self.read_example()
                resource_contents = resources_read_result.Contents(
                    uri=uri,
                    data=text_content.encode('utf-8'),
                    options=None,
                )
                resources_read_result.write(
                    req_id, output, resource_contents, None
                )
            else:
                # Send error - need to create error response
                error_text = f"Unknown resource: {uri}"
                error_contents = resources_read_result.Contents(
                    uri=uri,
                    data=error_text.encode('utf-8'),
                    options=None,
                )
                resources_read_result.write(
                    req_id, output, error_contents, None
                )
        except Exception as e:
            error_text = f"Resource read failed: {str(e)}"
            error_contents = resources_read_result.Contents(
                uri=uri,
                data=error_text.encode('utf-8'),
                options=None,
            )
            resources_read_result.write(
                req_id, output, error_contents, None
            )

    def read_example(self) -> str:
        """Read the example resource."""
        return "This is the content of example.txt"
