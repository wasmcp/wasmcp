"""{{ handler_type_capitalized }} handler for MCP."""

import sys
from wit_world import exports
from wit_world.imports import request, streams, incoming_handler as next_handler


class IncomingHandler(exports.IncomingHandler):
    """Implementation of the MCP incoming handler interface."""

    def handle(self, req: request.Request, output: streams.OutputStream) -> None:
        """Handle an incoming MCP request."""
        # Log the request
        feature = req.feature()
        req_id = req.id()
        print(f"[Middleware] Request: feature={feature}, id={req_id}", file=sys.stderr)

        # Forward to next handler in the chain
        next_handler.handle(req, output)
