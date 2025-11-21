#!/usr/bin/env python3
"""
Dependency Minimizer

This script minimizes the dependencies in a YAML manifest file by systematically
testing which dependencies are truly necessary for building the manifest.
It also automatically removes duplicate dependencies based on their commit.

Examples:
    # Full minimization with deduplication
    python3 dependency-minimizer.py manifests/base/24-podman.yaml

    # Only deduplicate dependencies by commit without testing
    python3 dependency-minimizer.py manifests/base/24-podman.yaml --only-deduplicate

    # Minimization without deduplication
    python3 dependency-minimizer.py manifests/base/24-podman.yaml --skip-deduplication
    
    # Overwrite the original file with minimized dependencies
    python3 dependency-minimizer.py manifests/base/24-podman.yaml --overwrite
    
    # Ignore specific dependencies during minimization
    python3 dependency-minimizer.py manifests/base/24-podman.yaml --ignore gmp glibc gcc
    
    # Specify custom builder path and output file
    python3 dependency-minimizer.py manifests/base/24-podman.yaml --builder ./nex --output podman-minimal.yaml
"""

import argparse
import copy
import json
import os
import subprocess
import tempfile
import yaml
import time
import shutil
import signal
import sys
import re
from pathlib import Path


# Global variables to track the current build process and cancellation
current_process = None
cancelled = False


# Signal handler for manual cancellation
def signal_handler(sig, frame):
    global current_process, cancelled
    if current_process:
        print("\n\nManually cancelling current build...")
        current_process.terminate()
        cancelled = True
    else:
        print("\n\nExiting script...")
        sys.exit(0)


def parse_args():
    parser = argparse.ArgumentParser(description="Minimize dependencies or packages in YAML manifest files")
    parser.add_argument("manifest_file", help="Path to the manifest YAML file")
    parser.add_argument("--builder", default="src/builder/target/debug/nex",
                        help="Path to the builder executable (default: src/builder/target/debug/nex)")
    parser.add_argument("--output", help="Path to save the minimized manifest file (default: original_name.minimized.yaml)")
    parser.add_argument("--overwrite", action="store_true",
                        help="Overwrite the original manifest file with the minimized version")
    parser.add_argument("--parallel", type=int, default=1, 
                        help="Number of dependencies to test removing in parallel (default: 1)")
    parser.add_argument("--timeout", type=int, default=600,
                        help="Timeout in seconds for each build attempt (default: 600)")
    parser.add_argument("--no-backup", action="store_true",
                        help="Don't create a backup of the original file")
    parser.add_argument("--no-removed-list", action="store_true",
                        help="Don't save a list of removed dependencies")
    parser.add_argument("--ignore", nargs="*", default=["gmp", "glibc"],
                        help="List of dependency names to ignore during minimization (default: gmp glibc)")
    parser.add_argument("--skip-deduplication", action="store_true",
                        help="Skip the commit-based dependency deduplication step")
    parser.add_argument("--only-deduplicate", action="store_true",
                        help="Only perform commit-based deduplication without minimization testing")
    parser.add_argument("--hydrate-dependencies-first", action="store_true",
                        help="Hydrate dependencies (expand transitive deps) before minimization")
    parser.add_argument(
        "--section",
        choices=["dependencies", "packages"],
        default="dependencies",
        help="Manifest section to minimize (default: dependencies)",
    )
    parser.add_argument("--repo", default="bootstrap_store",
                        help="OSTree repo path for hydration (default: bootstrap_store)")
    return parser.parse_args()


def load_yaml(file_path):
    """Load YAML file and return its content."""
    with open(file_path, 'r') as f:
        return yaml.safe_load(f)


def save_yaml(data, file_path):
    """Save data to a YAML file."""
    with open(file_path, 'w') as f:
        yaml.dump(data, f, default_flow_style=False)


def get_manifest_entries(manifest, section):
    """Return entries for the requested section."""
    section_data = manifest.get(section, [])
    if section_data is None:
        return []
    return section_data


def read_file_content(file_path):
    """Read the content of a file."""
    with open(file_path, 'r') as f:
        return f.read()


def write_file_content(file_path, content):
    """Write content to a file."""
    with open(file_path, 'w') as f:
        f.write(content)


def update_section_in_file(file_path, section, entries):
    """Update only the chosen section in the manifest file, preserving other content."""
    # Read the original file content
    content = read_file_content(file_path)
    
    # Convert dependencies to YAML format while preserving indentation
    deps_yaml = yaml.dump(entries, default_flow_style=False)
    
    # Find the dependencies section using regex
    pattern = rf'({section}:)(\s*-.*?(?=\n\w|\Z))'
    
    if re.search(pattern, content, re.DOTALL):
        # If found, replace only that section with the new dependencies
        new_content = re.sub(pattern, r'\1\n' + deps_yaml, content, flags=re.DOTALL)
    else:
        # If dependencies section is not found or has different format
        # Find any key called 'dependencies' and replace its value
        pattern = rf'({section}:).*?(\n\w+:|$)'
        if re.search(pattern, content, re.DOTALL):
            new_content = re.sub(pattern, r'\1\n' + deps_yaml + r'\2', content, flags=re.DOTALL)
        else:
            # If dependencies key is not found at all, we can't update correctly
            raise ValueError("Could not locate 'dependencies' section in the manifest file.")
    
    # Write the updated content back to the file
    write_file_content(file_path, new_content)


def create_temp_manifest(original_file, temp_path, entries, section):
    """Create a temporary manifest file with modified dependencies but original structure."""
    # Load the original manifest to get its structure
    manifest = load_yaml(original_file)
    
    # Update only the dependencies
    manifest[section] = entries
    
    # Save to the temporary location
    save_yaml(manifest, temp_path)


def test_build(manifest_file, builder_path, timeout):
    """Test if the manifest can be built with the current dependencies."""
    global current_process, cancelled
    
    if cancelled:
        return False, "", "Build cancelled by user"
        
    build_dir = "build_rootfs"
    
    # Remove build directory if it exists
    if os.path.exists(build_dir):
        shutil.rmtree(build_dir)
    
    # Run the build command with --single --force to avoid cache issues
    cmd = [builder_path, "bootstrap_store", "--single", "--force", manifest_file]
    try:
        current_process = subprocess.Popen(
            cmd, 
            stdout=subprocess.PIPE, 
            stderr=subprocess.PIPE, 
            text=True
        )
        
        print(f"Build started (PID: {current_process.pid}). Press Ctrl+C to cancel this build.")
        
        try:
            stdout, stderr = current_process.communicate(timeout=timeout)
            returncode = current_process.returncode
            current_process = None
            return returncode == 0, stdout, stderr
        except subprocess.TimeoutExpired:
            current_process.terminate()
            current_process = None
            print(f"Build timed out after {timeout} seconds")
            return False, "", "Build timed out"
            
    except Exception as e:
        if current_process:
            current_process.terminate()
            current_process = None
        print(f"Error during build: {e}")
        return False, "", f"Build error: {e}"


def deduplicate_dependencies(dependencies):
    """
    Remove duplicate dependencies based on their commit path.
    Returns the deduplicated list and a list of removed dependencies.
    """
    unique_deps = {}
    removed_deps = []

    for dep in dependencies:
        # use commit as the unique identifier
        commit = dep.get('commit', '')

        if commit:
            if commit not in unique_deps:
                # first time seeing this commit, add it
                unique_deps[commit] = dep
            else:
                # duplicate commit, add to removed_deps
                removed_deps.append(dep)
                name = dep.get('name', 'unnamed')
                print(f"Found duplicate dependency: {name}")
                print(f"  Commit: {commit}")
        else:
            # dependencies without commits are kept as is
            unique_id = id(dep)
            unique_deps[f"no_commit_{unique_id}"] = dep

    # convert the dictionary back to a list
    deduplicated_deps = list(unique_deps.values())

    if removed_deps:
        print(f"\nDeduplication by commit complete.")
        print(f"Original dependencies: {len(dependencies)}")
        print(f"Deduplicated dependencies: {len(deduplicated_deps)}")
        print(f"Removed duplicates: {len(removed_deps)}")

    return deduplicated_deps, removed_deps


def minimize_dependencies(
    manifest_file,
    builder_path,
    parallel=1,
    timeout=600,
    ignore_deps=None,
    section="dependencies",
):
    """Find the minimal set of dependencies required for building the manifest."""
    global cancelled
    
    # Initialize the list of dependencies to ignore
    if ignore_deps is None:
        ignore_deps = []
    
    # Load the original manifest
    manifest = load_yaml(manifest_file)
    
    # Extract the list of dependencies
    original_deps = get_manifest_entries(manifest, section)
    if not original_deps:
        print(f"No {section} found in the manifest file.")
        return manifest, []
    
    print(f"Original {section} count: {len(original_deps)}")
    entry_label = section[:-1] if section.endswith("s") else section
    
    # First deduplicate dependencies by commit
    print(f"\nDeduplicating {section} by commit...")
    deduplicated_deps, name_removed_deps = deduplicate_dependencies(original_deps)
    
    # Create a working directory for all temporary manifest files
    with tempfile.TemporaryDirectory() as main_temp_dir:
        # Verify that the deduplicated manifest builds successfully
        print("\nTesting deduplicated manifest build...")
        
        deduped_manifest_path = os.path.join(main_temp_dir, "deduplicated_manifest.yaml")
        create_temp_manifest(manifest_file, deduped_manifest_path, deduplicated_deps, section)
        success, stdout, stderr = test_build(deduped_manifest_path, builder_path, timeout)
        
        if not success:
            print("Error: Deduplicated manifest failed to build. Cannot proceed with minimization.")
            print(f"Build error: {stderr}")
            # Save the problematic deduplicated manifest for debugging
            debug_file = Path(manifest_file).with_suffix(f".dedup_failed{Path(manifest_file).suffix}")
            shutil.copy2(deduped_manifest_path, debug_file)
            print(f"Saved problematic deduplicated manifest to: {debug_file}")
            return None, name_removed_deps
        
        # Use the deduplicated dependencies for minimization
        current_deps = deduplicated_deps
        
        # Track which dependencies can be removed through minimization
        removable_deps = []

        # Start with all deps, progressively remove as we find removable ones
        working_deps = list(current_deps)

        # Test each dependency individually
        for i, dep in enumerate(current_deps):
            dep_name = dep.get('name', f"unknown-{i}")

            # Skip if this dependency is in the ignore list
            if dep_name in ignore_deps:
                print(f"\nSkipping {entry_label} {dep_name} (in ignore list)")
                continue

            # Skip if already removed in a previous iteration
            if dep not in working_deps:
                print(f"\nSkipping {entry_label} {dep_name} (already removed)")
                continue

            print(f"\nTesting removal of {entry_label}: {dep_name} ({i+1}/{len(current_deps)})")

            # Create a test dependencies list without the current dependency
            test_deps = [d for d in working_deps if d != dep]
            
            # Create a unique temporary manifest file for this test
            test_manifest_path = os.path.join(main_temp_dir, f"test_manifest_{i}.yaml")
            create_temp_manifest(manifest_file, test_manifest_path, test_deps, section)
            
            # Test if it builds
            success, stdout, stderr = test_build(test_manifest_path, builder_path, timeout)
            print(f"DEBUG: success={success}, stderr_tail={stderr[-200:] if stderr else 'empty'}")

            # Reset cancelled flag after each test
            if cancelled:
                print(f"⚠️ Build for {dep_name} was cancelled by user, marking as required")
                cancelled = False
                success = False
            
            if success:
                print(f"✅ {entry_label.capitalize()} {dep_name} can be removed!")
                removable_deps.append(dep)
                # progressively remove from working set
                working_deps = test_deps
            else:
                print(f"❌ {entry_label.capitalize()} {dep_name} is required")
                
            # Optional: give user a moment to review results
            time.sleep(0.5)
        
        # Use the progressively reduced working list as the minimized set
        minimized_deps = working_deps
        
        # Verify the final minimized manifest before returning
        print("\nVerifying the final minimized manifest...")
        final_manifest_path = os.path.join(main_temp_dir, "final_minimized_manifest.yaml")
        create_temp_manifest(manifest_file, final_manifest_path, minimized_deps, section)
        success, stdout, stderr = test_build(final_manifest_path, builder_path, timeout)
        
        if not success:
            print("Error: The final minimized manifest failed to build!")
            print(f"Build error: {stderr}")
            # Save the problematic minimized manifest for debugging
            debug_file = Path(manifest_file).with_suffix(f".min_failed{Path(manifest_file).suffix}")
            shutil.copy2(final_manifest_path, debug_file)
            print(f"Saved problematic minimized manifest to: {debug_file}")
            print(f"\nReturning only deduplicated {section} instead of fully minimized ones.")
            return deduplicated_deps, name_removed_deps
    
    # Combine the dependencies removed in both stages
    total_removed_deps = name_removed_deps + removable_deps
    
    print(f"\nMinimization complete.")
    print(f"Original {section}: {len(original_deps)}")
    print(f"After commit deduplication: {len(current_deps)}")
    print(f"{entry_label.capitalize()} entries tested: {len(current_deps) - len(ignore_deps)}")
    print(f"{entry_label.capitalize()} entries ignored: {len(ignore_deps)}")
    print(f"Final minimized {section}: {len(minimized_deps)}")
    print(f"Total removed {section}: {len(total_removed_deps)}")
    
    # Show a summary of what's been removed
    if name_removed_deps:
        print(f"\nDuplicate {section} removed by name:")
        for dep in name_removed_deps:
            print(f"  - {dep.get('name', str(dep))}: {dep.get('commit', 'no commit')}")
            
    if removable_deps:
        print(f"\nRemoved {section} entries:")
        for dep in removable_deps:
            print(f"  - {dep.get('name', str(dep))}: {dep.get('commit', 'no commit')}")
    
    # Show ignored dependencies
    if ignore_deps:
        print(f"\n{section.capitalize()} that were ignored:")
        for name in ignore_deps:
            print(f"  - {name}")
    
    return minimized_deps, total_removed_deps


def hydrate_dependencies(manifest_file, builder_path, repo_path):
    """Hydrate dependencies by expanding transitive deps using the builder."""
    print(f"\nHydrating dependencies...")
    cmd = [builder_path, repo_path, "--hydrate-dependencies", manifest_file]
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=60
        )
        if result.returncode != 0:
            print(f"Error hydrating dependencies: {result.stderr}")
            return False
        print(result.stdout.strip())
        return True
    except Exception as e:
        print(f"Error during hydration: {e}")
        return False


def only_deduplicate_dependencies(manifest_file, section):
    """Deduplicate dependencies by name without running any builds."""
    # Load the manifest
    manifest = load_yaml(manifest_file)
    
    # Extract the list of dependencies
    original_deps = get_manifest_entries(manifest, section)
    if not original_deps:
        print(f"No {section} found in the manifest file.")
        return None, []
    
    print(f"Original {section} count: {len(original_deps)}")
    
    # Deduplicate dependencies by name
    print(f"\nDeduplicating {section} by name...")
    deduplicated_deps, removed_deps = deduplicate_dependencies(original_deps)
    
    return deduplicated_deps, removed_deps


def main():
    global cancelled
    
    # Set up signal handler for Ctrl+C
    signal.signal(signal.SIGINT, signal_handler)
    
    args = parse_args()
    
    manifest_file = Path(args.manifest_file)
    if not manifest_file.exists():
        print(f"Error: Manifest file {manifest_file} does not exist.")
        return 1
    
    # Determine final output file path
    if args.overwrite:
        final_output_file = str(manifest_file)
    elif args.output:
        final_output_file = args.output
    else:
        final_output_file = manifest_file.with_suffix(f".minimized{manifest_file.suffix}")
    
    # Create a backup of the original file
    if not args.no_backup:
        backup_file = manifest_file.with_suffix(f".backup{manifest_file.suffix}")
        shutil.copy2(manifest_file, backup_file)
        print(f"Created backup of original file: {backup_file}")
    
    # Always work with a temporary copy of the manifest file
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_manifest_path = os.path.join(temp_dir, "working_manifest.yaml")
        # Create a temp copy of the original manifest
        shutil.copy2(manifest_file, temp_manifest_path)
        
        start_time = time.time()
        try:
            # Hydrate dependencies first if requested
            if args.hydrate_dependencies_first:
                if not hydrate_dependencies(temp_manifest_path, args.builder, args.repo):
                    print("Failed to hydrate dependencies. Aborting.")
                    return 1

            # Determine operation mode based on flags
            if args.only_deduplicate:
                print(f"Only deduplicating dependencies in manifest: {manifest_file}")
                processed_deps, removed_deps = only_deduplicate_dependencies(
                    temp_manifest_path, args.section
                )
            else:
                # Standard minimization (with or without deduplication)
                print(f"Analyzing manifest: {manifest_file}")
                print(f"Using builder: {args.builder}")
                if args.ignore:
                    print(f"Ignoring dependencies: {', '.join(args.ignore)}")
                if args.overwrite:
                    print(f"Original manifest will be overwritten with minimized version")
                if args.skip_deduplication:
                    print(f"Skipping name-based deduplication step")
                print(f"Target section: {args.section}")
                print(f"Press Ctrl+C during a build to cancel that specific build and mark the dependency as required.")
                print(f"Press Ctrl+C twice in rapid succession to exit the script completely.")
                
                # Load manifest to modify for skip_deduplication case
                if args.skip_deduplication:
                    # Run minimization without deduplication
                    # We temporarily monkey patch the deduplicate_dependencies function to just pass through
                    original_deduplicate_func = deduplicate_dependencies
                    
                    def bypass_deduplication(deps):
                        return deps, []
                    
                    # Swap the function temporarily
                    globals()['deduplicate_dependencies'] = bypass_deduplication
                    
                    processed_deps, removed_deps = minimize_dependencies(
                        temp_manifest_path,
                        args.builder,
                        args.parallel,
                        args.timeout,
                        args.ignore,
                        args.section,
                    )
                    
                    # Restore the original function
                    globals()['deduplicate_dependencies'] = original_deduplicate_func
                else:
                    # Normal minimization with deduplication
                    processed_deps, removed_deps = minimize_dependencies(
                        temp_manifest_path,
                        args.builder,
                        args.parallel,
                        args.timeout,
                        args.ignore,
                        args.section,
                    )
            
            # Reset cancelled flag after processing
            cancelled = False
            
            elapsed_time = time.time() - start_time
            
            # Skip saving if no results were returned (error case)
            if processed_deps is None:
                print("\nNo changes were made to the manifest.")
                return 1
            
            # Generate a deduped manifest for verification
            deduped_manifest_path = os.path.join(temp_dir, "deduped_manifest.yaml")
            # Make a fresh copy of the original to work with
            shutil.copy2(manifest_file, deduped_manifest_path)
            # Update only the dependencies section
            update_section_in_file(deduped_manifest_path, args.section, processed_deps)
            
            # Perform a verification build before saving the final result
            print("\nVerifying final manifest with updated dependencies...")
            success, stdout, stderr = test_build(deduped_manifest_path, args.builder, args.timeout)
            
            if not success:
                print("\nError: The minimized manifest failed to build!")
                print("Original manifest was NOT modified.")
                print(f"Build error: {stderr}")
                
                # Save the problematic manifest for debugging
                debug_file = manifest_file.with_suffix(f".debug{manifest_file.suffix}")
                shutil.copy2(deduped_manifest_path, debug_file)
                print(f"Saved problematic manifest to: {debug_file}")
                return 1
            
            print("\nVerification build succeeded! Saving final manifest...")
            
            # Now it's safe to update the final output file
            if args.overwrite or args.output:
                # Copy the verified manifest to the final destination
                shutil.copy2(deduped_manifest_path, final_output_file)
                if args.overwrite:
                    print(f"Original manifest file has been updated with processed {args.section}")
                else:
                    print(f"Saved processed manifest to: {final_output_file}")
            else:
                # Copy the verified manifest to the default output location
                shutil.copy2(deduped_manifest_path, final_output_file)
                print(f"Saved processed manifest to: {final_output_file}")

            # Save the list of removed dependencies for reference
            if not args.no_removed_list and removed_deps:
                removed_deps_file = Path(final_output_file).with_suffix(".removed_deps.json")
                with open(removed_deps_file, 'w') as f:
                    json.dump([{
                        "name": d.get('name', "unknown"),
                        "commit": d.get('commit', "unknown")
                    } for d in removed_deps], f, indent=2)
                print(f"Saved list of removed {args.section} entries to: {removed_deps_file}")
            
            print(f"\nProcess completed in {elapsed_time:.2f} seconds")
            return 0
        except KeyboardInterrupt:
            print("\nScript terminated by user.")
            return 1
        except ValueError as e:
            print(f"\nError: {e}")
            return 1


if __name__ == "__main__":
    exit(main())
