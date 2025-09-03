package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"

	"go.bytecodealliance.org/cm"
	authorizationtypes "weather_go/internal/fastertools/mcp/authorization-types"
	corecapabilities "weather_go/internal/fastertools/mcp/core-capabilities"
	"weather_go/wasihttp"
)

// EchoArgs contains arguments for the echo tool
type EchoArgs struct {
	Message string `json:"message"`
}

// WeatherArgs contains arguments for the weather tool
type WeatherArgs struct {
	Location string `json:"location"`
}

// MultiWeatherArgs contains arguments for the multi-weather tool
type MultiWeatherArgs struct {
	Cities []string `json:"cities"`
}

// ==============================================================================
// OAuth 2.0 authentication configuration.
//
// To enable authentication:
// 1. Uncomment the authConfig() function below
// 2. Replace the placeholder values with your actual OAuth provider details
// 3. Run `make build` to rebuild with authentication enabled
//
// To disable authentication:
// - Comment out the authConfig() function or have it return None
// ==============================================================================

// authConfig returns the OAuth 2.0 configuration for this provider
func authConfig() cm.Option[authorizationtypes.ProviderAuthConfig] {
	// Uncomment and configure the lines below to enable OAuth 2.0 authentication:
	/*
	return cm.Some(authorizationtypes.ProviderAuthConfig{
		ExpectedIssuer: "https://your-auth-domain.example.com",
		ExpectedAudiences: cm.NewList([]string{"your-client-id"}),
		JwksURI: "https://your-auth-domain.example.com/oauth2/jwks",
		Policy: cm.None[string](),     // Optional: Add Rego policy as a string for additional authorization rules
		PolicyData: cm.None[string](), // Optional: Add policy data as JSON string
	})
	*/
	return cm.None[authorizationtypes.ProviderAuthConfig]()
}

func init() {
	// Set up the auth configuration export
	corecapabilities.Exports.GetAuthConfig = authConfig
	// Configure WASI HTTP transport
	http.DefaultTransport = &wasihttp.Transport{}

	// Create the server
	server := NewServer(
		&Implementation{Name: "weather_go", Version: "v1.0.0"},
		nil,
	)

	// Register echo tool
	AddTool(server, &Tool{
		Name:        "echo",
		Description: "Echo a message back to the user",
		InputSchema: Schema(`{
			"type": "object",
			"properties": {
				"message": {
					"type": "string",
					"description": "The message to echo"
				}
			},
			"required": ["message"]
		}`),
	}, handleEcho)

	// Register weather tool
	AddTool(server, &Tool{
		Name:        "get_weather",
		Description: "Get current weather for a location",
		InputSchema: Schema(`{
			"type": "object",
			"properties": {
				"location": {
					"type": "string",
					"description": "City name to get weather for"
				}
			},
			"required": ["location"]
		}`),
	}, handleWeather)

	// Register multi-weather tool
	AddTool(server, &Tool{
		Name:        "multi_weather",
		Description: "Get weather for multiple cities concurrently",
		InputSchema: Schema(`{
			"type": "object",
			"properties": {
				"cities": {
					"type": "array",
					"description": "List of cities to get weather for",
					"items": {
						"type": "string"
					},
					"minItems": 1,
					"maxItems": 5
				}
			},
			"required": ["cities"]
		}`),
	}, handleMultiWeather)

	// Run the server
	server.Run(context.Background(), nil)
}

// Tool handlers as package-level functions
func handleEcho(ctx context.Context, args EchoArgs) (*CallToolResult, error) {
	return TextResult(fmt.Sprintf("Echo: %s", args.Message)), nil
}

func handleWeather(ctx context.Context, args WeatherArgs) (*CallToolResult, error) {
	result, err := getWeatherForCity(args.Location)
	if err != nil {
		return ErrorResult(fmt.Sprintf("Failed to get weather: %v", err)), nil
	}
	return TextResult(result), nil
}

func handleMultiWeather(ctx context.Context, args MultiWeatherArgs) (*CallToolResult, error) {
	if len(args.Cities) == 0 {
		return ErrorResult("No cities provided"), nil
	}

	if len(args.Cities) > 5 {
		return ErrorResult("Maximum 5 cities allowed"), nil
	}

	// Fetch weather data concurrently using WASI polling
	results := fetchMultiWeatherConcurrent(args.Cities)

	// Format results
	var output strings.Builder
	output.WriteString("=== Concurrent Weather Results ===\n\n")

	for _, result := range results {
		if result.err != nil {
			output.WriteString(fmt.Sprintf("Error fetching weather for %s: %v\n\n", result.city, result.err))
		} else {
			output.WriteString(result.weather)
			output.WriteString("\n\n")
		}
	}

	output.WriteString("=== All requests completed concurrently ===")

	return TextResult(output.String()), nil
}

func getWeatherForCity(location string) (string, error) {
	// Geocode the location
	geocodingURL := fmt.Sprintf("https://geocoding-api.open-meteo.com/v1/search?name=%s&count=1",
		url.QueryEscape(location))

	resp, err := http.Get(geocodingURL)
	if err != nil {
		return "", fmt.Errorf("failed to geocode location: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("failed to read response: %w", err)
	}

	var geocodingData struct {
		Results []struct {
			Latitude  float64 `json:"latitude"`
			Longitude float64 `json:"longitude"`
			Name      string  `json:"name"`
			Country   string  `json:"country"`
		} `json:"results"`
	}

	if err := json.Unmarshal(body, &geocodingData); err != nil {
		return "", fmt.Errorf("failed to parse geocoding response: %w", err)
	}

	if len(geocodingData.Results) == 0 {
		return "", fmt.Errorf("location '%s' not found", location)
	}

	loc := geocodingData.Results[0]

	// Fetch weather data
	weatherURL := fmt.Sprintf("https://api.open-meteo.com/v1/forecast?latitude=%f&longitude=%f&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code",
		loc.Latitude, loc.Longitude)

	weatherResp, err := http.Get(weatherURL)
	if err != nil {
		return "", fmt.Errorf("failed to fetch weather: %w", err)
	}
	defer weatherResp.Body.Close()

	weatherBody, err := io.ReadAll(weatherResp.Body)
	if err != nil {
		return "", fmt.Errorf("failed to read weather response: %w", err)
	}

	var weatherData struct {
		Current struct {
			Temperature         float64 `json:"temperature_2m"`
			ApparentTemperature float64 `json:"apparent_temperature"`
			Humidity            int     `json:"relative_humidity_2m"`
			WindSpeed           float64 `json:"wind_speed_10m"`
			WeatherCode         int     `json:"weather_code"`
		} `json:"current"`
	}

	if err := json.Unmarshal(weatherBody, &weatherData); err != nil {
		return "", fmt.Errorf("failed to parse weather response: %w", err)
	}

	conditions := getWeatherCondition(weatherData.Current.WeatherCode)

	result := fmt.Sprintf(`Weather in %s, %s:
Temperature: %.1f°C (feels like %.1f°C)
Conditions: %s
Humidity: %d%%
Wind: %.1f km/h`,
		loc.Name,
		loc.Country,
		weatherData.Current.Temperature,
		weatherData.Current.ApparentTemperature,
		conditions,
		weatherData.Current.Humidity,
		weatherData.Current.WindSpeed)

	return result, nil
}

type weatherResult struct {
	city    string
	weather string
	err     error
}

func fetchMultiWeatherConcurrent(cities []string) []weatherResult {
	// Build URLs for all cities (geocoding first)
	geocodeURLs := make([]string, len(cities))
	for i, city := range cities {
		geocodeURLs[i] = fmt.Sprintf("https://geocoding-api.open-meteo.com/v1/search?name=%s&count=1",
			url.QueryEscape(city))
	}

	// Fetch all geocoding results concurrently using WASI polling
	client := wasihttp.DefaultClient
	geocodeResponses := client.GetConcurrently(geocodeURLs)

	// Process geocoding results and build weather URLs
	weatherURLs := make([]string, 0, len(cities))
	cityIndexMap := make(map[int]int) // maps weather request index to city index
	results := make([]weatherResult, len(cities))

	for i, resp := range geocodeResponses {
		results[i].city = cities[i]

		if resp.Error != nil {
			results[i].err = fmt.Errorf("geocoding failed: %v", resp.Error)
			continue
		}

		body, err := io.ReadAll(resp.Body)
		resp.Body.Close()
		if err != nil {
			results[i].err = fmt.Errorf("reading geocode response: %v", err)
			continue
		}

		var geocodingData struct {
			Results []struct {
				Latitude  float64 `json:"latitude"`
				Longitude float64 `json:"longitude"`
				Name      string  `json:"name"`
				Country   string  `json:"country"`
			} `json:"results"`
		}

		if err := json.Unmarshal(body, &geocodingData); err != nil {
			results[i].err = fmt.Errorf("parsing geocode response: %v", err)
			continue
		}

		if len(geocodingData.Results) == 0 {
			results[i].err = fmt.Errorf("location not found")
			continue
		}

		loc := geocodingData.Results[0]

		// Build weather URL
		weatherURL := fmt.Sprintf("https://api.open-meteo.com/v1/forecast?latitude=%f&longitude=%f&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code",
			loc.Latitude, loc.Longitude)
		cityIndexMap[len(weatherURLs)] = i
		weatherURLs = append(weatherURLs, weatherURL)

		// Store location info for later formatting
		results[i].weather = fmt.Sprintf("%s, %s", loc.Name, loc.Country)
	}

	// Fetch all weather data concurrently
	if len(weatherURLs) > 0 {
		weatherResponses := client.GetConcurrently(weatherURLs)

		for weatherIdx, resp := range weatherResponses {
			cityIdx := cityIndexMap[weatherIdx]

			if resp.Error != nil {
				results[cityIdx].err = fmt.Errorf("weather fetch failed: %v", resp.Error)
				continue
			}

			body, err := io.ReadAll(resp.Body)
			resp.Body.Close()
			if err != nil {
				results[cityIdx].err = fmt.Errorf("reading weather response: %v", err)
				continue
			}

			var weatherData struct {
				Current struct {
					Temperature         float64 `json:"temperature_2m"`
					ApparentTemperature float64 `json:"apparent_temperature"`
					Humidity            int     `json:"relative_humidity_2m"`
					WindSpeed           float64 `json:"wind_speed_10m"`
					WeatherCode         int     `json:"weather_code"`
				} `json:"current"`
			}

			if err := json.Unmarshal(body, &weatherData); err != nil {
				results[cityIdx].err = fmt.Errorf("parsing weather response: %v", err)
				continue
			}

			conditions := getWeatherCondition(weatherData.Current.WeatherCode)

			// Get location name from previous storage
			locName := results[cityIdx].weather

			results[cityIdx].weather = fmt.Sprintf(`Weather in %s:
Temperature: %.1f°C (feels like %.1f°C)
Conditions: %s
Humidity: %d%%
Wind: %.1f km/h`,
				locName,
				weatherData.Current.Temperature,
				weatherData.Current.ApparentTemperature,
				conditions,
				weatherData.Current.Humidity,
				weatherData.Current.WindSpeed)
		}
	}

	return results
}

func getWeatherCondition(code int) string {
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
		56: "Light freezing drizzle",
		57: "Dense freezing drizzle",
		61: "Slight rain",
		63: "Moderate rain",
		65: "Heavy rain",
		66: "Light freezing rain",
		67: "Heavy freezing rain",
		71: "Slight snow fall",
		73: "Moderate snow fall",
		75: "Heavy snow fall",
		77: "Snow grains",
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

// main is required for TinyGo/Wasm but remains empty
func main() {}
