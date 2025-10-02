//go:build !setup

package main

import (
	"fmt"

	"github.com/{{ package_name }}/gen/wasi/io/v0.2.3/streams"
	incominghandler "github.com/{{ package_name }}/gen/wasmcp/mcp/{{ generated_version_path }}/incoming-handler"
	"github.com/{{ package_name }}/gen/wasmcp/mcp/{{ generated_version_path }}/request"
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
