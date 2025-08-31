package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"sync"

	"go.bytecodealliance.org/cm"
	"{{project-name | snake_case}}/internal/fastertools/mcp/tools"
	toolscapabilities "{{project-name | snake_case}}/internal/fastertools/mcp/tools-capabilities"
	"{{project-name | snake_case}}/internal/fastertools/mcp/types"

	// Enable WASI HTTP support
	_ "github.com/ydnar/wasi-http-go/wasihttp"
)

// WeatherData represents the weather response structure
type WeatherData struct {
	Name        string  `json:"name"`
	Country     string  `json:"country"`
	Temperature float64 `json:"temperature"`
	Humidity    int     `json:"humidity"`
	WindSpeed   float64 `json:"wind_speed"`
	WeatherCode int     `json:"weather_code"`
}

// GeocodingResult represents a geocoding API result
type GeocodingResult struct {
	Results []struct {
		Name      string  `json:"name"`
		Country   string  `json:"country"`
		Latitude  float64 `json:"latitude"`
		Longitude float64 `json:"longitude"`
	} `json:"results"`
}

// WeatherResponse represents the weather API response
type WeatherResponse struct {
	Current struct {
		Temperature2m       float64 `json:"temperature_2m"`
		RelativeHumidity2m  int     `json:"relative_humidity_2m"`
		WindSpeed10m        float64 `json:"wind_speed_10m"`
		WeatherCode         int     `json:"weather_code"`
	} `json:"current"`
}

func init() {
	// Register our MCP tools capabilities
	toolscapabilities.Exports.HandleListTools = handleListTools
	toolscapabilities.Exports.HandleCallTool = handleCallTool
}

func handleListTools(request tools.ListToolsRequest) cm.Result[toolscapabilities.ListToolsResponseShape, tools.ListToolsResponse, types.McpError] {
	// Define available tools
	toolsList := cm.ToList([]tools.Tool{
		{
			Base: types.BaseMetadata{
				Name:  "echo",
				Title: cm.Some("echo"),
			},
			Description:  cm.Some("Echo a message back to the user"),
			InputSchema:  types.JSONSchema(`{"type": "object", "properties": {"message": {"type": "string", "description": "The message to echo"}}, "required": ["message"]}`),
			OutputSchema: cm.None[types.JSONSchema](),
			Annotations:  cm.None[tools.ToolAnnotations](),
			Meta:         cm.None[types.MetaFields](),
		},
		{
			Base: types.BaseMetadata{
				Name:  "get_weather",
				Title: cm.Some("get_weather"),
			},
			Description:  cm.Some("Get current weather for a location"),
			InputSchema:  types.JSONSchema(`{"type": "object", "properties": {"location": {"type": "string", "description": "City name to get weather for"}}, "required": ["location"]}`),
			OutputSchema: cm.None[types.JSONSchema](),
			Annotations:  cm.None[tools.ToolAnnotations](),
			Meta:         cm.None[types.MetaFields](),
		},
		{
			Base: types.BaseMetadata{
				Name:  "multi_weather",
				Title: cm.Some("multi_weather"),
			},
			Description:  cm.Some("Get weather for multiple cities concurrently"),
			InputSchema:  types.JSONSchema(`{"type": "object", "properties": {"cities": {"type": "array", "description": "List of cities to get weather for", "items": {"type": "string"}}}, "required": ["cities"]}`),
			OutputSchema: cm.None[types.JSONSchema](),
			Annotations:  cm.None[tools.ToolAnnotations](),
			Meta:         cm.None[types.MetaFields](),
		},
	})

	response := tools.ListToolsResponse{
		Tools:      toolsList,
		NextCursor: cm.None[types.Cursor](),
		Meta:       cm.None[types.MetaFields](),
	}

	return cm.OK[cm.Result[toolscapabilities.ListToolsResponseShape, tools.ListToolsResponse, types.McpError]](response)
}

func handleCallTool(request tools.CallToolRequest) cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError] {
	// Parse arguments if provided
	var args map[string]interface{}
	if request.Arguments.Some() != nil {
		argStr := string(*request.Arguments.Some())
		if err := json.Unmarshal([]byte(argStr), &args); err != nil {
			return errorResult(fmt.Sprintf("Invalid arguments: %v", err))
		}
	}

	switch request.Name {
	case "echo":
		return handleEcho(args)
	case "get_weather":
		return handleWeather(args)
	case "multi_weather":
		return handleMultiWeather(args)
	default:
		return mcpError(types.ErrorCodeToolNotFound(), fmt.Sprintf("Unknown tool: %s", request.Name))
	}
}

func handleEcho(args map[string]interface{}) cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError] {
	message, ok := args["message"].(string)
	if !ok {
		return errorResult("Missing 'message' argument")
	}
	return successResult(fmt.Sprintf("Echo: %s", message))
}

func handleWeather(args map[string]interface{}) cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError] {
	location, ok := args["location"].(string)
	if !ok {
		return errorResult("Missing 'location' argument")
	}

	weatherData, err := fetchWeather(location)
	if err != nil {
		return errorResult(fmt.Sprintf("Weather fetch failed: %v", err))
	}

	formatted := formatWeather(weatherData)
	jsonBytes, _ := json.MarshalIndent(formatted, "", "  ")
	return successResult(string(jsonBytes))
}

func handleMultiWeather(args map[string]interface{}) cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError] {
	citiesRaw, ok := args["cities"].([]interface{})
	if !ok {
		return errorResult("Missing 'cities' argument")
	}

	cities := make([]string, len(citiesRaw))
	for i, city := range citiesRaw {
		cities[i], ok = city.(string)
		if !ok {
			return errorResult("Invalid city in list")
		}
	}

	// Fetch weather data concurrently using goroutines
	results := fetchMultiWeather(cities)

	formatted := make([]map[string]interface{}, len(results))
	for i, result := range results {
		if result.err != nil {
			formatted[i] = map[string]interface{}{"error": result.err.Error()}
		} else {
			formatted[i] = formatWeather(result.data)
		}
	}

	jsonBytes, _ := json.MarshalIndent(formatted, "", "  ")
	return successResult(string(jsonBytes))
}

type weatherResult struct {
	data *WeatherData
	err  error
}

func fetchWeather(city string) (*WeatherData, error) {
	// Geocode the city
	geoURL := fmt.Sprintf("https://geocoding-api.open-meteo.com/v1/search?name=%s&count=1", url.QueryEscape(city))

	resp, err := http.Get(geoURL)
	if err != nil {
		return nil, fmt.Errorf("geocoding request failed: %v", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading geocoding response: %v", err)
	}

	var geoData GeocodingResult
	if err := json.Unmarshal(body, &geoData); err != nil {
		return nil, fmt.Errorf("parsing geocoding response: %v", err)
	}

	if len(geoData.Results) == 0 {
		return nil, fmt.Errorf("location '%s' not found", city)
	}

	location := geoData.Results[0]

	// Get weather data
	weatherURL := fmt.Sprintf(
		"https://api.open-meteo.com/v1/forecast?latitude=%f&longitude=%f&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code",
		location.Latitude, location.Longitude,
	)

	resp, err = http.Get(weatherURL)
	if err != nil {
		return nil, fmt.Errorf("weather request failed: %v", err)
	}
	defer resp.Body.Close()

	body, err = io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading weather response: %v", err)
	}

	var weather WeatherResponse
	if err := json.Unmarshal(body, &weather); err != nil {
		return nil, fmt.Errorf("parsing weather response: %v", err)
	}

	return &WeatherData{
		Name:        location.Name,
		Country:     location.Country,
		Temperature: weather.Current.Temperature2m,
		Humidity:    weather.Current.RelativeHumidity2m,
		WindSpeed:   weather.Current.WindSpeed10m,
		WeatherCode: weather.Current.WeatherCode,
	}, nil
}

func fetchMultiWeather(cities []string) []weatherResult {
	results := make([]weatherResult, len(cities))
	var wg sync.WaitGroup

	// Launch goroutines for concurrent fetching
	for i, city := range cities {
		wg.Add(1)
		go func(idx int, c string) {
			defer wg.Done()
			data, err := fetchWeather(c)
			results[idx] = weatherResult{data: data, err: err}
		}(i, city)
	}

	// Wait for all goroutines to complete
	wg.Wait()
	return results
}

func formatWeather(data *WeatherData) map[string]interface{} {
	return map[string]interface{}{
		"location":    fmt.Sprintf("%s, %s", data.Name, data.Country),
		"temperature": fmt.Sprintf("%.1f°C", data.Temperature),
		"conditions":  fmt.Sprintf("Weather code %d", data.WeatherCode),
		"humidity":    fmt.Sprintf("%d%%", data.Humidity),
		"wind":        fmt.Sprintf("%.1f m/s", data.WindSpeed),
	}
}

func successResult(text string) cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError] {
	result := tools.ToolResult{
		Content: cm.ToList([]types.ContentBlock{
			types.ContentBlockText(types.TextContent{
				Text:        text,
				Annotations: cm.None[types.Annotations](),
				Meta:        cm.None[types.MetaFields](),
			}),
		}),
		StructuredContent: cm.None[types.JSONValue](),
		IsError:           cm.Some(false),
		Meta:              cm.None[types.MetaFields](),
	}
	return cm.OK[cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError]](result)
}

func errorResult(text string) cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError] {
	result := tools.ToolResult{
		Content: cm.ToList([]types.ContentBlock{
			types.ContentBlockText(types.TextContent{
				Text:        text,
				Annotations: cm.None[types.Annotations](),
				Meta:        cm.None[types.MetaFields](),
			}),
		}),
		StructuredContent: cm.None[types.JSONValue](),
		IsError:           cm.Some(true),
		Meta:              cm.None[types.MetaFields](),
	}
	return cm.OK[cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError]](result)
}

func mcpError(code types.ErrorCode, message string) cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError] {
	return cm.Err[cm.Result[toolscapabilities.ToolResultShape, tools.ToolResult, types.McpError]](types.McpError{
		Code:    code,
		Message: message,
		Data:    cm.None[string](),
	})
}

// main is required for the wasip2 target, even if unused
func main() {}