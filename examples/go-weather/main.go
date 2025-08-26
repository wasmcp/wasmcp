package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strings"
	"sync"

	mcp "github.com/fastertools/wasmcp/src/sdk/wasmcp-go"
)

func init() {
	mcp.Handle(func(h *mcp.Handler) {
		// Register tools
		h.Tool("echo", "Echo a message back to the user", echoSchema(), echoHandler)
		h.Tool("weather", "Get current weather for a location using Open-Meteo API", weatherSchema(), weatherHandler)
		h.Tool("multi_weather", "Get weather for multiple cities concurrently", multiWeatherSchema(), multiWeatherHandler)
	})
}

func echoSchema() json.RawMessage {
	return mcp.Schema(`{
		"type": "object",
		"properties": {
			"message": {
				"type": "string",
				"description": "Message to echo back",
				"minLength": 1
			}
		},
		"required": ["message"]
	}`)
}

func echoHandler(args json.RawMessage) (string, error) {
	var params struct {
		Message string `json:"message"`
	}
	if err := json.Unmarshal(args, &params); err != nil {
		return "", fmt.Errorf("invalid arguments: %w", err)
	}
	return fmt.Sprintf("Echo: %s", params.Message), nil
}

func weatherSchema() json.RawMessage {
	return mcp.Schema(`{
		"type": "object",
		"properties": {
			"location": {
				"type": "string",
				"description": "City name to get weather for"
			}
		},
		"required": ["location"]
	}`)
}

func weatherHandler(args json.RawMessage) (string, error) {
	var params struct {
		Location string `json:"location"`
	}
	if err := json.Unmarshal(args, &params); err != nil {
		return "", fmt.Errorf("invalid arguments: %w", err)
	}

	// First, geocode the location using standard net/http
	geocodingUrl := fmt.Sprintf("https://geocoding-api.open-meteo.com/v1/search?name=%s&count=1", 
		url.QueryEscape(params.Location))
	
	resp, err := http.Get(geocodingUrl)
	if err != nil {
		return "", fmt.Errorf("failed to geocode location: %w", err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("failed to read geocoding response: %w", err)
	}

	var geocodingData struct {
		Results []struct {
			Latitude  float64 `json:"latitude"`
			Longitude float64 `json:"longitude"`
			Name      string  `json:"name"`
		} `json:"results"`
	}
	
	if err := json.Unmarshal(body, &geocodingData); err != nil {
		return "", fmt.Errorf("failed to parse geocoding response: %w", err)
	}

	if len(geocodingData.Results) == 0 {
		return fmt.Sprintf("Location '%s' not found", params.Location), nil
	}

	location := geocodingData.Results[0]

	// Now fetch the weather data using standard net/http
	weatherUrl := fmt.Sprintf("https://api.open-meteo.com/v1/forecast?latitude=%f&longitude=%f&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code",
		location.Latitude, location.Longitude)
	
	weatherResp, err := http.Get(weatherUrl)
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
			Temperature2m         float64 `json:"temperature_2m"`
			ApparentTemperature   float64 `json:"apparent_temperature"`
			RelativeHumidity2m    int     `json:"relative_humidity_2m"`
			WindSpeed10m          float64 `json:"wind_speed_10m"`
			WeatherCode           int     `json:"weather_code"`
		} `json:"current"`
	}
	
	if err := json.Unmarshal(weatherBody, &weatherData); err != nil {
		return "", fmt.Errorf("failed to parse weather response: %w", err)
	}

	conditions := getWeatherCondition(weatherData.Current.WeatherCode)

	return fmt.Sprintf(`Weather in %s:
Temperature: %.1f°C (feels like %.1f°C)
Conditions: %s
Humidity: %d%%
Wind: %.1f km/h`,
		location.Name,
		weatherData.Current.Temperature2m,
		weatherData.Current.ApparentTemperature,
		conditions,
		weatherData.Current.RelativeHumidity2m,
		weatherData.Current.WindSpeed10m), nil
}

// Helper function to decode weather conditions
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

func multiWeatherSchema() json.RawMessage {
	return mcp.Schema(`{
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
	}`)
}

func multiWeatherHandler(args json.RawMessage) (string, error) {
	var params struct {
		Cities []string `json:"cities"`
	}
	if err := json.Unmarshal(args, &params); err != nil {
		return "", fmt.Errorf("invalid arguments: %w", err)
	}

	type cityWeather struct {
		city string
		weather string
		err error
	}

	// Channel to collect results
	results := make(chan cityWeather, len(params.Cities))
	
	// WaitGroup to wait for all goroutines
	var wg sync.WaitGroup
	
	// Launch concurrent requests for each city
	for _, city := range params.Cities {
		wg.Add(1)
		go func(cityName string) {
			defer wg.Done()
			
			// Call the existing weather function
			weatherJSON, _ := json.Marshal(map[string]string{"location": cityName})
			weatherInfo, err := weatherHandler(weatherJSON)
			
			results <- cityWeather{
				city: cityName,
				weather: weatherInfo,
				err: err,
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
	
	return output.String(), nil
}

func main() {
	// Required for TinyGo - must be empty
}