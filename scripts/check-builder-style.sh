#!/usr/bin/env bash
set -euo pipefail

python3 - "$@" <<'PY'
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path.cwd()
TARGETS = [
    ROOT / "src/cli/src/build",
    ROOT / "src/cli/src/system",
    ROOT / "src/cli/src/materializer",
    ROOT / "src/cli/src/outputs",
    ROOT / "src/cli/src/commands/build.rs",
]

MAX_FILE_LINES = 400
MAX_FUNCTION_LINES = 60


def target_files() -> list[Path]:
    files: list[Path] = []
    for target in TARGETS:
        if target.is_dir():
            files.extend(sorted(target.rglob("*.rs")))
        elif target.exists():
            files.append(target)
    return sorted(files)


def has_module_doc(lines: list[str]) -> bool:
    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        return stripped.startswith("//!")
    return False


def has_glob_import(line: str) -> bool:
    return bool(re.match(r"^\s*use\s+.*::\*\s*;", line))


def function_name(line: str) -> str | None:
    match = re.match(
        r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\b",
        line,
    )
    if not match:
        return None
    return match.group(1)


def brace_delta(line: str) -> int:
    code = line.split("//", 1)[0]
    return code.count("{") - code.count("}")


def function_lengths(path: Path, lines: list[str]) -> list[tuple[str, int, int]]:
    functions: list[tuple[str, int, int]] = []
    active_name: str | None = None
    start_line = 0
    brace_depth = 0
    waiting_for_body = False

    for index, line in enumerate(lines, start=1):
        if active_name is None:
            name = function_name(line)
            if name is None:
                continue
            active_name = name
            start_line = index
            brace_depth = brace_delta(line)
            waiting_for_body = "{" not in line
            if brace_depth <= 0 and not waiting_for_body:
                functions.append((active_name, start_line, index - start_line + 1))
                active_name = None
            continue

        brace_depth += brace_delta(line)
        if waiting_for_body and "{" in line:
            waiting_for_body = False
        if not waiting_for_body and brace_depth <= 0:
            functions.append((active_name, start_line, index - start_line + 1))
            active_name = None

    return functions


def has_inline_tests(lines: list[str]) -> bool:
    for index, line in enumerate(lines):
        if "#[cfg(test)]" not in line:
            continue
        for next_line in lines[index + 1 :]:
            stripped = next_line.strip()
            if not stripped:
                continue
            if re.match(r"mod\s+[A-Za-z_][A-Za-z0-9_]*\s*\{", stripped):
                return True
            break
    return False


def main() -> int:
    failures: list[str] = []

    for path in target_files():
        rel = path.relative_to(ROOT)
        lines = path.read_text(encoding="utf-8").splitlines()
        line_count = len(lines)

        if line_count > MAX_FILE_LINES:
            failures.append(f"{rel}: {line_count} lines exceeds {MAX_FILE_LINES}")

        if not path.name.endswith("_tests.rs") and not has_module_doc(lines):
            failures.append(f"{rel}: missing module doc comment")

        for index, line in enumerate(lines, start=1):
            if has_glob_import(line):
                failures.append(f"{rel}:{index}: glob import")

        if line_count >= 100 and has_inline_tests(lines):
            failures.append(f"{rel}: inline tests in module with {line_count} lines")

        for name, start, length in function_lengths(path, lines):
            if length > MAX_FUNCTION_LINES:
                failures.append(
                    f"{rel}:{start}: function {name} has {length} lines, exceeds {MAX_FUNCTION_LINES}"
                )

    if failures:
        print("builder style check failed:")
        for failure in failures:
            print(f"  - {failure}")
        return 1

    print("builder style check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
PY
