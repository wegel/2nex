#!/usr/bin/env python3
# Package-side repair for NVIDIA's proprietary application-profile reader.
# Six shipped libraries compile in one colon-separated system path table:
# the administrator main file, the administrator directory, a versioned
# vendor file, and an undocumented unversioned vendor fallback. There is no
# /run entry. This script swaps the versioned-vendor slot for the
# normalized runtime directory, padding it with harmless trailing slashes
# so every patched library keeps its exact original byte count, and
# repoints the installed documentation and profile-file comments at the
# resulting order: administrator main, administrator directory, runtime
# directory, then the stable unversioned vendor link this script installs
# to the packaged versioned profile file.

import os
import sys

LIB_NAMES = [
    "libcuda.so.{version}",
    "libnvidia-eglcore.so.{version}",
    "libnvidia-glcore.so.{version}",
    "libnvidia-glsi.so.{version}",
    "libnvidia-opencl.so.{version}",
    "libnvidia-vksc-core.so.{version}",
]


def build_path_tables(version):
    admin_main = "/etc/nvidia/nvidia-application-profiles-rc"
    admin_dir = "/etc/nvidia/nvidia-application-profiles-rc.d/"
    vendor_versioned = f"/usr/share/nvidia/nvidia-application-profiles-{version}-rc"
    vendor_stable = "/usr/share/nvidia/nvidia-application-profiles-rc"
    runtime_dir = "/run/nvidia/nvidia-application-profiles-rc.d"

    old = ":".join([admin_main, admin_dir, vendor_versioned, vendor_stable])
    unpadded_new = ":".join([admin_main, admin_dir, runtime_dir, vendor_stable])
    pad = len(old) - len(unpadded_new)
    if pad < 0:
        sys.exit(
            "nvidia application-profile repair: the runtime directory entry "
            "does not fit inside the original path table's byte count"
        )
    new = ":".join([admin_main, admin_dir, runtime_dir + ("/" * pad), vendor_stable])
    if len(new) != len(old):
        sys.exit("nvidia application-profile repair: padded path table length mismatch")
    return old.encode("ascii"), new.encode("ascii")


def patch_library(path, old, new):
    with open(path, "rb") as handle:
        data = handle.read()

    occurrences = data.count(old)
    if occurrences != 1:
        sys.exit(
            f"nvidia application-profile repair: expected exactly one copy of "
            f"the vendor path table in {path}, found {occurrences}"
        )
    if data.count(new) != 0:
        sys.exit(
            f"nvidia application-profile repair: patched path table already "
            f"present in {path}"
        )

    patched = data.replace(old, new, 1)
    if len(patched) != len(data):
        sys.exit(
            f"nvidia application-profile repair: patch changed the byte "
            f"count of {path}"
        )

    with open(path, "wb") as handle:
        handle.write(patched)

    with open(path, "rb") as handle:
        verify = handle.read()
    if old in verify:
        sys.exit(f"nvidia application-profile repair: old path table still present in {path}")
    if verify.count(new) != 1:
        sys.exit(
            f"nvidia application-profile repair: patched path table is not "
            f"present exactly once in {path}"
        )
    if os.path.getsize(path) != len(data):
        sys.exit(f"nvidia application-profile repair: {path} changed size on disk")


def install_vendor_symlink(share_dir, version):
    link_path = os.path.join(share_dir, "nvidia-application-profiles-rc")
    if os.path.lexists(link_path):
        os.remove(link_path)
    os.symlink(f"nvidia-application-profiles-{version}-rc", link_path)


def replace_exactly_once(path, old_text, new_text, what):
    with open(path, "r", encoding="utf-8") as handle:
        text = handle.read()
    if text.count(old_text) != 1:
        sys.exit(f"nvidia application-profile repair: {what} did not match {path}")
    text = text.replace(old_text, new_text, 1)
    with open(path, "w", encoding="utf-8") as handle:
        handle.write(text)


def update_profile_header(share_dir, version):
    rc_path = os.path.join(share_dir, f"nvidia-application-profiles-{version}-rc")
    old_header = (
        "# These profiles were provided by NVIDIA and should not be modified.  If you\n"
        "# wish to change the defaults provided here, you can override them by creating\n"
        "# custom rules in /etc/nvidia/nvidia-application-profiles-rc (which will apply\n"
        "# system-wide) or, for a given user, $HOME/.nv/nvidia-application-profiles-rc\n"
        "# (which will apply to that particular user). See the \"APPLICATION PROFILE\n"
        "# SEARCH PATH\" section of the NVIDIA Linux Graphics Driver README for more\n"
        "# information.\n"
    )
    new_header = (
        "# These profiles were provided by NVIDIA and should not be modified.  If you\n"
        "# wish to change the defaults provided here, you can override them by creating\n"
        "# custom rules in /etc/nvidia/nvidia-application-profiles-rc (which will apply\n"
        "# system-wide) or, for a given user, $HOME/.nv/nvidia-application-profiles-rc\n"
        "# (which will apply to that particular user). See the \"APPLICATION PROFILE\n"
        "# SEARCH PATH\" section of the NVIDIA Linux Graphics Driver README for more\n"
        "# information. This driver package reads /run/nvidia/nvidia-application-profiles-rc.d\n"
        "# before it reaches this file, and it only reaches this file through the\n"
        "# stable /usr/share/nvidia/nvidia-application-profiles-rc link, so an\n"
        "# administrator or the running system can still override every rule below.\n"
    )
    replace_exactly_once(rc_path, old_header, new_header, "profile file header")


def update_doc(doc_dir, version):
    readme_path = os.path.join(doc_dir, "README.txt")
    old_readme_line = f"   o '/usr/share/nvidia/nvidia-application-profiles-{version}-rc'\n"
    new_readme_lines = (
        "   o '/run/nvidia/nvidia-application-profiles-rc.d'\n"
        "\n"
        "   o '/usr/share/nvidia/nvidia-application-profiles-rc' (a stable name this\n"
        f"     package links to the versioned file above, nvidia-application-profiles-{version}-rc)\n"
    )
    replace_exactly_once(readme_path, old_readme_line, new_readme_lines, "README.txt search path")

    html_path = os.path.join(doc_dir, "html/profiles.html")
    old_html_entry = (
        "<li>\n"
        "<p><code class=\n"
        f"\"filename\">/usr/share/nvidia/nvidia-application-profiles-{version}-rc</code></p>\n"
        "</li>\n"
    )
    new_html_entry = (
        "<li>\n"
        "<p><code class=\n"
        "\"filename\">/run/nvidia/nvidia-application-profiles-rc.d</code></p>\n"
        "</li>\n"
        "<li>\n"
        "<p><code class=\n"
        "\"filename\">/usr/share/nvidia/nvidia-application-profiles-rc</code></p>\n"
        "<p>This package installs this stable name as a link to the versioned vendor\n"
        f"file nvidia-application-profiles-{version}-rc.</p>\n"
        "</li>\n"
    )
    replace_exactly_once(html_path, old_html_entry, new_html_entry, "profiles.html search path")


def main():
    if len(sys.argv) != 3:
        sys.exit("usage: nvidia-application-profile-repair.py OUT_DIR VERSION")
    out_dir, version = sys.argv[1], sys.argv[2]

    old, new = build_path_tables(version)

    lib_dir = os.path.join(out_dir, "usr/lib")
    for template in LIB_NAMES:
        patch_library(os.path.join(lib_dir, template.format(version=version)), old, new)

    share_dir = os.path.join(out_dir, "usr/share/nvidia")
    install_vendor_symlink(share_dir, version)
    update_profile_header(share_dir, version)
    update_doc(os.path.join(out_dir, f"usr/share/doc/nvidia-{version}"), version)


if __name__ == "__main__":
    main()
