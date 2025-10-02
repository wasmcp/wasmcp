//go:build !setup

package main

import (
	"fmt"

	"github.com/go_logger/gen/wasi/io/v0.2.3/streams"
	incominghandler "github.com/go_logger/gen/wasmcp/mcp/v0.3.0-alpha.59/incoming-handler"
	"github.com/go_logger/gen/wasmcp/mcp/v0.3.0-alpha.59/request"
)

func init() {
	incominghandler.Exports.Handle = Handle
}

func Handle(req request.Request, output streams.OutputStream) {
	// Log the request
	feature := req.Feature()
	id := req.ID()
	fmt.Printf("[Middleware] Request: feature=%v, id=%v\n", feature, id)

	// Forward to next handler in the chain
	incominghandler.Handle(req, output)
}

func main() {}
