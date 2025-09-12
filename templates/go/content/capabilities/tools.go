package capabilities

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"

	"go.bytecodealliance.org/cm"
	authorizationtypes "weather_go/internal/wasmcp/mcp/authorization-types"
	mcptypes "weather_go/internal/wasmcp/mcp/mcp-types"
	tools "weather_go/internal/wasmcp/mcp/tools"
	tooltypes "weather_go/internal/wasmcp/mcp/tools-types"
	"weather_go/wasihttp"
)


// ListTools returns the list of available tools.
//
// The Result type with McpErrorShape is necessary because WIT functions can return
// result<list-tools-result, mcp-error>. The Shape type is for internal storage.
func ListTools(request tooltypes.ListToolsRequest) cm.Result[tools.McpErrorShape, tooltypes.ListToolsResult, mcptypes.McpError] {
	// Define tools directly as a slice, matching the Python pattern.
	// Each tool maps to a WIT record with required and optional fields.
	toolsList := []tooltypes.Tool{
		{
			Name:        "echo",
			Title:       cm.Some("echo"),  // cm.Some wraps a value for WIT's option<T>
			Description: cm.Some("Echo a message back to the user"),
			InputSchema: mcptypes.JSONObject(`{
				"type": "object",
				"properties": {
					"message": {
						"type": "string",
						"description": "The message to echo"
					}
				},
				"required": ["message"]
			}`),
			OutputSchema: cm.None[mcptypes.JSONObject](),
			Icons:        cm.None[cm.List[mcptypes.Icon]](),
			Annotations:  cm.None[tooltypes.ToolAnnotations](),
		},
		{
			Name:        "get_weather",
			Title:       cm.Some("get_weather"),
			Description: cm.Some("Get current weather for a location"),
			InputSchema: mcptypes.JSONObject(`{
				"type": "object",
				"properties": {
					"location": {
						"type": "string",
						"description": "City name to get weather for"
					}
				},
				"required": ["location"]
			}`),
			OutputSchema: cm.None[mcptypes.JSONObject](),
			Icons:        cm.None[cm.List[mcptypes.Icon]](),
			Annotations:  cm.None[tooltypes.ToolAnnotations](),
		},
		{
			Name:        "multi_weather",
			Title:       cm.Some("multi_weather"),
			Description: cm.Some("Get weather for multiple locations concurrently"),
			InputSchema: mcptypes.JSONObject(`{
				"type": "object",
				"properties": {
					"cities": {
						"type": "array",
						"description": "List of city names (max 5)",
						"items": {
							"type": "string"
						}
					}
				},
				"required": ["cities"]
			}`),
			OutputSchema: cm.None[mcptypes.JSONObject](),
			Icons:        cm.None[cm.List[mcptypes.Icon]](),
			Annotations:  cm.None[tooltypes.ToolAnnotations](),
		},
	}

	result := tooltypes.ListToolsResult{
		Tools:      cm.ToList(toolsList),
		NextCursor: cm.None[string](),
	}

	var res cm.Result[tools.McpErrorShape, tooltypes.ListToolsResult, mcptypes.McpError]
	res.SetOK(result)
	return res
}

// CallTool executes a tool with the given request.
//
// The context parameter is cm.Option because WIT defines it as option<auth-context>.
// This allows the authentication context to be optional.
func CallTool(request tooltypes.CallToolRequest, context cm.Option[authorizationtypes.AuthContext]) cm.Result[tools.CallToolResultShape, tooltypes.CallToolResult, mcptypes.McpError] {
	// Parse arguments
	var args json.RawMessage
	if !request.Arguments.None() {
		argStr := *request.Arguments.Some()
		args = json.RawMessage(argStr)
	}

	// Route to tool handler
	var result string
	var err error

	switch request.Name {
	case "echo":
		result, err = handleEcho(args)
	case "get_weather":
		result, err = handleGetWeather(args)
	case "multi_weather":
		result, err = handleMultiWeather(args)
	default:
		return errorResult(fmt.Sprintf("Unknown tool: %s", request.Name))
	}

	if err != nil {
		return errorResult(fmt.Sprintf("Tool execution failed: %v", err))
	}

	return textResult(result)
}

// Helper functions for creating results
func textResult(text string) cm.Result[tools.CallToolResultShape, tooltypes.CallToolResult, mcptypes.McpError] {
	content := mcptypes.ContentBlockText(mcptypes.TextContent{
		Text:        text,
		Meta:        cm.None[mcptypes.JSONObject](),
		Annotations: cm.None[mcptypes.Annotations](),
	})

	result := tooltypes.CallToolResult{
		Content:           cm.ToList([]mcptypes.ContentBlock{content}),
		StructuredContent: cm.None[mcptypes.JSONValue](),
		IsError:           cm.Some(false),
		Meta:              cm.None[mcptypes.JSONObject](),
	}

	var res cm.Result[tools.CallToolResultShape, tooltypes.CallToolResult, mcptypes.McpError]
	res.SetOK(result)
	return res
}

func errorResult(message string) cm.Result[tools.CallToolResultShape, tooltypes.CallToolResult, mcptypes.McpError] {
	content := mcptypes.ContentBlockText(mcptypes.TextContent{
		Text:        message,
		Meta:        cm.None[mcptypes.JSONObject](),
		Annotations: cm.None[mcptypes.Annotations](),
	})

	result := tooltypes.CallToolResult{
		Content:           cm.ToList([]mcptypes.ContentBlock{content}),
		StructuredContent: cm.None[mcptypes.JSONValue](),
		IsError:           cm.Some(true),
		Meta:              cm.None[mcptypes.JSONObject](),
	}

	var res cm.Result[tools.CallToolResultShape, tooltypes.CallToolResult, mcptypes.McpError]
	res.SetOK(result)
	return res
}

// Tool implementations
func handleEcho(args json.RawMessage) (string, error) {
	var params struct {
		Message string `json:"message"`
	}
	if err := json.Unmarshal(args, &params); err != nil {
		return "", fmt.Errorf("invalid arguments: %v", err)
	}
	return fmt.Sprintf("Echo: %s", params.Message), nil
}

func handleGetWeather(args json.RawMessage) (string, error) {
	var params struct {
		Location string `json:"location"`
	}
	if err := json.Unmarshal(args, &params); err != nil {
		return "", fmt.Errorf("invalid arguments: %v", err)
	}

	weatherData, err := fetchWeather(params.Location)
	if err != nil {
		return "", err
	}

	return formatWeather(weatherData), nil
}

func handleMultiWeather(args json.RawMessage) (string, error) {
	var params struct {
		Cities []string `json:"cities"`
	}
	if err := json.Unmarshal(args, &params); err != nil {
		return "", fmt.Errorf("invalid arguments: %v", err)
	}

	if len(params.Cities) == 0 {
		return "No cities provided", nil
	}
	if len(params.Cities) > 5 {
		return "Maximum 5 cities allowed", nil
	}

	// Build URLs for geocoding requests
	geoURLs := make([]string, len(params.Cities))
	for i, city := range params.Cities {
		geoURLs[i] = fmt.Sprintf("https://geocoding-api.open-meteo.com/v1/search?name=%s&count=1", url.QueryEscape(city))
	}

	// Fetch geocoding data concurrently using wasihttp
	geoResponses := wasihttp.GetConcurrently(geoURLs)

	// Process geocoding results and build weather URLs
	weatherURLs := make([]string, 0, len(params.Cities))
	type locationInfo struct {
		city    string
		name    string
		country string
	}
	locations := make([]locationInfo, 0, len(params.Cities))
	var errors []string

	for i, resp := range geoResponses {
		city := params.Cities[i]

		if resp.Error != nil {
			errors = append(errors, fmt.Sprintf("Error fetching location for %s: %v", city, resp.Error))
			continue
		}

		defer resp.Body.Close()
		body, err := io.ReadAll(resp.Body)
		if err != nil {
			errors = append(errors, fmt.Sprintf("Error reading response for %s: %v", city, err))
			continue
		}

		var geoData map[string]interface{}
		if err := json.Unmarshal(body, &geoData); err != nil {
			errors = append(errors, fmt.Sprintf("Error parsing geocoding for %s: %v", city, err))
			continue
		}

		results, ok := geoData["results"].([]interface{})
		if !ok || len(results) == 0 {
			errors = append(errors, fmt.Sprintf("Location '%s' not found", city))
			continue
		}

		location := results[0].(map[string]interface{})
		lat := location["latitude"].(float64)
		lon := location["longitude"].(float64)

		// Extract location name and country
		name, _ := location["name"].(string)
		country, _ := location["country"].(string)

		weatherURL := fmt.Sprintf(
			"https://api.open-meteo.com/v1/forecast?latitude=%f&longitude=%f&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code",
			lat, lon,
		)
		weatherURLs = append(weatherURLs, weatherURL)
		locations = append(locations, locationInfo{
			city:    city,
			name:    name,
			country: country,
		})
	}

	// Fetch weather data concurrently
	var output strings.Builder
	output.WriteString("=== Weather Results ===\n\n")

	if len(weatherURLs) > 0 {
		weatherResponses := wasihttp.GetConcurrently(weatherURLs)

		for i, resp := range weatherResponses {
			loc := locations[i]

			if resp.Error != nil {
				fmt.Fprintf(&output, "Error fetching weather for %s: %v\n\n", loc.city, resp.Error)
				continue
			}

			defer resp.Body.Close()
			body, err := io.ReadAll(resp.Body)
			if err != nil {
				fmt.Fprintf(&output, "Error reading weather for %s: %v\n\n", loc.city, err)
				continue
			}

			var weatherData map[string]interface{}
			if err := json.Unmarshal(body, &weatherData); err != nil {
				fmt.Fprintf(&output, "Error parsing weather for %s: %v\n\n", loc.city, err)
				continue
			}

			// Add location info and current weather data for formatting
			current, _ := weatherData["current"].(map[string]interface{})
			formattedData := map[string]interface{}{
				"name":                 loc.name,
				"country":              loc.country,
				"temperature":          current["temperature_2m"],
				"apparent_temperature": current["apparent_temperature"],
				"humidity":             current["relative_humidity_2m"],
				"wind_speed":           current["wind_speed_10m"],
				"weather_code":         current["weather_code"],
			}

			output.WriteString(formatWeather(formattedData))
			output.WriteString("\n\n")
		}
	}

	// Add any errors from geocoding
	for _, errMsg := range errors {
		output.WriteString(errMsg)
		output.WriteString("\n\n")
	}

	output.WriteString("=== All requests completed ===")
	return output.String(), nil
}

// Weather API helper functions
func fetchWeather(city string) (map[string]interface{}, error) {
	// Geocode the location
	geoURL := fmt.Sprintf("https://geocoding-api.open-meteo.com/v1/search?name=%s&count=1", url.QueryEscape(city))

	geoResp, err := http.Get(geoURL)
	if err != nil {
		return nil, fmt.Errorf("geocoding request failed: %v", err)
	}
	defer geoResp.Body.Close()

	geoBody, err := io.ReadAll(geoResp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading geocoding response: %v", err)
	}

	var geoData map[string]interface{}
	if err := json.Unmarshal(geoBody, &geoData); err != nil {
		return nil, fmt.Errorf("parsing geocoding response: %v", err)
	}

	results, ok := geoData["results"].([]interface{})
	if !ok || len(results) == 0 {
		return nil, fmt.Errorf("location '%s' not found", city)
	}

	location := results[0].(map[string]interface{})
	lat := location["latitude"].(float64)
	lon := location["longitude"].(float64)

	// Get weather data
	weatherURL := fmt.Sprintf(
		"https://api.open-meteo.com/v1/forecast?latitude=%f&longitude=%f&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code",
		lat, lon,
	)

	weatherResp, err := http.Get(weatherURL)
	if err != nil {
		return nil, fmt.Errorf("weather request failed: %v", err)
	}
	defer weatherResp.Body.Close()

	weatherBody, err := io.ReadAll(weatherResp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading weather response: %v", err)
	}

	var weatherData map[string]interface{}
	if err := json.Unmarshal(weatherBody, &weatherData); err != nil {
		return nil, fmt.Errorf("parsing weather response: %v", err)
	}

	current := weatherData["current"].(map[string]interface{})

	return map[string]interface{}{
		"name":                 location["name"],
		"country":              location["country"],
		"temperature":          current["temperature_2m"],
		"apparent_temperature": current["apparent_temperature"],
		"humidity":             current["relative_humidity_2m"],
		"wind_speed":           current["wind_speed_10m"],
		"weather_code":         current["weather_code"],
	}, nil
}

func formatWeather(data map[string]interface{}) string {
	temp := getFloat(data, "temperature")
	feels := getFloat(data, "apparent_temperature")
	humidity := getFloat(data, "humidity")
	wind := getFloat(data, "wind_speed")
	code := getFloat(data, "weather_code")
	name := getString(data, "name")
	country := getString(data, "country")

	return fmt.Sprintf(
		"Weather in %s, %s:\nTemperature: %.1f°C (feels like %.1f°C)\nConditions: %s\nHumidity: %.0f%%\nWind: %.1f km/h",
		name, country, temp, feels, weatherCondition(int(code)), humidity, wind,
	)
}

func getFloat(data map[string]interface{}, key string) float64 {
	if val, ok := data[key].(float64); ok {
		return val
	}
	return 0
}

func getString(data map[string]interface{}, key string) string {
	if val, ok := data[key].(string); ok {
		return val
	}
	return ""
}

func weatherCondition(code int) string {
	conditions := map[int]string{
		0:  "Clear sky",
		1:  "Mainly clear",
		2:  "Partly cloudy",
		3:  "Overcast",
		45: "Foggy",
		48: "Depositing rime fog",
		51: "Light drizzle",
		53: "Moderate drizzle",
		55: "Dense drizzle",
		61: "Slight rain",
		63: "Moderate rain",
		65: "Heavy rain",
		71: "Slight snow fall",
		73: "Moderate snow fall",
		75: "Heavy snow fall",
		80: "Slight rain showers",
		81: "Moderate rain showers",
		82: "Violent rain showers",
		85: "Slight snow showers",
		86: "Heavy snow showers",
		95: "Thunderstorm",
		96: "Thunderstorm with slight hail",
		99: "Thunderstorm with heavy hail",
	}
	if condition, ok := conditions[code]; ok {
		return condition
	}
	return "Unknown"
}