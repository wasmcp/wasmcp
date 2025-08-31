package wasihttp

import (
	"fmt"
	"io"
	"net/http"
	"strings"

	"go.bytecodealliance.org/cm"
	"weather_go/internal/wasi/http/types"
)

// toWASIRequest converts an http.Request to a WASI OutgoingRequest
func toWASIRequest(req *http.Request) (types.OutgoingRequest, error) {
	// Create headers
	headers := types.NewFields()
	for key, values := range req.Header {
		for _, value := range values {
			headers.Set(types.FieldKey(strings.ToLower(key)), cm.ToList([]types.FieldValue{
				types.FieldValue(cm.ToList([]byte(value))),
			}))
		}
	}

	// Create outgoing request (takes ownership of headers)
	wasiReq := types.NewOutgoingRequest(headers)
	
	// Set method
	wasiReq.SetMethod(toMethod(req.Method))
	
	// Set scheme
	scheme := "http"
	if req.URL != nil && req.URL.Scheme != "" {
		scheme = req.URL.Scheme
	}
	wasiReq.SetScheme(cm.Some(toScheme(scheme)))
	
	// Set authority (host)
	host := req.Host
	if host == "" && req.URL != nil {
		host = req.URL.Host
	}
	if host != "" {
		wasiReq.SetAuthority(cm.Some(host))
	}
	
	// Set path with query
	path := "/"
	if req.URL != nil {
		if req.URL.Path != "" {
			path = req.URL.Path
		}
		if req.URL.RawQuery != "" {
			path += "?" + req.URL.RawQuery
		}
	}
	wasiReq.SetPathWithQuery(cm.Some(path))
	
	return wasiReq, nil
}

// sendRequestBody sends the request body to the WASI outgoing request
func sendRequestBody(wasiReq types.OutgoingRequest, body io.Reader) error {
	bodyResult := wasiReq.Body()
	if bodyResult.IsErr() {
		return fmt.Errorf("failed to get request body")
	}
	
	outgoingBody := *bodyResult.OK()
	defer types.OutgoingBodyFinish(outgoingBody, cm.None[types.Trailers]())
	
	streamResult := outgoingBody.Write()
	if streamResult.IsErr() {
		return fmt.Errorf("failed to get body stream")
	}
	
	stream := *streamResult.OK()
	defer stream.ResourceDrop()
	
	// Write body data
	buf := make([]byte, 64*1024)
	for {
		n, err := body.Read(buf)
		if n > 0 {
			writeResult := stream.BlockingWriteAndFlush(cm.ToList(buf[:n]))
			if writeResult.IsErr() {
				if !writeResult.Err().Closed() {
					return fmt.Errorf("failed to write body: %v", writeResult.Err())
				}
			}
		}
		if err == io.EOF {
			break
		}
		if err != nil {
			return err
		}
	}
	
	stream.Flush()
	return nil
}

// fromWASIResponse converts a WASI IncomingResponse to an http.Response
// It also reads and returns the body if immediate=true
func fromWASIResponse(wasiResp types.IncomingResponse) (*http.Response, []byte, error) {
	httpResp := &http.Response{
		StatusCode: int(wasiResp.Status()),
		Header:     make(http.Header),
	}
	
	// Convert headers
	headers := wasiResp.Headers()
	entries := headers.Entries()
	for _, entry := range entries.Slice() {
		key := string(entry.F0)
		value := string(entry.F1.Slice())
		httpResp.Header.Add(key, value)
	}
	
	// For concurrent requests, read body immediately
	bodyResult := wasiResp.Consume()
	if bodyResult.IsOK() {
		incomingBody := *bodyResult.OK()
		streamResult := incomingBody.Stream()
		if streamResult.IsOK() {
			stream := *streamResult.OK()
			
			var bodyBytes []byte
			for {
				readResult := stream.Read(64 * 1024)
				if readResult.IsErr() {
					break
				}
				data := *readResult.OK()
				if len(data.Slice()) == 0 {
					break
				}
				bodyBytes = append(bodyBytes, data.Slice()...)
			}
			
			// Drop stream after reading
			stream.ResourceDrop()
			
			// Finish the incoming body
			futureTrailers := types.IncomingBodyFinish(incomingBody)
			trailerPoll := futureTrailers.Subscribe()
			
			// Poll until ready
			for {
				trailersResult := futureTrailers.Get()
				if !trailersResult.None() {
					break
				}
				if !trailerPoll.Ready() {
					trailerPoll.Block()
				}
			}
			
			trailerPoll.ResourceDrop()
			futureTrailers.ResourceDrop()
			
			return httpResp, bodyBytes, nil
		}
	}
	
	return httpResp, nil, nil
}

// toMethod converts an HTTP method string to WASI Method type
func toMethod(method string) types.Method {
	switch strings.ToUpper(method) {
	case "GET":
		return types.MethodGet()
	case "HEAD":
		return types.MethodHead()
	case "POST":
		return types.MethodPost()
	case "PUT":
		return types.MethodPut()
	case "PATCH":
		return types.MethodPatch()
	case "DELETE":
		return types.MethodDelete()
	case "CONNECT":
		return types.MethodConnect()
	case "OPTIONS":
		return types.MethodOptions()
	case "TRACE":
		return types.MethodTrace()
	default:
		return types.MethodOther(method)
	}
}

// toScheme converts a scheme string to WASI Scheme type
func toScheme(scheme string) types.Scheme {
	switch strings.ToLower(scheme) {
	case "http":
		return types.SchemeHTTP()
	case "https":
		return types.SchemeHTTPS()
	default:
		return types.SchemeOther(scheme)
	}
}