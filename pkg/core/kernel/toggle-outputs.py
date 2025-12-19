#!/usr/bin/env python3
"""
toggle-outputs.py - prepare linux.yaml for output regeneration

Usage:
  ./toggle-outputs.py prepare   # comment out bundles, delete outputs, add blank sections
  ./toggle-outputs.py restore   # restore bundles, remove blank sections
"""
import sys
import re

YAML_FILE = "pkg/core/kernel/linux.yaml"
MARKER_START = "# --- ORIGINAL BUNDLES (commented out for regeneration) ---"
MARKER_END = "# --- END ORIGINAL BUNDLES ---"

def prepare():
    with open(YAML_FILE, 'r') as f:
        content = f.read()

    # find bundles section (starts with "bundles:" at column 0)
    bundles_match = re.search(r'^bundles:\n', content, re.MULTILINE)
    if not bundles_match:
        print("Error: bundles: section not found")
        sys.exit(1)

    bundles_start = bundles_match.start()

    # find outputs section
    outputs_match = re.search(r'^outputs:\n', content, re.MULTILINE)
    if not outputs_match:
        print("Error: outputs: section not found")
        sys.exit(1)

    outputs_start = outputs_match.start()

    # extract bundles section (from bundles: to outputs:)
    bundles_section = content[bundles_start:outputs_start]

    # comment out bundles section
    commented_bundles = MARKER_START + "\n"
    for line in bundles_section.rstrip().split('\n'):
        commented_bundles += "# " + line + "\n"
    commented_bundles += MARKER_END + "\n\n"

    # build new content: everything before bundles + commented bundles + blank sections
    new_content = content[:bundles_start]
    new_content += commented_bundles
    new_content += "bundles:\n\noutputs:\n"

    with open(YAML_FILE, 'w') as f:
        f.write(new_content)

    print(f"Prepared {YAML_FILE} for output regeneration")
    print("- Original bundles commented out")
    print("- Outputs section removed")
    print("- Blank bundles/outputs added")

def restore():
    with open(YAML_FILE, 'r') as f:
        content = f.read()

    # find the marker section
    marker_start_match = re.search(re.escape(MARKER_START) + r'\n', content)
    if not marker_start_match:
        print("Error: marker not found - was prepare run?")
        sys.exit(1)

    marker_end_match = re.search(re.escape(MARKER_END) + r'\n', content)
    if not marker_end_match:
        print("Error: end marker not found")
        sys.exit(1)

    # extract commented bundles and uncomment
    commented_section = content[marker_start_match.end():marker_end_match.start()]
    bundles_section = ""
    for line in commented_section.split('\n'):
        if line.startswith("# "):
            bundles_section += line[2:] + "\n"
        elif line == "#":
            bundles_section += "\n"

    # find the blank bundles: line (should be right after marker end)
    blank_bundles_match = re.search(r'^bundles:\s*\n\s*\noutputs:\s*\n', content, re.MULTILINE)
    if not blank_bundles_match:
        print("Error: blank bundles/outputs section not found")
        sys.exit(1)

    # find the real outputs section (the one with content after it)
    # it's the second "outputs:" in the file
    outputs_matches = list(re.finditer(r'^outputs:\n', content, re.MULTILINE))
    if len(outputs_matches) < 2:
        print("Error: expected two outputs: sections (blank and generated)")
        sys.exit(1)

    real_outputs_start = outputs_matches[1].start()

    # build new content
    before_markers = content[:marker_start_match.start()]
    after_blank = content[real_outputs_start:]

    new_content = before_markers + bundles_section + after_blank

    with open(YAML_FILE, 'w') as f:
        f.write(new_content)

    print(f"Restored {YAML_FILE}")
    print("- Original bundles uncommented")
    print("- Blank sections removed")

def main():
    if len(sys.argv) != 2 or sys.argv[1] not in ('prepare', 'restore'):
        print(__doc__)
        sys.exit(1)

    if sys.argv[1] == 'prepare':
        prepare()
    else:
        restore()

if __name__ == "__main__":
    main()
