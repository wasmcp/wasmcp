#!/usr/bin/env python3
"""Generate Python bindings from WIT files using componentize-py."""

import subprocess
import shutil
import sys
from pathlib import Path

def generate_bindings():
    """Generate Python bindings from WIT files."""
    # Define paths
    root_dir = Path(__file__).parent.parent
    wit_dir = root_dir / "wit"
    src_dir = root_dir / "src" / "wasmcp"
    final_bindings_dir = src_dir / "wit"
    
    print(f"📁 WIT directory: {wit_dir}")
    print(f"📁 Source directory: {src_dir}")
    print(f"📁 Final bindings location: {final_bindings_dir}")
    
    # Check if WIT directory exists
    if not wit_dir.exists():
        print(f"❌ WIT directory not found: {wit_dir}")
        print("Creating WIT directory structure...")
        wit_dir.mkdir(parents=True, exist_ok=True)
        return False
    
    # Clean existing bindings
    temp_bindings = src_dir / "wasmcp_wit"
    if temp_bindings.exists():
        print(f"🧹 Cleaning temporary bindings in {temp_bindings}")
        shutil.rmtree(temp_bindings)
    
    if final_bindings_dir.exists():
        print(f"🧹 Cleaning existing bindings in {final_bindings_dir}")
        shutil.rmtree(final_bindings_dir)
    
    # Check if componentize-py is installed
    if shutil.which("componentize-py") is None:
        print("❌ componentize-py not found. Install it with: pip install componentize-py")
        return False
    
    # Generate bindings using componentize-py
    print("🔨 Generating Python bindings from WIT files...")
    # componentize-py syntax: componentize-py -d <wit-dir> -w <world> bindings <output-module-name>
    cmd = [
        "componentize-py",
        "-d", str(wit_dir),
        "-w", "mcp-handler",
        "bindings",
        "wasmcp_wit"  # This will be the module name
    ]
    
    print(f"Running: {' '.join(cmd)}")
    
    try:
        # componentize-py generates bindings in the current directory
        # So we need to change to the src/wasmcp directory first
        import os
        original_dir = os.getcwd()
        os.chdir(src_dir)
        
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        print("✅ Bindings generated successfully!")
        if result.stdout:
            print(result.stdout)
        
        # Move the generated bindings to the final location
        if temp_bindings.exists():
            print(f"📦 Moving bindings from {temp_bindings} to {final_bindings_dir}")
            shutil.move(str(temp_bindings), str(final_bindings_dir))
            print("✅ Bindings installed to final location")
        
        os.chdir(original_dir)
        return True
    except subprocess.CalledProcessError as e:
        print(f"❌ Failed to generate bindings: {e}")
        if e.stderr:
            print(f"Error output: {e.stderr}")
        os.chdir(original_dir)
        return False
    except Exception as e:
        print(f"❌ Unexpected error: {e}")
        os.chdir(original_dir)
        return False

if __name__ == "__main__":
    success = generate_bindings()
    sys.exit(0 if success else 1)