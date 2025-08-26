// Package assets contains embedded WIT files for the wasmcp-go SDK.
// This package has no dependencies on WebAssembly-specific code,
// allowing it to be imported by both host tools and WebAssembly components.
package assets

import (
	"embed"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
)

// WIT contains all WIT interface definitions needed for building MCP components.
//
//go:embed all:wit
var WIT embed.FS

// ExtractTo extracts embedded WIT files to the specified directory.
func ExtractTo(targetDir string) error {
	// Walk through embedded files and extract them
	return fs.WalkDir(WIT, "wit", func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}

		// Calculate destination path
		relPath, _ := filepath.Rel("wit", path)
		destPath := filepath.Join(targetDir, relPath)

		if d.IsDir() {
			return os.MkdirAll(destPath, 0755)
		}

		// Read embedded file
		data, err := WIT.ReadFile(path)
		if err != nil {
			return fmt.Errorf("failed to read %s: %w", path, err)
		}

		// Ensure parent directory exists
		if err := os.MkdirAll(filepath.Dir(destPath), 0755); err != nil {
			return fmt.Errorf("failed to create directory: %w", err)
		}

		// Write to destination
		return os.WriteFile(destPath, data, 0644)
	})
}