package mcp

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	
	_ "github.com/ydnar/wasi-http-go/wasihttp" // enable wasi-http
)

// HTTPClient provides a simple HTTP client using WASI HTTP
type HTTPClient struct{}

// Get performs an HTTP GET request and returns the response body as a string
func (c *HTTPClient) Get(url string) (string, error) {
	resp, err := http.Get(url)
	if err != nil {
		return "", fmt.Errorf("failed to get %s: %w", url, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return "", fmt.Errorf("failed to read response body: %w", err)
	}

	if resp.StatusCode >= 400 {
		return "", fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(body))
	}

	return string(body), nil
}

// GetJSON performs an HTTP GET and decodes JSON response
func (c *HTTPClient) GetJSON(url string, target interface{}) error {
	body, err := c.Get(url)
	if err != nil {
		return err
	}
	
	return json.Unmarshal([]byte(body), target)
}

// DefaultHTTPClient is the default HTTP client instance
var DefaultHTTPClient = &HTTPClient{}