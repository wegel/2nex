#!/usr/bin/env python3
"""
Remove Duplicate Dependencies

This script removes duplicate dependencies from a YAML manifest file based on their commit values.
"""

import yaml
import os
import argparse
import sys
import shutil
from pathlib import Path

def parse_args():
    parser = argparse.ArgumentParser(description="Remove duplicate dependencies in YAML manifest files")
    parser.add_argument("manifest_file", help="Path to the manifest YAML file")
    parser.add_argument("--output", help="Path to save the deduplicated manifest file (default: original_name.deduped.yaml)")
    parser.add_argument("--overwrite", action="store_true",
                        help="Overwrite the original manifest file with the deduplicated version")
    parser.add_argument("--no-backup", action="store_true",
                        help="Don't create a backup of the original file")
    return parser.parse_args()

def load_yaml(file_path):
    """Load YAML file and return its content."""
    with open(file_path, 'r') as f:
        return yaml.safe_load(f)

def save_yaml(data, file_path):
    """Save data to a YAML file."""
    with open(file_path, 'w') as f:
        yaml.dump(data, f, default_flow_style=False)

def read_file_content(file_path):
    """Read the content of a file."""
    with open(file_path, 'r') as f:
        return f.read()

def write_file_content(file_path, content):
    """Write content to a file."""
    with open(file_path, 'w') as f:
        f.write(content)

def update_dependencies_in_file(file_path, dependencies):
    """Update only the dependencies section in the manifest file, preserving all other content."""
    # Load the original manifest to get its structure
    manifest = load_yaml(file_path)
    
    # Update only the dependencies
    manifest['dependencies'] = dependencies
    
    # Save to the same location
    save_yaml(manifest, file_path)

def remove_duplicate_dependencies(manifest_file):
    """Remove duplicate dependencies based on their commit value."""
    # Load the manifest
    manifest = load_yaml(manifest_file)
    
    # Extract the list of dependencies
    original_deps = manifest.get('dependencies', [])
    if not original_deps:
        print("No dependencies found in the manifest file.")
        return manifest, []
    
    print(f"Original dependency count: {len(original_deps)}")
    
    # Track unique dependencies by commit value
    unique_deps = {}
    removed_deps = []
    
    for dep in original_deps:
        # Get the commit value, use an empty string if not present
        commit = dep.get('commit', '')
        
        if commit:
            if commit not in unique_deps:
                # First time seeing this commit, add it to unique_deps
                unique_deps[commit] = dep
            else:
                # Duplicate commit, add to removed_deps
                removed_deps.append(dep)
                print(f"Found duplicate dependency with commit: {commit}")
                print(f"  Keeping: {unique_deps[commit].get('name', 'unnamed')}")
                print(f"  Removing: {dep.get('name', 'unnamed')}")
        else:
            # Dependencies without commit values are kept as is
            # But we'll use a unique identifier to avoid overriding
            unique_id = id(dep)
            unique_deps[f"no_commit_{unique_id}"] = dep
    
    # Convert the dictionary back to a list
    deduplicated_deps = list(unique_deps.values())
    
    print(f"\nDeduplication complete.")
    print(f"Original dependencies: {len(original_deps)}")
    print(f"Deduplicated dependencies: {len(deduplicated_deps)}")
    print(f"Removed duplicates: {len(removed_deps)}")
    
    return deduplicated_deps, removed_deps

def main():
    args = parse_args()
    
    manifest_file = Path(args.manifest_file)
    if not manifest_file.exists():
        print(f"Error: Manifest file {manifest_file} does not exist.")
        return 1
    
    # Handle output file path
    if args.overwrite:
        output_file = str(manifest_file)
    elif args.output:
        output_file = args.output
    else:
        output_file = manifest_file.with_suffix(f".deduped{manifest_file.suffix}")
    
    # Create a backup of the original file
    if not args.no_backup:
        backup_file = manifest_file.with_suffix(f".backup{manifest_file.suffix}")
        shutil.copy2(manifest_file, backup_file)
        print(f"Created backup of original file: {backup_file}")
    
    # Remove duplicate dependencies
    print(f"Analyzing manifest: {manifest_file}")
    
    try:
        deduplicated_deps, removed_deps = remove_duplicate_dependencies(manifest_file)
        
        # Update the manifest file with the deduplicated dependencies
        if args.overwrite:
            update_dependencies_in_file(manifest_file, deduplicated_deps)
            print(f"Original manifest file has been updated with deduplicated dependencies")
        else:
            # For a new output file, create a copy first
            shutil.copy2(manifest_file, output_file)
            update_dependencies_in_file(output_file, deduplicated_deps)
            print(f"Saved deduplicated manifest to: {output_file}")
        
        return 0
    except Exception as e:
        print(f"\nError: {e}")
        return 1

if __name__ == "__main__":
    exit(main())