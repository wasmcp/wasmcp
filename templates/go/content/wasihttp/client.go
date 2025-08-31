// Package wasihttp provides HTTP client implementations for WebAssembly System Interface (WASI)
// It includes both standard http.RoundTripper implementation and concurrent request capabilities
package wasihttp

import (
	"bytes"
	"fmt"
	"io"
	"net/http"

	"go.bytecodealliance.org/cm"
	outgoinghandler "{{project-name | snake_case}}/internal/wasi/http/outgoing-handler"
	"{{project-name | snake_case}}/internal/wasi/http/types"
	"{{project-name | snake_case}}/internal/wasi/io/poll"
)

// Client provides HTTP client capabilities using WASI HTTP bindings
type Client struct {
	// MaxConcurrency limits the number of concurrent requests (0 = unlimited)
	MaxConcurrency int
}

// DefaultClient is the default WASI HTTP client
var DefaultClient = &Client{}

// init replaces the standard library's HTTP transport with WASI implementation
func init() {
	transport := &Transport{}
	http.DefaultTransport = transport
	if http.DefaultClient != nil {
		http.DefaultClient.Transport = transport
	}
}

// Get performs a GET request (convenience method)
func (c *Client) Get(url string) (*http.Response, error) {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, err
	}
	return c.Do(req)
}

// Do performs a single HTTP request
func (c *Client) Do(req *http.Request) (*http.Response, error) {
	transport := &Transport{}
	return transport.RoundTrip(req)
}

// GetConcurrently fetches multiple URLs concurrently
func (c *Client) GetConcurrently(urls []string) []*Response {
	requests := make([]*http.Request, len(urls))
	for i, u := range urls {
		req, err := http.NewRequest("GET", u, nil)
		if err != nil {
			requests[i] = nil
		} else {
			requests[i] = req
		}
	}
	return c.DoConcurrently(requests)
}

// Response wraps an HTTP response with error information
type Response struct {
	*http.Response
	Error error
	Index int // Original request index for ordering
}

// DoConcurrently performs multiple HTTP requests concurrently
func (c *Client) DoConcurrently(requests []*http.Request) []*Response {
	if len(requests) == 0 {
		return nil
	}

	results := make([]*Response, len(requests))
	futures := make([]types.FutureIncomingResponse, len(requests))
	wasiRequests := make([]types.OutgoingRequest, len(requests))
	pollables := make([]poll.Pollable, 0, len(requests))
	pollableToIndex := make(map[uint32]int)

	// Start all requests
	for i, req := range requests {
		results[i] = &Response{Index: i}
		
		if req == nil {
			results[i].Error = fmt.Errorf("nil request")
			continue
		}

		// Convert to WASI request
		wasiReq, err := toWASIRequest(req)
		if err != nil {
			results[i].Error = err
			continue
		}
		wasiRequests[i] = wasiReq

		// Send request body if present
		if req.Body != nil {
			if err := sendRequestBody(wasiReq, req.Body); err != nil {
				results[i].Error = err
				continue
			}
		}

		// Start the request
		futureResult := outgoinghandler.Handle(wasiReq, cm.None[types.RequestOptions]())
		if futureResult.IsErr() {
			results[i].Error = fmt.Errorf("failed to start request")
			continue
		}

		future := *futureResult.OK()
		futures[i] = future
		
		pollable := future.Subscribe()
		pollables = append(pollables, pollable)
		pollableToIndex[uint32(pollable)] = i
	}

	// Poll for responses
	remaining := len(pollables)
	for remaining > 0 {
		// Wait for at least one request to complete
		readyIndices := poll.Poll(cm.ToList(pollables))
		
		// Process completed requests
		for _, readyIdx := range readyIndices.Slice() {
			pollable := pollables[readyIdx]
			reqIdx := pollableToIndex[uint32(pollable)]
			future := futures[reqIdx]

			// Get the response
			futureResult := future.Get()
			if futureResult.None() {
				results[reqIdx].Error = fmt.Errorf("no response after polling")
			} else {
				innerResult := *futureResult.Some()
				if innerResult.IsErr() {
					results[reqIdx].Error = fmt.Errorf("request failed")
				} else {
					okResult := innerResult.OK()
					if okResult.IsErr() {
						errCode := okResult.Err()
						results[reqIdx].Error = fmt.Errorf("HTTP error: %v", errCode.String())
					} else {
						// Convert WASI response to http.Response
						wasiResp := *okResult.OK()
						httpResp, body, err := fromWASIResponse(wasiResp)
						if err != nil {
							results[reqIdx].Error = err
						} else {
							results[reqIdx].Response = httpResp
							// If there's a body, wrap it in a reader
							if body != nil {
								results[reqIdx].Response.Body = io.NopCloser(bytes.NewReader(body))
							}
						}
					}
				}
			}

			// Clean up
			pollable.ResourceDrop()
			future.ResourceDrop()
			futures[reqIdx] = 0
			remaining--
		}

		// Rebuild pollables list without completed ones
		if remaining > 0 {
			newPollables := make([]poll.Pollable, 0, remaining)
			newPollableToIndex := make(map[uint32]int)
			for _, p := range pollables {
				reqIdx := pollableToIndex[uint32(p)]
				if futures[reqIdx] != 0 {
					newPollables = append(newPollables, p)
					newPollableToIndex[uint32(p)] = reqIdx
				}
			}
			pollables = newPollables
			pollableToIndex = newPollableToIndex
		}
	}

	return results
}

// Get performs a GET request using the default client
func Get(url string) (*http.Response, error) {
	return DefaultClient.Get(url)
}

// GetConcurrently fetches multiple URLs concurrently using the default client
func GetConcurrently(urls []string) []*Response {
	return DefaultClient.GetConcurrently(urls)
}