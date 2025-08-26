package main

import (
	"encoding/json"
	"fmt"

	mcp "github.com/fastertools/wasmcp-go"
)

func init() {
	mcp.Handle(func(h *mcp.Handler) {
		// Register tools
		h.Tool("echo", "Echo a message back", echoSchema(), echoHandler)

		// Register resources (optional)
		// h.Resource("config://app", "Application config", "", "text/plain", configHandler)

		// Register prompts (optional)
		// h.Prompt("greeting", "Generate a greeting", greetingArgs(), greetingHandler)
	})
}

func echoSchema() json.RawMessage {
	return mcp.Schema(`{
		"type": "object",
		"properties": {
			"message": {
				"type": "string",
				"description": "Message to echo back"
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

// Example of using Spin SDK for HTTP requests:
// import spinhttp "github.com/fermyon/spin-go-sdk/http"
//
// func weatherHandler(args json.RawMessage) (string, error) {
//     var params struct {
//         Location string `json:"location"`
//     }
//     json.Unmarshal(args, &params)
//     
//     resp, err := spinhttp.Get(fmt.Sprintf("https://api.weather.com/%s", params.Location))
//     if err != nil {
//         return "", err
//     }
//     return resp.Body, nil
// }

func main() {
	// Required for TinyGo - must be empty
}