#!/usr/bin/env python3
"""
Synchronize versions across the ftl-components repository.
Reads versions from versions.toml and updates all references.
"""

import os
import sys
import toml
import json
import re
from pathlib import Path

# Get the repository root
REPO_ROOT = Path(__file__).parent.parent

def load_versions():
    """Load versions from versions.toml"""
    versions_file = REPO_ROOT / "versions.toml"
    if not versions_file.exists():
        print(f"Error: {versions_file} not found")
        sys.exit(1)
    return toml.load(versions_file)

def update_cargo_toml(file_path, package_name, new_version):
    """Update version in a Cargo.toml file"""
    with open(file_path, 'r') as f:
        content = f.read()
    
    # Update package version
    if f'name = "{package_name}"' in content:
        content = re.sub(
            r'(name = "' + package_name + r'".*?\n(?:.*\n)*?)version = "[^"]*"',
            r'\1version = "' + new_version + '"',
            content,
            flags=re.MULTILINE
        )
        
        with open(file_path, 'w') as f:
            f.write(content)
        print(f"Updated {file_path}: {package_name} = {new_version}")

def update_package_json(file_path, package_name, new_version):
    """Update version in a package.json file"""
    with open(file_path, 'r') as f:
        data = json.load(f)
    
    if data.get('name') == package_name:
        data['version'] = new_version
        
        with open(file_path, 'w') as f:
            json.dump(data, f, indent=2)
            f.write('\n')
        print(f"Updated {file_path}: {package_name} = {new_version}")

def update_template_dependencies(versions):
    """Update dependency versions in templates"""
    templates_dir = REPO_ROOT / "templates"
    
    # Update Rust template
    rust_cargo = templates_dir / "rust/content/handler/Cargo.toml"
    if rust_cargo.exists():
        with open(rust_cargo, 'r') as f:
            content = f.read()
        
        # Update ftl-sdk version
        content = re.sub(
            r'ftl-sdk = "[^"]*"',
            f'ftl-sdk = "{versions["packages"]["ftl-sdk-rust"]}"',
            content
        )
        
        with open(rust_cargo, 'w') as f:
            f.write(content)
        print(f"Updated Rust template: ftl-sdk = {versions['packages']['ftl-sdk-rust']}")
    
    # Update JavaScript/TypeScript templates
    for lang in ["javascript", "typescript"]:
        package_json = templates_dir / f"{lang}/content/handler/package.json"
        if package_json.exists():
            with open(package_json, 'r') as f:
                data = json.load(f)
            
            # Update @fastertools/ftl-sdk version
            if 'dependencies' in data and '@fastertools/ftl-sdk' in data['dependencies']:
                data['dependencies']['@fastertools/ftl-sdk'] = f"^{versions['packages']['ftl-sdk-typescript']}"
                
                with open(package_json, 'w') as f:
                    json.dump(data, f, indent=2)
                    f.write('\n')
                print(f"Updated {lang} template: @fastertools/ftl-sdk = ^{versions['packages']['ftl-sdk-typescript']}")

def update_spin_toml_references(versions):
    """Update mcp-gateway references in spin.toml files"""
    gateway_version = versions['registry']['ghcr.io/bowlofarugula/mcp-http-component']
    
    # Update templates
    for template_dir in (REPO_ROOT / "templates").iterdir():
        if template_dir.is_dir():
            spin_toml = template_dir / "content/spin.toml"
            if spin_toml.exists():
                with open(spin_toml, 'r') as f:
                    content = f.read()
                
                # Update mcp-gateway version
                content = re.sub(
                    r'(bowlofarugula:mcp-gateway[^"]*version = ")[^"]*(")',
                    f'\\g<1>{gateway_version}\\g<2>',
                    content
                )
                
                with open(spin_toml, 'w') as f:
                    f.write(content)
                print(f"Updated {spin_toml}: mcp-gateway = {gateway_version}")
            
            # Also update component.txt snippet
            snippet = template_dir / "metadata/snippets/component.txt"
            if snippet.exists():
                with open(snippet, 'r') as f:
                    content = f.read()
                
                content = re.sub(
                    r'(bowlofarugula:mcp-gateway[^"]*version = ")[^"]*(")',
                    f'\\g<1>{gateway_version}\\g<2>',
                    content
                )
                
                with open(snippet, 'w') as f:
                    f.write(content)
                print(f"Updated {snippet}: mcp-gateway = {gateway_version}")

def update_wit_versions(versions):
    """Update WIT package versions"""
    wit_version = versions['wit']['mcp']
    
    # Update main WIT file
    mcp_wit = REPO_ROOT / "wit/mcp.wit"
    if mcp_wit.exists():
        with open(mcp_wit, 'r') as f:
            content = f.read()
        
        content = re.sub(
            r'package component:mcp@[0-9.]+',
            f'package component:mcp@{wit_version}',
            content
        )
        
        with open(mcp_wit, 'w') as f:
            f.write(content)
        print(f"Updated WIT package version: component:mcp@{wit_version}")

def validate_versions():
    """Validate that all versions are in sync"""
    versions = load_versions()
    errors = []
    
    # Check package versions match
    cargo_files = [
        (REPO_ROOT / "src/mcp-http-component/Cargo.toml", "mcp-http-component"),
        (REPO_ROOT / "src/ftl-sdk-rust/Cargo.toml", "ftl-sdk"),
    ]
    
    for file_path, package_name in cargo_files:
        if file_path.exists():
            with open(file_path, 'r') as f:
                content = f.read()
            
            # Extract version
            match = re.search(r'version = "([^"]*)"', content)
            if match:
                actual_version = match.group(1)
                expected_key = package_name.replace('-', '_')
                if expected_key == "ftl_sdk":
                    expected_key = "ftl-sdk-rust"
                expected_version = versions['packages'].get(expected_key)
                
                if actual_version != expected_version:
                    errors.append(f"{file_path}: version {actual_version} != expected {expected_version}")
    
    if errors:
        print("\nVersion mismatches found:")
        for error in errors:
            print(f"  - {error}")
        return False
    else:
        print("\nAll versions are in sync! ✓")
        return True

def main():
    """Main entry point"""
    if len(sys.argv) > 1 and sys.argv[1] == "validate":
        sys.exit(0 if validate_versions() else 1)
    
    versions = load_versions()
    
    print("Synchronizing versions across ftl-components...\n")
    
    # Update component versions
    update_cargo_toml(
        REPO_ROOT / "src/mcp-http-component/Cargo.toml",
        "mcp-http-component",
        versions['packages']['mcp-http-component']
    )
    
    update_cargo_toml(
        REPO_ROOT / "src/ftl-sdk-rust/Cargo.toml",
        "ftl-sdk",
        versions['packages']['ftl-sdk-rust']
    )
    
    update_package_json(
        REPO_ROOT / "src/ftl-sdk-typescript/package.json",
        "@fastertools/ftl-sdk",
        versions['packages']['ftl-sdk-typescript']
    )
    
    # Update templates
    update_template_dependencies(versions)
    update_spin_toml_references(versions)
    
    # Update WIT versions
    update_wit_versions(versions)
    
    print("\nVersion sync complete!")
    
    # Run validation
    validate_versions()

if __name__ == "__main__":
    main()