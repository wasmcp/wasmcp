package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"

	"go.bytecodealliance.org/cm"
	outgoinghandler "weather-go/internal/wasi/http/outgoing-handler"
	"weather-go/internal/wasi/http/types"
	"weather-go/internal/wasi/io/poll"
	"weather-go/internal/wasi/io/streams"
)

func init() {
	// Replace the default HTTP transport with our WASI implementation
	// This makes standard http.Get() calls work
	http.DefaultTransport = &WASITransport{}
	if http.DefaultClient != nil {
		http.DefaultClient.Transport = &WASITransport{}
	}
}

// WASITransport implements http.RoundTripper using WASI HTTP
type WASITransport struct{}

// RoundTrip implements the http.RoundTripper interface
func (t *WASITransport) RoundTrip(req *http.Request) (*http.Response, error) {
	// Create WASI headers from http.Request headers
	headers := types.NewFields()
	for key, values := range req.Header {
		for _, value := range values {
			headers.Append(types.FieldName(strings.ToLower(key)), types.FieldValue(cm.ToList([]byte(value))))
		}
	}

	// Create outgoing request
	outReq := types.NewOutgoingRequest(headers)
	
	// Set method
	switch req.Method {
	case "GET":
		outReq.SetMethod(types.MethodGet())
	case "POST":
		outReq.SetMethod(types.MethodPost())
	case "PUT":
		outReq.SetMethod(types.MethodPut())
	case "DELETE":
		outReq.SetMethod(types.MethodDelete())
	case "PATCH":
		outReq.SetMethod(types.MethodPatch())
	case "HEAD":
		outReq.SetMethod(types.MethodHead())
	case "OPTIONS":
		outReq.SetMethod(types.MethodOptions())
	default:
		outReq.SetMethod(types.MethodOther(req.Method))
	}
	
	// Set scheme
	if req.URL.Scheme == "https" {
		outReq.SetScheme(cm.Some(types.SchemeHTTPS()))
	} else {
		outReq.SetScheme(cm.Some(types.SchemeHTTP()))
	}
	
	// Set authority (host)
	host := req.Host
	if host == "" {
		host = req.URL.Host
	}
	outReq.SetAuthority(cm.Some(host))
	
	// Set path with query
	path := req.URL.Path
	if path == "" {
		path = "/"
	}
	if req.URL.RawQuery != "" {
		path = path + "?" + req.URL.RawQuery
	}
	outReq.SetPathWithQuery(cm.Some(path))

	// Handle request body
	bodyResult := outReq.Body()
	if bodyResult.IsOK() {
		body := *bodyResult.OK()
		if req.Body != nil {
			bodyData, err := io.ReadAll(req.Body)
			req.Body.Close()
			if err != nil {
				return nil, fmt.Errorf("reading request body: %v", err)
			}
			if len(bodyData) > 0 {
				writeResult := body.Write()
				if writeResult.IsOK() {
					stream := *writeResult.OK()
					stream.BlockingWriteAndFlush(cm.ToList(bodyData))
					stream.ResourceDrop()
				}
			}
		}
		types.OutgoingBodyFinish(body, cm.None[types.Trailers]())
	}

	// Make the request
	futureResult := outgoinghandler.Handle(outReq, cm.None[types.RequestOptions]())
	if futureResult.IsErr() {
		return nil, fmt.Errorf("request failed: %v", futureResult.Err().String())
	}
	future := *futureResult.OK()
	defer future.ResourceDrop()

	// Wait for response
	pollable := future.Subscribe()
	defer pollable.ResourceDrop()
	
	// Poll until ready (like Python's send function)
	var incomingResp types.IncomingResponse
	for {
		futureGetResult := future.Get()
		if !futureGetResult.None() {
			innerResult := *futureGetResult.Some()
			if innerResult.IsErr() {
				return nil, fmt.Errorf("request failed")
			}
			
			okResult := innerResult.OK()
			if okResult.IsErr() {
				errCode := okResult.Err()
				return nil, fmt.Errorf("HTTP error: %v", errCode.String())
			}
			
			// Got the response
			incomingResp = *okResult.OK()
			break
		}
		
		// Not ready yet, block until it is
		if !pollable.Ready() {
			pollable.Block()
		}
	}
	
	// Convert WASI response to http.Response
	httpResp := &http.Response{
		Status:     fmt.Sprintf("%d", incomingResp.Status()),
		StatusCode: int(incomingResp.Status()),
		Proto:      "HTTP/1.1",
		ProtoMajor: 1,
		ProtoMinor: 1,
		Header:     make(http.Header),
		Request:    req,
	}
	
	// Convert headers
	respHeaders := incomingResp.Headers()
	entries := respHeaders.Entries()
	for _, entry := range entries.Slice() {
		key := string(entry.F0)
		value := string(entry.F1.Slice())
		httpResp.Header.Add(key, value)
	}
	// Don't drop headers - they're owned by the response
	
	// Get body  
	bodyConsumeResult := incomingResp.Consume()
	if bodyConsumeResult.IsOK() {
		incomingBody := *bodyConsumeResult.OK()
		// Create a lazy body reader that will handle cleanup
		httpResp.Body = &wasiBodyReader{
			body:     incomingBody,
			response: incomingResp,
		}
	} else {
		httpResp.Body = io.NopCloser(bytes.NewReader(nil))
		// Only drop response if we didn't consume the body
		incomingResp.ResourceDrop()
	}
	
	return httpResp, nil
}

// wasiBodyReader implements io.ReadCloser for WASI incoming body
// It holds the response and body resources and cleans them up on Close
type wasiBodyReader struct {
	body     types.IncomingBody
	response types.IncomingResponse
	stream   streams.InputStream
	finished bool
}

func (r *wasiBodyReader) Read(p []byte) (int, error) {
	if r.finished {
		return 0, io.EOF
	}
	
	// Lazily get stream on first read
	if r.stream == 0 {
		streamResult := r.body.Stream()
		if streamResult.IsOK() {
			r.stream = *streamResult.OK()
		} else {
			return 0, fmt.Errorf("failed to get stream")
		}
	}
	
	// Block until data is ready
	poll := r.stream.Subscribe()
	poll.Block()
	poll.ResourceDrop()
	
	// Read data
	readResult := r.stream.Read(uint64(len(p)))
	if readResult.IsErr() {
		// Check if stream is closed
		err := readResult.Err()
		if err.Closed() {
			r.finished = true
			r.finish()
			return 0, io.EOF
		}
		return 0, fmt.Errorf("stream read error")
	}
	
	data := *readResult.OK()
	if len(data.Slice()) == 0 {
		r.finished = true
		r.finish()
		return 0, io.EOF
	}
	
	copy(p, data.Slice())
	return len(data.Slice()), nil
}

func (r *wasiBodyReader) Close() error {
	return r.finish()
}

func (r *wasiBodyReader) finish() error {
	if r.stream != 0 {
		r.stream.ResourceDrop()
		r.stream = 0
	}
	
	// Finish the body and wait for completion
	future := types.IncomingBodyFinish(r.body)
	defer future.ResourceDrop()
	
	p := future.Subscribe()
	p.Block()
	p.ResourceDrop()
	
	// Wait for trailers to be ready
	future.Get()
	
	// Now we can safely drop the response
	r.response.ResourceDrop()
	
	return nil
}


// HTTPResult holds the result of an HTTP request
type HTTPResult struct {
	Index    int
	Response *http.Response
	Body     []byte
	Error    error
}

// DoGetConcurrently performs multiple GET requests concurrently using WASI HTTP
func DoGetConcurrently(urls []string) []HTTPResult {
	if len(urls) == 0 {
		return nil
	}

	results := make([]HTTPResult, len(urls))
	futures := make([]types.FutureIncomingResponse, len(urls))
	requests := make([]types.OutgoingRequest, len(urls))
	pollables := make([]poll.Pollable, 0, len(urls))
	pollableToIndex := make(map[uint32]int)

	// Start all requests
	for i, urlStr := range urls {
		results[i].Index = i
		
		// Parse URL
		parsedURL, err := url.Parse(urlStr)
		if err != nil {
			results[i].Error = fmt.Errorf("parsing URL: %v", err)
			continue
		}

		// Create headers
		headers := types.NewFields()
		headers.Set("user-agent", cm.ToList([]types.FieldValue{
			types.FieldValue(cm.ToList([]byte("Go-MCP/1.0"))),
		}))
		headers.Set("accept", cm.ToList([]types.FieldValue{
			types.FieldValue(cm.ToList([]byte("application/json"))),
		}))

		// Create outgoing request (takes ownership of headers)
		req := types.NewOutgoingRequest(headers)
		requests[i] = req
		req.SetMethod(types.MethodGet())
		
		if parsedURL.Scheme == "https" {
			req.SetScheme(cm.Some(types.SchemeHTTPS()))
		} else {
			req.SetScheme(cm.Some(types.SchemeHTTP()))
		}
		
		req.SetAuthority(cm.Some(parsedURL.Host))
		
		path := parsedURL.Path
		if path == "" {
			path = "/"
		}
		if parsedURL.RawQuery != "" {
			path = path + "?" + parsedURL.RawQuery
		}
		req.SetPathWithQuery(cm.Some(path))

		// Get the body (even for GET, we need to finish it)
		bodyResult := req.Body()
		if bodyResult.IsOK() {
			body := *bodyResult.OK()
			types.OutgoingBodyFinish(body, cm.None[types.Trailers]())
		}

		// Start the request
		futureResult := outgoinghandler.Handle(req, cm.None[types.RequestOptions]())
		if futureResult.IsErr() {
			results[i].Error = fmt.Errorf("starting request: %v", futureResult.Err().String())
			continue
		}

		future := *futureResult.OK()
		futures[i] = future
		pollable := future.Subscribe()
		pollables = append(pollables, pollable)
		pollableToIndex[uint32(pollable)] = i
	}

	if len(pollables) == 0 {
		return results // All requests failed to start
	}

	// Poll all requests concurrently
	remaining := len(pollables)
	for remaining > 0 {
		// Wait for any request to complete
		readyIndices := poll.Poll(cm.ToList(pollables))
		
		// Process completed requests - indices are into the pollables array
		for _, pollIdx := range readyIndices.Slice() {
			if pollIdx >= uint32(len(pollables)) {
				continue
			}
			
			pollable := pollables[pollIdx]
			reqIdx := pollableToIndex[uint32(pollable)]
			
			if futures[reqIdx] == 0 {
				continue // Already processed
			}

			// Get the response
			future := futures[reqIdx]
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
						// Process successful response
						incomingResp := *okResult.OK()
						
						// Store status code first
						results[reqIdx].Response = &http.Response{
							StatusCode: int(incomingResp.Status()),
							Header:     make(http.Header),
						}
						
						// Read the body immediately
						bodyResult := incomingResp.Consume()
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
								results[reqIdx].Body = bodyBytes
								
								// Drop stream AFTER reading all data
								stream.ResourceDrop()
							}
							
							// Finish the incoming body and wait for completion
							futureTrailers := types.IncomingBodyFinish(incomingBody)
							trailerPoll := futureTrailers.Subscribe()
							
							// Keep polling until ready
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
							// IncomingBody is consumed by IncomingBodyFinish, don't drop it
						}
						
						// Extract headers after body is finished
						headers := incomingResp.Headers()
						entries := headers.Entries()
						for _, entry := range entries.Slice() {
							key := string(entry.F0)
							value := string(entry.F1.Slice())
							results[reqIdx].Response.Header.Add(key, value)
						}
						
						// Don't drop headers - they're owned by response
						// Don't drop response - let GC handle it like wasi-http-go
					}
				}
			}

			// Clean up pollable first (it's a child of future)
			pollable.ResourceDrop()
			// Then drop the future
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
	
	// Don't drop outgoing requests - they were consumed by Handle()
	
	return results
}

// convertWASIResponse converts a WASI response to http.Response
func convertWASIResponse(resp types.IncomingResponse) *http.Response {
	httpResp := &http.Response{
		StatusCode: int(resp.Status()),
		Header:     make(http.Header),
	}

	// Convert headers
	headers := resp.Headers()
	entries := headers.Entries()
	for _, entry := range entries.Slice() {
		key := string(entry.F0)
		value := string(entry.F1.Slice())
		httpResp.Header.Add(key, value)
	}
	// Don't drop headers - they're owned by the response

	return httpResp
}

// FetchMultiWeatherConcurrent fetches weather for multiple cities concurrently
func FetchMultiWeatherConcurrent(cities []string) []weatherResult {
	results := make([]weatherResult, len(cities))
	
	// Build URLs for geocoding
	geoURLs := make([]string, len(cities))
	for i, city := range cities {
		geoURLs[i] = fmt.Sprintf("https://geocoding-api.open-meteo.com/v1/search?name=%s&count=1", 
			url.QueryEscape(city))
	}
	
	// Fetch all geocoding data concurrently
	geoResults := DoGetConcurrently(geoURLs)
	
	// Process geocoding results and prepare weather URLs
	weatherURLs := make([]string, 0, len(cities))
	cityIndices := make([]int, 0, len(cities))
	cityData := make(map[int]*WeatherData)
	
	for i, geoRes := range geoResults {
		if geoRes.Error != nil {
			results[i] = weatherResult{err: fmt.Errorf("geocoding failed: %v", geoRes.Error)}
			continue
		}
		
		var geoData GeocodingResult
		if err := json.Unmarshal(geoRes.Body, &geoData); err != nil {
			results[i] = weatherResult{err: fmt.Errorf("parsing geocoding: %v", err)}
			continue
		}
		
		if len(geoData.Results) == 0 {
			results[i] = weatherResult{err: fmt.Errorf("location '%s' not found", cities[i])}
			continue
		}
		
		location := geoData.Results[0]
		weatherURL := fmt.Sprintf(
			"https://api.open-meteo.com/v1/forecast?latitude=%f&longitude=%f&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code",
			location.Latitude, location.Longitude,
		)
		weatherURLs = append(weatherURLs, weatherURL)
		cityIndices = append(cityIndices, i)
		
		// Store location info
		cityData[i] = &WeatherData{
			Name:    location.Name,
			Country: location.Country,
		}
	}
	
	if len(weatherURLs) == 0 {
		return results // All geocoding failed
	}
	
	// Fetch all weather data concurrently
	weatherResults := DoGetConcurrently(weatherURLs)
	
	// Process weather results
	for j, weatherRes := range weatherResults {
		i := cityIndices[j]
		
		if weatherRes.Error != nil {
			results[i] = weatherResult{err: fmt.Errorf("weather request failed: %v", weatherRes.Error)}
			continue
		}
		
		var weather WeatherResponse
		if err := json.Unmarshal(weatherRes.Body, &weather); err != nil {
			results[i] = weatherResult{err: fmt.Errorf("parsing weather: %v", err)}
			continue
		}
		
		// Combine location and weather data
		data := cityData[i]
		data.Temperature = weather.Current.Temperature2m
		data.Humidity = weather.Current.RelativeHumidity2m
		data.WindSpeed = weather.Current.WindSpeed10m
		data.WeatherCode = weather.Current.WeatherCode
		
		results[i] = weatherResult{data: data}
	}
	
	return results
}