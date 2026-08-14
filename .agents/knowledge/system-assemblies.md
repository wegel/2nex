# System Assemblies

This note includes the still-valid assembly concepts from the former
`.claude/agent_docs/working-on-assembly-manifests.md`. Nex removed that file
because its command examples and rule against agent-run assembly tests no
longer matched the repository. `AGENTS.md` and `.agents/TESTING.md` require
agents to build and exercise affected assemblies.

## `nex_structure` Reproducibility

For a `nex_structure: true` system, both passes of `nex build <asm> --check`
must materialize the same explicit package refs from the assembly manifest.
Using the expanded runtime dependency closure in the second pass changes the
root filesystem and makes checksums differ.

Evidence: before commit `d757d0b`, `desktop-vwl` logged
`Materialized 108 packages` in the first pass and `Materialized 206 packages`
in the second pass, then failed the checksum comparison. After the fix,
`desktop-vwl` logged the same package count in both passes and passed with
checksum `ed3fbaa8397e02d42968b2d0201d52d58e2d45ac5d03506beed3acf6d7d74b09`.

## Source Snapshots

An assembly that copies `asm/` into the built root filesystem must avoid
embedding mutable `checksum:` fields from those copied manifests. Otherwise
`--update-checksum` changes the source tree between the first and second
build pass, and the root filesystem checksum can never stay stable.

`asm/desktop-vwl/desktop-vwl.yaml` strips `checksum:` lines from copied
`asm/*.yaml` files after extracting the dev source.

The installer assembly embeds the package manifest database under
`/nex/db/pkg`, so changing package manifests can change the installer assembly
checksum even when the installer scripts do not change.

Evidence: EP007 changed `kvmfr`, `nvidia-580`, and `v4l2loopback` manifest
checksums. A read-only installer build then changed from
`75d6fdeefd99732cf626303636924e26056433050cf9c599aeffb1fb1ae9f934` to
`ca2299734945b8e9e1a2b6b4415c30ff8a4040ee2863c15117c2fd85d0be7849`.

## Artifact Inspection

Use `zub --repo .nex/repo checkout --copy systems/<slug>/<version> <dir>` to
inspect a built system tree. Public `/usr` paths often use symlinks into
`/nex/pkg`, so host-side `test -e` can report false negatives for absolute
symlinks. Check `test -L` or inspect the symlink target under the checked-out
root.

Use `zub --repo .nex/repo ls-tree -r systems/<slug>/<version>` when a smoke
only needs to prove that a nested path exists in a built root. `zub ls` is not
a command.

Evidence: ExecPlan 004 used recursive `ls-tree` to find `/boot/amd-ucode.cpio`
in `systems/desktop-vwl/0.0.1`,
`systems/desktop-vwl-nvidia-580/0.0.1`, and
`systems/desktop-vwl-nvidia-current/0.0.1`.

The desktop assemblies now include both `/boot/amd-ucode.cpio` and
`/boot/intel-ucode.cpio` through the base `desktop-vwl` assembly. The Nvidia
580 and current variants inherit both boot artifacts.

Evidence: after adding Intel microcode, recursive `ls-tree` found both
`/boot/amd-ucode.cpio` and `/boot/intel-ucode.cpio` in
`systems/desktop-vwl/0.0.1`, `systems/desktop-vwl-nvidia-580/0.0.1`, and
`systems/desktop-vwl-nvidia-current/0.0.1`.

For file-existence proof in a checked-out `nex_structure` root, prefer
`unshare --root <root> /usr/bin/test -e /path`. That makes absolute symlinks
under `/nex/pkg` resolve against the checked-out root instead of the host.

When the check also needs root privileges inside an unprivileged user
namespace, use `unshare --user --map-root-user --root <root>`.

Evidence: host-side `test -e` failed for
`.nex/tmp/desktop-vwl-002h-smoke/usr/include/linux/eventpoll.h`, while
`unshare --root .nex/tmp/desktop-vwl-002h-smoke /usr/bin/test -e
/usr/include/linux/eventpoll.h` passed.

Some desktop command smokes must run as the test desktop user because the
program refuses root. Use `unshare --map-user=1000 --map-group=1000 --root
<root> <command>` to map the current host user to UID and GID 1000 inside the
checked-out root.

Evidence: Looking Glass B7 printed `Do not run looking glass as root!` under a
root-mapped namespace. `unshare --map-user=1000 --map-group=1000 --root
.nex/tmp/desktop-vwl-nvidia-580-smoke /usr/bin/env HOME=/tmp/lg-home
XDG_CONFIG_HOME=/tmp/lg-home/.config XDG_DATA_HOME=/tmp/lg-home/.local/share
/usr/bin/looking-glass-client --help` printed `Looking Glass (B7)` and
returned 255 after help.

For large package runtime closures, `zub union-checkout` can fail on duplicate
files. For smoke roots, write the full refs to a disposable file and overlay
them with sequential `zub checkout --copy --force` calls.

Evidence: Chromium's 55-ref runtime closure failed `zub union-checkout` on
`/usr/include/at-spi2-atk/2.0/atk-bridge.h`. Sequential checkouts into the
same `.nex/tmp/chromium-smoke` root produced a usable tree where
`chromium --version` and `chromedriver --version` ran.

For runtime checks inside a checked-out `nex_structure` root, use
`unshare --root "$root"` before testing public paths or running programs.
`/lib64/ld-linux-x86-64.so.2` in the root is the Nex loader shim, so direct
loader smokes should call the real glibc loader under `/nex/pkg`.

Evidence: the `desktop-vwl` Loupe smoke used
`/nex/pkg/libs/system/glibc/2.39/7fa68fd3/usr/lib/ld-linux-x86-64.so.2` with
`unshare --root .nex/tmp/desktop-vwl-loupe-smoke` to resolve
`/nex/pkg/apps/graphics/loupe/49.2/5324034a/usr/bin/loupe`, then ran
`/usr/bin/loupe --help` through the same checked-out root.

Small package smoke roots assembled from output refs may lack the `/lib64`
interpreter path that full Nex system roots provide. If direct execution fails
with `No such file or directory`, run the binary through
`/usr/lib/ld-linux-x86-64.so.2` in the smoke root, then run a direct command
smoke later in a full assembly root.

Evidence: a `wlopm` package smoke root could run
`unshare --root .nex/tmp/wlopm-smoke /usr/lib/ld-linux-x86-64.so.2
/usr/bin/wlopm --version`, but direct `/usr/bin/wlopm --version` needed the
full `desktop-vwl` root with the Nex loader shim installed at `/lib64`.

## Overlay Checks

Assembly overlay files such as `asm/nex-systemd-overlay.yaml` are not
standalone manifests. Check the assembly that consumes the overlay instead.

Evidence: `./src/cli/target/debug/nex check asm/nex-systemd-overlay.yaml`
reported `needs formatting` and `missing field package`, while
`./src/cli/target/debug/nex check asm/nex-systemd.yaml` passed because
`asm/nex-systemd.yaml` consumes the overlay through `overlays:`.

## Script Runtime Packages

System assemblies do not currently pull script interpreters from package output
`needs` metadata. If an assembly adds a script package, add the interpreter
package explicitly unless another package already supplies it.

Evidence: after `desktop-vwl` added `dool` and `mosh` package refs, the
checked-out system could not run `dool --version` because `python3` was absent,
and could not run `mosh --version` because Perl was absent. Adding the Python
and Perl runtime package refs to `asm/desktop-vwl/desktop-vwl.yaml` made the
next reproducible assembly build and checked-out system smoke pass.

## Command Name Conflicts

When two packages install the same command name, keep the command that best
matches the system role and test the alternate binary name when it exists.

Evidence: `ast-grep` installs both `/usr/bin/ast-grep` and `/usr/bin/sg`, but
a checked-out `desktop-dev` system kept Shadow's standard `/usr/bin/sg` group
command. The assembly smoke used `ast-grep --version` instead of `sg --version`
and left Shadow's command in place.

## Development Tool Assemblies

Development assemblies should use full or dev bundles for build tools that
need support files. A bare `outputs/bin` ref can expose the command while it
omits modules, data directories, pkg-config files, or completion files that the
command needs at runtime.

Evidence: `desktop-dev` originally used `outputs/bin` for CMake and Meson.
The checked-out root failed `cmake --version` because `CMAKE_ROOT` data under
`/usr/share/cmake-3.31` was missing, and failed `meson --version` because
`mesonbuild` was not importable. Switching CMake and other build tools to
full/dev bundles, fixing the Meson wrapper, and rebuilding `desktop-dev`
produced reproducible checksum
`a0b3c72bc6b8e4f815e897e1998dce51c2d899bdeb1cbd486a93b6fab5c18588`. The
checked-out system then ran version checks for CMake, Meson, Ninja, autotools,
GN, Rust, Node, Python, Perl, GDB, LLDB, and related dev tools.

## App Capsule Runtime Files

`nex_structure` systems expose public commands as symlinks into package
capsules under `/nex/pkg`. Libraries can be flattened into those capsules, but
programs that load plugins by search path may still need package-local plugin
files and wrapper environment variables.

Evidence: a checked-out `desktop-dev` root exposed
`/usr/bin/sqlitebrowser -> /nex/pkg/apps/misc/sqlitebrowser/3.13.1+ce16bd0/2275a161/usr/bin/sqlitebrowser`.
The first assembly smoke failed because Qt could not find the offscreen
platform plugin from that capsule. After the package copied the Qt platform
plugins and the wrapper set `QT_PLUGIN_PATH`, a checked-out system smoke ran
`sqlitebrowser --version` with `QT_QPA_PLATFORM=offscreen` and again with
`QT_QPA_PLATFORM=wayland`.

Python applications that run from capsules need their wrappers to point Python
at the capsule-local `site-packages` directory.

Evidence: `desktop-vwl` exposed `/usr/bin/virt-manager` as an absolute symlink
into `/nex/pkg/apps/virt/virt-manager/5.1.0/...`. The public
`/usr/lib/python3.12/site-packages` tree did not contain the application's
Python dependencies, while the capsule-local `usr/lib/python3.12/site-packages`
tree did. After the wrapper inserted that path, a checked-out
`systems/desktop-vwl/0.0.1` root ran `virt-manager --version`,
`virt-install --version`, `virt-clone --version`, and `virt-xml --version`
under `unshare --root`, and a Python smoke imported GTK, GTKSource, GTK-VNC,
VTE, LibvirtGLib, Libosinfo, `libvirt`, `libxml2`, `requests`, `virtinst`, and
`virtManager.virtmanager`.

GLVND, GBM, and Vulkan provider files are runtime inputs even though ELF
`DT_NEEDED` metadata does not name them. A graphics app capsule can contain
Mesa loader libraries such as `libEGL.so.1` and `libgbm.so.1` while missing
the Nvidia vendor JSON and provider libraries that those loaders open later.
On Nvidia machines, the capsule also needs the right generic EGL and GLES
provider files. If the capsule keeps Mesa `libEGL.so.*` or `libGLESv2.so.*`,
those files can try Mesa DRI before the Nvidia provider can create a screen.

Assemblies choose graphics runtime providers through their `providers:` map.
`desktop-vwl` binds `graphics.egl`, `graphics.gles`, and `graphics.gbm` to
Mesa. `desktop-vwl-nvidia-580` and `desktop-vwl-nvidia-current` bind
`graphics.egl` and `graphics.gles` to their Nvidia runtime bundles, then bind
`graphics.gbm` to Mesa.

Evidence: the real `desktop-vwl-nvidia-580` laptop loaded the Nvidia kernel
driver, but `/tmp/lol.txt` from `vwl` showed `eglInitialize` failing with
`EGL_NOT_INITIALIZED`; the `vwl` capsule had Nvidia JSON,
`libEGL_nvidia.so.0`, and `nvidia-drm_gbm.so`, but inherited Mesa
`libEGL.so.1` and `libGLESv2.so.2` through `wlroots`. EP008 replaced the
Nvidia-specific materializer hook with explicit capability providers. Fresh
checkouts of `systems/desktop-vwl/0.0.1`,
`systems/desktop-vwl-nvidia-580/0.0.1`, and
`systems/desktop-vwl-nvidia-current/0.0.1` then showed the base `vwl` capsule
using Mesa EGL, GLES, and GBM; the Nvidia capsules using Nvidia EGL and GLES;
and the Nvidia capsules using Mesa GBM. The current Nvidia capsule also showed
`libnvidia-egl-wayland2.so.1` and `09_nvidia_wayland2.json`.
