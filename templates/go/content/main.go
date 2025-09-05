package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"

	"go.bytecodealliance.org/cm"

	// Import the generated WIT bindings directly - these are our SDK
	authorizationtypes "{{project-name | snake_case}}/internal/fastertools/mcp/authorization-types"
	corecapabilities "{{project-name | snake_case}}/internal/fastertools/mcp/core-capabilities"
	coretypes "{{project-name | snake_case}}/internal/fastertools/mcp/core-types"
	tooltypes "{{project-name | snake_case}}/internal/fastertools/mcp/tool-types"
	toolscapabilities "{{project-name | snake_case}}/internal/fastertools/mcp/tools-capabilities"
	mcptypes "{{project-name | snake_case}}/internal/fastertools/mcp/types"
	"{{project-name | snake_case}}/wasihttp"
)

// Tool definitions as a simple map
var tools = map[string]struct {
	description string
	schema      string
	handler     func(json.RawMessage) (string, error)
}{
	"echo": {
		description: "Echo a message back to the user",
		schema: `{
			"type": "object",
			"properties": {
				"message": {
					"type": "string",
					"description": "The message to echo"
				}
			},
			"required": ["message"]
		}`,
		handler: handleEcho,
	},
	"get_weather": {
		description: "Get current weather for a location",
		schema: `{
			"type": "object",
			"properties": {
				"location": {
					"type": "string",
					"description": "City name to get weather for"
				}
			},
			"required": ["location"]
		}`,
		handler: handleGetWeather,
	},
	"multi_weather": {
		description: "Get weather for multiple cities concurrently",
		schema: `{
			"type": "object",
			"properties": {
				"cities": {
					"type": "array",
					"items": {"type": "string"},
					"description": "List of city names (max 5)"
				}
			},
			"required": ["cities"]
		}`,
		handler: handleMultiWeather,
	},
}

func init() {
	// Configure WASI HTTP transport
	http.DefaultTransport = &wasihttp.Transport{}

	// -------------------------------------------------------------------------
	// Core Capabilities (session management)
	// -------------------------------------------------------------------------

	corecapabilities.Exports.HandleInitialize = handleInitialize
	corecapabilities.Exports.HandleInitialized = handleInitialized
	corecapabilities.Exports.HandlePing = handlePing
	corecapabilities.Exports.HandleShutdown = handleShutdown
	corecapabilities.Exports.GetAuthConfig = getAuthConfig
	corecapabilities.Exports.JwksCacheGet = jwksCacheGet
	corecapabilities.Exports.JwksCacheSet = jwksCacheSet

	// -------------------------------------------------------------------------
	// Tools Capabilities
	// -------------------------------------------------------------------------

	toolscapabilities.Exports.HandleListTools = handleListTools
	toolscapabilities.Exports.HandleCallTool = handleCallTool
}

// -------------------------------------------------------------------------
// Core Capabilities Implementation
// -------------------------------------------------------------------------

func handleInitialize(request coretypes.InitializeRequest) cm.Result[corecapabilities.InitializeResponseShape, corecapabilities.InitializeResponse, mcptypes.McpError] {
	response := coretypes.InitializeResponse{
		ProtocolVersion: coretypes.ProtocolVersionV20250618,
		Capabilities: coretypes.ServerCapabilities{
			Tools: cm.Some(coretypes.ToolsCapability{
				ListChanged: cm.None[bool](),
			}),
			Experimental: cm.None[coretypes.MetaFields](),
			Logging:      cm.None[bool](),
			Completions:  cm.None[bool](),
			Prompts:      cm.None[coretypes.PromptsCapability](),
			Resources:    cm.None[coretypes.ResourcesCapability](),
		},
		ServerInfo: coretypes.ImplementationInfo{
			Name:    "{{project-name}}",
			Version: "0.1.0",
			Title:   cm.Some("{{project-name}} Server"),
		},
		Instructions: cm.Some("A Go MCP server providing weather tools"),
		Meta:         cm.None[mcptypes.MetaFields](),
	}

	// Create a result with the response
	var result cm.Result[corecapabilities.InitializeResponseShape, corecapabilities.InitializeResponse, mcptypes.McpError]
	result.SetOK(response)
	return result
}

func handleInitialized() cm.Result[mcptypes.McpError, struct{}, mcptypes.McpError] {
	var result cm.Result[mcptypes.McpError, struct{}, mcptypes.McpError]
	result.SetOK(struct{}{})
	return result
}

func handlePing() cm.Result[mcptypes.McpError, struct{}, mcptypes.McpError] {
	var result cm.Result[mcptypes.McpError, struct{}, mcptypes.McpError]
	result.SetOK(struct{}{})
	return result
}

func handleShutdown() cm.Result[mcptypes.McpError, struct{}, mcptypes.McpError] {
	var result cm.Result[mcptypes.McpError, struct{}, mcptypes.McpError]
	result.SetOK(struct{}{})
	return result
}

func getAuthConfig() cm.Option[authorizationtypes.ProviderAuthConfig] {
	// Uncomment and configure to enable OAuth authentication:
	// return cm.Some(authorizationtypes.ProviderAuthConfig{
	//     ExpectedIssuer: "https://your-auth-domain.example.com",
	//     ExpectedAudiences: cm.ToList([]string{"your-client-id"}),
	//     JwksURI: "https://your-auth-domain.example.com/oauth2/jwks",
	//     Policy: cm.None[string](),
	//     PolicyData: cm.None[string](),
	// })
	return cm.None[authorizationtypes.ProviderAuthConfig]()
}

func jwksCacheGet(jwksURI string) cm.Option[string] {
	// Optional: Implement JWKS caching
	return cm.None[string]()
}

func jwksCacheSet(jwksURI string, jwks string) {
	// Optional: Implement JWKS caching
}

// -------------------------------------------------------------------------
// Tools Capabilities Implementation
// -------------------------------------------------------------------------

func handleListTools(request tooltypes.ListToolsRequest) cm.Result[toolscapabilities.ListToolsResponseShape, toolscapabilities.ListToolsResponse, mcptypes.McpError] {
	var toolsList []tooltypes.Tool

	for name, def := range tools {
		tool := tooltypes.Tool{
			Base: mcptypes.BaseMetadata{
				Name:  name,
				Title: cm.Some(name),
			},
			Description:  cm.Some(def.description),
			InputSchema:  mcptypes.JSONSchema(def.schema),
			OutputSchema: cm.None[mcptypes.JSONSchema](),
			Annotations:  cm.None[tooltypes.ToolAnnotations](),
			Meta:         cm.None[mcptypes.MetaFields](),
		}
		toolsList = append(toolsList, tool)
	}

	response := tooltypes.ListToolsResponse{
		Tools:      cm.ToList(toolsList),
		NextCursor: cm.None[mcptypes.Cursor](),
		Meta:       cm.None[mcptypes.MetaFields](),
	}

	var result cm.Result[toolscapabilities.ListToolsResponseShape, toolscapabilities.ListToolsResponse, mcptypes.McpError]
	result.SetOK(response)
	return result
}

func handleCallTool(request tooltypes.CallToolRequest) cm.Result[toolscapabilities.ToolResultShape, toolscapabilities.ToolResult, mcptypes.McpError] {
	tool, exists := tools[request.Name]
	if !exists {
		return errorResult(fmt.Sprintf("Unknown tool: %s", request.Name))
	}

	// Parse arguments
	var args json.RawMessage
	if !request.Arguments.None() {
		argStr := *request.Arguments.Some()
		args = json.RawMessage(argStr)
	}

	// Execute tool
	result, err := tool.handler(args)
	if err != nil {
		return errorResult(fmt.Sprintf("Tool execution failed: %v", err))
	}

	return textResult(result)
}

// -------------------------------------------------------------------------
// Tool Implementations
// -------------------------------------------------------------------------

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

// -------------------------------------------------------------------------
// Weather API Functions
// -------------------------------------------------------------------------

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
	code := int(data["weather_code"].(float64))
	condition := weatherCondition(code)

	return fmt.Sprintf(
		"Weather in %s, %s:\nTemperature: %.1f°C (feels like %.1f°C)\nConditions: %s\nHumidity: %.0f%%\nWind: %.1f km/h",
		data["name"], data["country"],
		data["temperature"], data["apparent_temperature"],
		condition,
		data["humidity"],
		data["wind_speed"],
	)
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

// -------------------------------------------------------------------------
// Helper Functions for Creating MCP Results
// -------------------------------------------------------------------------

func textResult(text string) cm.Result[toolscapabilities.ToolResultShape, toolscapabilities.ToolResult, mcptypes.McpError] {
	content := mcptypes.ContentBlockText(mcptypes.TextContent{
		Text:        text,
		Annotations: cm.None[mcptypes.Annotations](),
		Meta:        cm.None[mcptypes.MetaFields](),
	})

	toolResult := tooltypes.ToolResult{
		Content:           cm.ToList([]mcptypes.ContentBlock{content}),
		StructuredContent: cm.None[mcptypes.JSONValue](),
		IsError:           cm.Some(false),
		Meta:              cm.None[mcptypes.MetaFields](),
	}

	var result cm.Result[toolscapabilities.ToolResultShape, toolscapabilities.ToolResult, mcptypes.McpError]
	result.SetOK(toolResult)
	return result
}

func errorResult(message string) cm.Result[toolscapabilities.ToolResultShape, toolscapabilities.ToolResult, mcptypes.McpError] {
	content := mcptypes.ContentBlockText(mcptypes.TextContent{
		Text:        message,
		Annotations: cm.None[mcptypes.Annotations](),
		Meta:        cm.None[mcptypes.MetaFields](),
	})

	toolResult := tooltypes.ToolResult{
		Content:           cm.ToList([]mcptypes.ContentBlock{content}),
		StructuredContent: cm.None[mcptypes.JSONValue](),
		IsError:           cm.Some(true),
		Meta:              cm.None[mcptypes.MetaFields](),
	}

	var result cm.Result[toolscapabilities.ToolResultShape, toolscapabilities.ToolResult, mcptypes.McpError]
	result.SetOK(toolResult)
	return result
}

// Required main function for WebAssembly
func main() {
	// WebAssembly components run until the host terminates them
	// The exports are already initialized in init()
	select {}
}
