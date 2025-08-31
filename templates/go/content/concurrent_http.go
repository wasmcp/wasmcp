package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"

	"go.bytecodealliance.org/cm"
	outgoinghandler "{{project-name | snake_case}}/internal/wasi/http/outgoing-handler"
	"{{project-name | snake_case}}/internal/wasi/http/types"
	"{{project-name | snake_case}}/internal/wasi/io/poll"
)

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
		headers.Set("user-agent", cm.ToList([]byte("Go-MCP/1.0")))
		headers.Set("accept", cm.ToList([]byte("application/json")))

		// Create outgoing request
		req := types.NewOutgoingRequest(headers)
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
		req.SetPathWithQuery(path)

		// Get the body (even for GET, we need to finish it)
		body, _, _ := req.Body().Result()
		types.OutgoingBody(body).Finish(cm.None[types.Trailers]())

		// Start the request
		future, errCode, isErr := outgoinghandler.Handle(req, cm.None[types.RequestOptions]()).Result()
		if isErr {
			results[i].Error = fmt.Errorf("starting request: %v", errCode.String())
			continue
		}

		futures[i] = future
		pollable := future.Subscribe()
		pollables = append(pollables, poll.Pollable(pollable))
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
		
		// Process completed requests
		for _, idx := range readyIndices.Slice() {
			pollableIdx := uint32(idx)
			if pollableIdx >= uint32(len(pollables)) {
				continue
			}
			
			pollable := pollables[pollableIdx]
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
				innerResult := futureResult.Some()
				if innerResult.IsErr() {
					results[reqIdx].Error = fmt.Errorf("request failed")
				} else {
					okResult := innerResult.OK()
					if okResult.IsErr() {
						errCode := okResult.Err()
						results[reqIdx].Error = fmt.Errorf("HTTP error: %v", errCode.String())
					} else {
						// Process successful response
						incomingResp := okResult.OK()
						results[reqIdx].Response = convertWASIResponse(incomingResp)
						
						// Read the body immediately
						bodyResult := incomingResp.Consume()
						if bodyResult.Some() != nil {
							incomingBody := *bodyResult.Some()
							stream := incomingBody.Stream()
							
							var bodyBytes []byte
							for {
								data, _, _ := stream.Read(64 * 1024).Result()
								if len(data.Slice()) == 0 {
									break
								}
								bodyBytes = append(bodyBytes, data.Slice()...)
							}
							results[reqIdx].Body = bodyBytes
							stream.ResourceDrop()
						}
					}
				}
			}

			// Clean up
			future.ResourceDrop()
			pollable.ResourceDrop()
			futures[reqIdx] = 0
			remaining--
		}

		// Rebuild pollables list without completed ones
		if remaining > 0 {
			newPollables := make([]poll.Pollable, 0, remaining)
			newPollableToIndex := make(map[uint32]int)
			for i, p := range pollables {
				reqIdx := pollableToIndex[uint32(pollables[i])]
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
		value := string(entry.F1)
		httpResp.Header.Add(key, value)
	}

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