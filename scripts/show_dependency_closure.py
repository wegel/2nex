#!/usr/bin/env python3
"""
Display the transitive dependency closure encoded in OSTree metadata.

Each bundle/output the builder creates carries `nex.bundle.requires`
and/or `nex.output.requires`. This helper walks those metadata keys,
printing the entire dependency graph so it's easy to spot what will be
layered before a build.

Examples:
    # Show the closure for a single bundle using the default repo
    ./scripts/show_dependency_closure.py \
        x86_64/bash/5.2.21/bootstrap/phase3/bundles/dev

    # Point at a custom repo and emit JSON
    ./scripts/show_dependency_closure.py --repo /path/to/repo \
        --json asm/demo-minimal/0.1.0
"""

import argparse
import collections
import json
import subprocess
import sys
from typing import Dict, List

METADATA_KEYS = (
    "nex.bundle.requires",
    "nex.output.requires",
    "nex.system.packages",
    "nex.system.dependencies",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Show the dependency closure recorded on OSTree refs."
    )
    parser.add_argument(
        "refs",
        nargs="+",
        help="OSTree refs/commits to treat as graph roots.",
    )
    parser.add_argument(
        "--repo",
        default="bootstrap_store",
        help="Path to the OSTree repository (default: %(default)s)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Emit the graph as JSON instead of a human-readable tree.",
    )
    return parser.parse_args()


def read_metadata(repo: str, ref: str, key: str) -> List[str]:
    cmd = [
        "ostree",
        f"--repo={repo}",
        "show",
        f"--print-metadata-key={key}",
        ref,
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        stderr = proc.stderr.strip()
        if "No such metadata key" in stderr:
            return []
        raise RuntimeError(
            f"Failed to read metadata {key} from {ref}: {stderr}"
        )
    raw = proc.stdout.strip()
    if not raw:
        return []
    # Metadata values are stored as JSON arrays; handle both
    # single-line and multi-line output defensively.
    try:
        return [entry for entry in json.loads(raw) if entry]
    except json.JSONDecodeError:
        entries = []
        for line in raw.splitlines():
            line = line.strip()
            if not line:
                continue
            if (line.startswith("'") and line.endswith("'")) or (
                line.startswith('"') and line.endswith('"')
            ):
                line = line[1:-1]
            try:
                parsed = json.loads(line)
                if isinstance(parsed, list):
                    entries.extend(parsed)
                elif isinstance(parsed, str):
                    entries.append(parsed)
            except json.JSONDecodeError:
                entries.append(line)
        return [entry for entry in entries if entry]


def resolve_requires(repo: str, ref: str) -> List[str]:
    deps: List[str] = []
    for key in METADATA_KEYS:
        deps.extend(read_metadata(repo, ref, key))
    # Preserve order but drop duplicates
    seen = set()
    filtered = []
    for dep in deps:
        if dep and dep not in seen:
            seen.add(dep)
            filtered.append(dep)
    return filtered


def compute_closure(repo: str, roots: List[str]) -> Dict[str, List[str]]:
    graph: Dict[str, List[str]] = collections.defaultdict(list)
    queue = collections.deque(roots)
    visited = set()

    while queue:
        ref = queue.popleft()
        if ref in visited:
            continue
        visited.add(ref)
        try:
            deps = resolve_requires(repo, ref)
        except RuntimeError as exc:
            print(exc, file=sys.stderr)
            continue
        graph[ref] = deps
        for dep in deps:
            if dep not in visited:
                queue.append(dep)

    return graph


def print_tree(graph: Dict[str, List[str]]) -> None:
    for ref, deps in graph.items():
        print(ref)
        for dep in deps:
            print(f"  -> {dep}")


def main() -> None:
    args = parse_args()
    graph = compute_closure(args.repo, args.refs)
    if args.json:
        json.dump(graph, sys.stdout, indent=2)
        sys.stdout.write("\n")
    else:
        print_tree(graph)


if __name__ == "__main__":
    main()
