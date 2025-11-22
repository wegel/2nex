#!/usr/bin/env python3
"""
Dump the dependency refs referenced by a system manifest before building it.

This reads `packages` and `dependencies` from the YAML manifest, then invokes
`show_dependency_closure.py` so you can see the entire transitive graph without
needing the system commit to exist yet.

Example:
    ./scripts/show_system_dependencies.py asm/demo-minimal.yaml
"""

import argparse
import subprocess
import sys
import yaml


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Show the dependency closure for a system manifest."
    )
    parser.add_argument("manifest", help="Path to the system manifest YAML file.")
    parser.add_argument(
        "--repo",
        default="bootstrap_store",
        help="Path to the OSTree repository (default: %(default)s)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit JSON instead of a tree (forwarded to show_dependency_closure.py).",
    )
    parser.add_argument(
        "--closure-script",
        default="scripts/show_dependency_closure.py",
        help="Path to the closure helper (default: %(default)s).",
    )
    return parser.parse_args()


def collect_refs(manifest_path: str) -> list[str]:
    with open(manifest_path, "r", encoding="utf-8") as handle:
        doc = yaml.safe_load(handle)
    packages = [entry["commit"] for entry in doc.get("packages", []) if "commit" in entry]
    dependencies = [
        entry["commit"] for entry in doc.get("dependencies", []) if "commit" in entry
    ]
    if not packages and not dependencies:
        raise SystemExit("Manifest has no packages or dependencies to inspect.")
    return packages + dependencies


def main() -> None:
    args = parse_args()
    refs = collect_refs(args.manifest)
    cmd = [args.closure_script, f"--repo={args.repo}"]
    if args.json:
        cmd.append("--json")
    cmd.extend(refs)
    subprocess.run(cmd, check=True)


if __name__ == "__main__":
    main()
