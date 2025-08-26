package main

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"sync"

	mcp "github.com/fastertools/wasmcp/sdk/go"
)

func init() {
	// Create the server
	server := mcp.NewServer(
		&mcp.Implementation{Name: "go_fresh", Version: "v1.0.0"},
		nil,
	)
	
	// Echo tool - demonstrates typed handler with struct args
	type EchoArgs struct {
		Message string `json:"message"`
	}
	
	mcp.AddTool(server, &mcp.Tool{
		Name:        "echo",
		Description: "Echo a message back to the user",
		InputSchema: mcp.Schema(`{
			"type": "object",
			"properties": {
				"message": {
					"type": "string",
					"description": "The message to echo"
				}
			},
			"required": ["message"]
		}`),
	}, func(ctx context.Context, args EchoArgs) (*mcp.CallToolResult, error) {
		return &mcp.CallToolResult{
			Content: []mcp.Content{
				&mcp.TextContent{Text: fmt.Sprintf("Echo: %s", args.Message)},
			},
		}, nil
	})
	
	// Weather tool - demonstrates typed handler
	type WeatherArgs struct {
		Location string `json:"location"`
	}
	
	mcp.AddTool(server, &mcp.Tool{
		Name:        "weather",
		Description: "Get current weather for a location",
		InputSchema: mcp.Schema(`{
			"type": "object",
			"properties": {
				"location": {
					"type": "string",
					"description": "City name to get weather for"
				}
			},
			"required": ["location"]
		}`),
	}, func(ctx context.Context, args WeatherArgs) (*mcp.CallToolResult, error) {
		result, err := getWeatherForCity(args.Location)
		if err != nil {
			return nil, err
		}
		return &mcp.CallToolResult{
			Content: []mcp.Content{&mcp.TextContent{Text: result}},
		}, nil
	})
	
	// Multi-weather tool - demonstrates concurrent requests with typed struct
	type MultiWeatherArgs struct {
		Cities []string `json:"cities"`
	}
	
	mcp.AddTool(server, &mcp.Tool{
		Name:        "multi_weather",
		Description: "Get weather for multiple cities concurrently",
		InputSchema: mcp.Schema(`{
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
	}, func(ctx context.Context, args MultiWeatherArgs) (*mcp.CallToolResult, error) {
		type cityWeather struct {
			city    string
			weather string
			err     error
		}

		// Channel to collect results
		results := make(chan cityWeather, len(args.Cities))
		
		// WaitGroup to wait for all goroutines
		var wg sync.WaitGroup
		
		// Launch concurrent requests for each city
		for _, city := range args.Cities {
			wg.Add(1)
			go func(cityName string) {
				defer wg.Done()
				
				// Fetch weather concurrently using the helper
				weatherInfo, err := getWeatherForCity(cityName)
				
				results <- cityWeather{
					city:    cityName,
					weather: weatherInfo,
					err:     err,
				}
			}(city)
		}
		
		// Wait for all goroutines to complete
		go func() {
			wg.Wait()
			close(results)
		}()
		
		// Collect results
		var output strings.Builder
		output.WriteString("=== Concurrent Weather Results ===\n\n")
		
		for result := range results {
			if result.err != nil {
				output.WriteString(fmt.Sprintf("Error fetching weather for %s: %v\n\n", result.city, result.err))
			} else {
				output.WriteString(result.weather)
				output.WriteString("\n\n")
			}
		}
		
		output.WriteString("=== All requests completed concurrently ===")
		
		return &mcp.CallToolResult{
			Content: []mcp.Content{&mcp.TextContent{Text: output.String()}},
		}, nil
	})
	
	// Run the server (initializes WASM exports)
	server.Run(context.Background(), nil)
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
		return fmt.Sprintf("Location '%s' not found", location), nil
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
		61: "Slight rain",
		63: "Moderate rain",
		65: "Heavy rain",
		71: "Slight snow",
		73: "Moderate snow",
		75: "Heavy snow",
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

func main() {
	// Required for TinyGo - must be empty
}