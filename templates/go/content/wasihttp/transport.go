package wasihttp

import (
	"bytes"
	"fmt"
	"io"
	"net/http"

	"go.bytecodealliance.org/cm"
	outgoinghandler "{{project-name | snake_case}}/internal/wasi/http/outgoing-handler"
	"{{project-name | snake_case}}/internal/wasi/http/types"
)

// Transport implements http.RoundTripper using WASI HTTP bindings
// This allows it to be used as a drop-in replacement for http.DefaultTransport
type Transport struct{}

// RoundTrip implements the http.RoundTripper interface
func (t *Transport) RoundTrip(req *http.Request) (*http.Response, error) {
	// Convert to WASI request
	wasiReq, err := toWASIRequest(req)
	if err != nil {
		return nil, err
	}

	// Send request body if present
	if req.Body != nil {
		defer req.Body.Close()
		if err := sendRequestBody(wasiReq, req.Body); err != nil {
			return nil, err
		}
	}

	// Make the request
	futureResult := outgoinghandler.Handle(wasiReq, cm.None[types.RequestOptions]())
	if futureResult.IsErr() {
		return nil, fmt.Errorf("request failed: %v", futureResult.Err().String())
	}

	future := *futureResult.OK()
	defer future.ResourceDrop()

	// Wait for response
	pollable := future.Subscribe()
	pollable.Block()
	pollable.ResourceDrop()

	// Get the response
	result := future.Get()
	if result.None() {
		return nil, fmt.Errorf("no response received")
	}

	innerResult := *result.Some()
	if innerResult.IsErr() {
		return nil, fmt.Errorf("request failed")
	}

	okResult := innerResult.OK()
	if okResult.IsErr() {
		return nil, fmt.Errorf("HTTP error: %v", okResult.Err().String())
	}

	// Convert WASI response to http.Response, reading the body immediately
	wasiResp := *okResult.OK()
	httpResp, body, err := fromWASIResponse(wasiResp)
	if err != nil {
		return nil, err
	}

	// Wrap the body in a reader
	if body != nil {
		httpResp.Body = io.NopCloser(bytes.NewReader(body))
	} else {
		httpResp.Body = io.NopCloser(bytes.NewReader(nil))
	}

	return httpResp, nil
}

