// Command extract-wit extracts embedded WIT files to a specified directory.
// Usage: extract-wit <target-dir>
package main

import (
	"fmt"
	"os"

	"github.com/fastertools/wasmcp/sdk/go/internal/assets"
)

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintf(os.Stderr, "Usage: %s <target-dir>\n", os.Args[0])
		os.Exit(1)
	}

	targetDir := os.Args[1]
	
	// Extract WIT files to the specified directory
	if err := assets.ExtractTo(targetDir); err != nil {
		fmt.Fprintf(os.Stderr, "Error extracting WIT files: %v\n", err)
		os.Exit(1)
	}
}