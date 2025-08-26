#!/usr/bin/env python3
"""Build module for compiling Python MCP handlers to WebAssembly components.

Can be invoked as:
    python -m wasmcp.build handler.py -o handler.wasm
"""

import argparse
import os
import subprocess
import sys
import json
from pathlib import Path
from typing import Optional, List

class WasmcpBuilder:
    """Builder for WebAssembly MCP components."""
    
    def __init__(self, handler_file: str, output: Optional[str] = None):
        """Initialize the builder.
        
        Args:
            handler_file: Path to the Python handler file
            output: Output WASM file path (defaults to handler.wasm)
        """
        self.handler_file = handler_file
        self.output = output or "handler.wasm"
        self.package_dir = Path(__file__).parent
        self.wit_dir = self.package_dir.parent.parent / "wit"
    
    def get_wit_world_path(self) -> str:
        """Get path to the complete WIT world definition."""
        return str(self.wit_dir / "world.wit")
    
    def get_componentize_config(self) -> str:
        """Get path to componentize-py configuration."""
        return str(self.package_dir / "componentize-py.toml")
    
    def check_dependencies(self) -> bool:
        """Check that required dependencies are installed.
        
        Returns:
            True if all dependencies are present
        """
        # Check componentize-py
        try:
            result = subprocess.run(
                ["componentize-py", "--version"],
                capture_output=True,
                text=True,
                check=False
            )
            if result.returncode != 0:
                print("Error: componentize-py not installed", file=sys.stderr)
                print("Install with: pip install componentize-py", file=sys.stderr)
                return False
        except FileNotFoundError:
            print("Error: componentize-py not found", file=sys.stderr)
            print("Install with: pip install componentize-py", file=sys.stderr)
            return False
        
        return True
    
    def validate_handler(self) -> bool:
        """Validate the handler file.
        
        Returns:
            True if handler is valid
        """
        if not os.path.exists(self.handler_file):
            print(f"Error: Handler file '{self.handler_file}' not found", file=sys.stderr)
            return False
        
        # Basic Python syntax check
        try:
            with open(self.handler_file) as f:
                compile(f.read(), self.handler_file, 'exec')
        except SyntaxError as e:
            print(f"Error: Python syntax error in handler: {e}", file=sys.stderr)
            return False
        
        return True
    
    def build(self, verbose: bool = False) -> bool:
        """Build the WebAssembly component.
        
        Args:
            verbose: Enable verbose output
            
        Returns:
            True if build succeeded
        """
        # Validate environment
        if not self.check_dependencies():
            return False
        
        if not self.validate_handler():
            return False
        
        # Check WIT files exist
        wit_world = self.get_wit_world_path()
        if not os.path.exists(wit_world):
            print(f"Error: WIT world file not found: {wit_world}", file=sys.stderr)
            print("This should be bundled with the SDK", file=sys.stderr)
            return False
        
        # Build componentize-py command
        cmd: List[str] = ["componentize-py"]
        
        # Use world.wit which includes all necessary imports
        cmd.extend(["-w", wit_world])
        
        # Check for componentize-py.toml config
        config_path = self.get_componentize_config()
        if os.path.exists(config_path):
            cmd.extend(["--config", config_path])
        
        # Add output and input
        cmd.extend(["-o", self.output, self.handler_file])
        
        # Show build info
        print(f"Building WebAssembly component...")
        print(f"  Handler: {self.handler_file}")
        print(f"  WIT: {wit_world}")
        print(f"  Output: {self.output}")
        
        if verbose:
            print(f"  Command: {' '.join(cmd)}")
        
        # Run componentize-py
        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=False
            )
            
            if verbose and result.stdout:
                print(result.stdout)
            
            if result.returncode != 0:
                print(f"Error: Build failed", file=sys.stderr)
                if result.stderr:
                    print(result.stderr, file=sys.stderr)
                return False
            
            # Verify output was created
            if not os.path.exists(self.output):
                print(f"Error: Output file was not created", file=sys.stderr)
                return False
            
            # Get file size for info
            size = os.path.getsize(self.output)
            size_kb = size / 1024
            
            print(f"✓ Successfully built {self.output} ({size_kb:.1f} KB)")
            return True
            
        except Exception as e:
            print(f"Error: Build failed with exception: {e}", file=sys.stderr)
            return False

def main():
    """CLI entry point."""
    parser = argparse.ArgumentParser(
        description="Build Python MCP handlers as WebAssembly components",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  wasmcp-build handler.py
  wasmcp-build handler.py -o my-handler.wasm
  wasmcp-build handler.py --verbose
  
This tool bundles all necessary WIT files and configurations
so you don't need to manage them manually.
"""
    )
    
    parser.add_argument(
        "handler",
        help="Python handler file to compile"
    )
    
    parser.add_argument(
        "-o", "--output",
        default="handler.wasm",
        help="Output WASM file (default: handler.wasm)"
    )
    
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Enable verbose output"
    )
    
    parser.add_argument(
        "--wit",
        help="Custom WIT directory (advanced users only)"
    )
    
    args = parser.parse_args()
    
    # Create builder
    builder = WasmcpBuilder(args.handler, args.output)
    
    # Override WIT path if provided
    if args.wit:
        builder.wit_dir = Path(args.wit)
    
    # Run build
    success = builder.build(verbose=args.verbose)
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()