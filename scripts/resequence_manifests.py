#!/usr/bin/env python3

import argparse
import re
import shutil
import sys
import uuid
from pathlib import Path
from typing import List, Optional


MANIFEST_PATTERN = re.compile(r"^(\d+)-(.*)$")


class ManifestEntry:
    def __init__(self, path: Optional[Path], suffix: str, kind: str):
        self.path = path
        self.suffix = suffix
        self.kind = kind  # 'existing' or 'insert'
        self.tmp_path: Optional[Path] = None
        self.final_path: Optional[Path] = None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Resequence manifests within a directory by moving an existing file "
            "or inserting a new manifest at a specific position."
        )
    )
    parser.add_argument(
        "directory",
        type=Path,
        help="Directory containing numbered manifests (e.g., manifests/base)",
    )
    action_group = parser.add_mutually_exclusive_group(required=True)
    action_group.add_argument(
        "--move",
        type=Path,
        help="Path to an existing manifest inside the target directory to reposition",
    )
    action_group.add_argument(
        "--insert",
        type=Path,
        help="Path to a manifest file that should be inserted into the directory",
    )
    parser.add_argument(
        "--position",
        type=int,
        required=True,
        help="1-based position where the manifest should appear after resequencing",
    )
    parser.add_argument(
        "--copy",
        action="store_true",
        help="Copy (instead of move) the manifest when using --insert",
    )
    return parser.parse_args()


def load_manifest_entries(directory: Path) -> List[ManifestEntry]:
    entries: List[ManifestEntry] = []
    for path in sorted(directory.glob("*.yaml")):
        match = MANIFEST_PATTERN.match(path.name)
        if not match:
            continue
        suffix = match.group(2)
        entries.append(ManifestEntry(path=path, suffix=suffix, kind="existing"))

    entries.sort(key=lambda entry: _manifest_sort_key(entry))
    return entries


def _manifest_sort_key(entry: ManifestEntry):
    match = MANIFEST_PATTERN.match(entry.path.name if entry.path else f"0-{entry.suffix}")
    number = int(match.group(1)) if match else 0
    return number, entry.suffix


def ensure_directory(path: Path) -> Path:
    resolved = path.resolve()
    if not resolved.is_dir():
        sys.exit(f"Directory '{path}' does not exist")
    return resolved


def ensure_existing_manifest(target: Path, directory: Path) -> Path:
    target_resolved = target.resolve()
    if not target_resolved.exists():
        sys.exit(f"Manifest '{target}' does not exist")
    if target_resolved.parent != directory:
        sys.exit(f"Manifest '{target}' must be inside '{directory}'")
    if not MANIFEST_PATTERN.match(target_resolved.name):
        sys.exit(
            f"Manifest '{target}' does not follow the '<number>-<name>.yaml' format"
        )
    return target_resolved


def ensure_insert_source(path: Path, directory: Path) -> Path:
    source = path.resolve()
    if not source.is_file():
        sys.exit(f"Manifest '{path}' does not exist or is not a file")
    if source.parent == directory and MANIFEST_PATTERN.match(source.name):
        sys.exit(
            "Source manifest already has a numeric prefix inside the target directory. "
            "Use --move instead."
        )
    return source


def determine_suffix(path: Path) -> str:
    match = MANIFEST_PATTERN.match(path.name)
    if match:
        return match.group(2)
    return path.name


def resequence_manifests(entries: List[ManifestEntry], directory: Path):
    for entry in entries:
        if entry.kind == "existing" and entry.path:
            temp_name = f".tmp_{uuid.uuid4().hex}_{entry.suffix}"
            entry.tmp_path = entry.path.with_name(temp_name)
            entry.path.rename(entry.tmp_path)


def apply_new_order(entries: List[ManifestEntry], directory: Path, copy_insert: bool):
    for index, entry in enumerate(entries, start=1):
        new_name = f"{index}-{entry.suffix}"
        destination = directory / new_name
        if entry.kind == "existing":
            assert entry.tmp_path is not None
            entry.tmp_path.rename(destination)
        else:
            assert entry.path is not None
            if copy_insert:
                shutil.copy2(entry.path, destination)
            else:
                shutil.move(entry.path, destination)
        entry.final_path = destination


def main():
    args = parse_args()
    directory = ensure_directory(args.directory)
    entries = load_manifest_entries(directory)

    if args.move:
        target = ensure_existing_manifest(args.move, directory)
        target_index = next(
            (idx for idx, entry in enumerate(entries) if entry.path == target), None
        )
        if target_index is None:
            sys.exit(f"Manifest '{target}' was not found in '{directory}'")

        entry = entries.pop(target_index)
        max_position = len(entries) + 1
        if not (1 <= args.position <= max_position):
            sys.exit(f"--position must be between 1 and {max_position}")
        entries.insert(args.position - 1, entry)
        operation_desc = f"Moved {target.name} to position {args.position}"

    else:
        source = ensure_insert_source(args.insert, directory)
        suffix = determine_suffix(source)
        new_entry = ManifestEntry(path=source, suffix=suffix, kind="insert")
        max_position = len(entries) + 1
        if not (1 <= args.position <= max_position):
            sys.exit(f"--position must be between 1 and {max_position}")
        entries.insert(args.position - 1, new_entry)
        action = "Copied" if args.copy else "Moved"
        operation_desc = (
            f"{action} {source.name} into {directory} at position {args.position}"
        )

    resequence_manifests(entries, directory)
    apply_new_order(entries, directory, copy_insert=args.copy)

    print(operation_desc)
    print("New manifest order:")
    for entry in entries:
        print(f"  {entry.final_path.name}")


if __name__ == "__main__":
    main()
