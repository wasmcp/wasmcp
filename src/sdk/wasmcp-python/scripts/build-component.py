#!/usr/bin/env python3
"""Build script for compiling Python MCP handlers to WebAssembly components."""

import argparse
import os
import subprocess
import sys

def main():
    parser = argparse.ArgumentParser(
        description="Compile Python MCP handler to WebAssembly component"
    )
    parser.add_argument(
        "handler",
        help="Path to the Python handler file"
    )
    parser.add_argument(
        "-o", "--output",
        default="handler.wasm",
        help="Output WASM file (default: handler.wasm)"
    )
    parser.add_argument(
        "--wit",
        help="Custom WIT file path (default: uses bundled WIT from SDK)"
    )
    
    args = parser.parse_args()
    
    # Check that handler file exists
    if not os.path.exists(args.handler):
        print(f"Error: Handler file '{args.handler}' not found", file=sys.stderr)
        sys.exit(1)
    
    # Get WIT path
    if args.wit:
        wit_path = args.wit
    else:
        try:
            import wasmcp
            wit_path = wasmcp.get_wit_path()
        except ImportError:
            print("Error: wasmcp SDK not installed. Run: pip install wasmcp", file=sys.stderr)
            sys.exit(1)
    
    # Check that WIT file exists
    if not os.path.exists(wit_path):
        print(f"Error: WIT file '{wit_path}' not found", file=sys.stderr)
        sys.exit(1)
    
    # Check that componentize-py is installed
    try:
        subprocess.run(["componentize-py", "--version"], capture_output=True, check=True)
    except (subprocess.CalledProcessError, FileNotFoundError):
        print("Error: componentize-py not installed. Run: pip install componentize-py", file=sys.stderr)
        sys.exit(1)
    
    # Build the component
    cmd = [
        "componentize-py",
        "-w", wit_path,
        "-o", args.output,
        args.handler
    ]
    
    print(f"Building WebAssembly component...")
    print(f"  Handler: {args.handler}")
    print(f"  WIT: {wit_path}")
    print(f"  Output: {args.output}")
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        if result.stdout:
            print(result.stdout)
        print(f"✓ Successfully built {args.output}")
    except subprocess.CalledProcessError as e:
        print(f"Error: Failed to build component", file=sys.stderr)
        if e.stderr:
            print(e.stderr, file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()