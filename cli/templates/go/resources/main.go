//go:build !setup

package main

import (
	incominghandler "github.com/{{ package_name }}/gen/wasmcp/mcp/{{ generated_version_path }}/incoming-handler"
	"github.com/{{ package_name }}/gen/wasmcp/mcp/{{ generated_version_path }}/request"
	"github.com/{{ package_name }}/gen/wasmcp/mcp/{{ generated_version_path }}/types"
	resourceslistresult "github.com/{{ package_name }}/gen/wasmcp/mcp/{{ generated_version_path }}/resources-list-result"
	resourcesreadresult "github.com/{{ package_name }}/gen/wasmcp/mcp/{{ generated_version_path }}/resources-read-result"
	errorresult "github.com/{{ package_name }}/gen/wasmcp/mcp/{{ generated_version_path }}/error-result"
	"github.com/{{ package_name }}/gen/wasi/io/v0.2.3/streams"
	"go.bytecodealliance.org/cm"
)

func init() {
	incominghandler.Exports.Handle = Handle
}

func Handle(req request.Request, output streams.OutputStream) {
	if !req.Needs(types.ServerCapabilitiesResources) {
		incominghandler.Handle(req, output)
		return
	}

	id := req.ID()
	params := req.Params()

	if !params.IsOK() {
		_ = errorresult.Write(id, output, *params.Err())
		return
	}

	p := params.OK()
	if resourcesListParams := p.ResourcesList(); resourcesListParams != nil {
		handleResourcesList(id, output)
	} else if uri := p.ResourcesRead(); uri != nil {
		handleResourcesRead(id, string(*uri), output)
	}
}

func handleResourcesList(id types.ID, output streams.OutputStream) {
	resources := []resourceslistresult.Resource{
		{
			URI:  "file:///example.txt",
			Name: "example.txt",
			Options: cm.Some(resourceslistresult.ResourceOptions{
				Size:        cm.None[uint64](),
				Title:       cm.None[string](),
				Description: cm.Some[string]("An example text resource"),
				MIMEType:    cm.Some[string]("text/plain"),
				Annotations: cm.None[resourceslistresult.Annotations](),
				Meta:        types.Meta{},
			}),
		},
	}

	_ = resourceslistresult.Write(id, output, cm.ToList(resources), cm.None[resourceslistresult.Options]())
}

func handleResourcesRead(id types.ID, uri string, output streams.OutputStream) {
	var content []byte
	if uri == "file:///example.txt" {
		content = []byte(readExample())
	} else {
		content = []byte("Unknown resource: " + uri)
	}

	resourceContents := resourcesreadresult.Contents{
		URI:     uri,
		Data:    cm.ToList(content),
		Options: cm.None[resourcesreadresult.ContentsOptions](),
	}

	_ = resourcesreadresult.Write(id, output, resourceContents, cm.None[resourcesreadresult.Options]())
}

func readExample() string {
	return "This is the content of example.txt"
}

func main() {}
