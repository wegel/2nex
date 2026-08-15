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

## Build Dependencies And Finished Roots

An assembly entry under `dependencies` populates the assembly build root but
does not guarantee that the selected output appears in the finished system.
Put an output under `packages` when the installed root must contain it. Base
assemblies should select shared runtime data once so child assemblies inherit
it.

Evidence: `nex-systemd` and the installer already used Glibc's library output
as a dependency, but the first EP011 Edgebox checkout lacked
`/usr/lib/nsswitch.conf` and `/usr/lib/rpc`. Adding Glibc's `outputs/conf` to
the `packages` list in `flat-minimal`, `flat-systemd`, `nex-minimal`,
`nex-systemd`, and the installer put both databases in all checked roots.

## Artifact Inspection

Use `zub --repo .nex/repo checkout systems/<slug>/<version> <dir>` to
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

When `unshare --root` cannot provide the required identity, create a rootless
user and mount namespace and enter the checkout with `chroot`:

```bash
unshare --user --map-root-user --mount --pid --fork \
    chroot <root> /usr/bin/bash -c 'test -f /usr/lib/nsswitch.conf'
```

EP011 used this form to prove that absolute BlueZ and Glibc public symlinks
resolved to real capsule files inside checked-out Nex-structured roots.

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
them with sequential `zub checkout --force` calls. Current Zub versions copy
by default; do not add `--hardlink` to a smoke root that tests may modify.

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

## First UAPI Parser Wave System Proof

After BlueZ, PulseAudio, OpenSSH, and Glibc moved their vendor defaults below
`/usr`, EP011 rebuilt all 11 affected assemblies twice. The final checksums
were:

- flat-minimal: `04d87da9a07545e09f4689eccfd57f2a97a588351e0d2c3ab74ae4301c2e348b`
- flat-systemd: `4df96ca03dc9c74ac0ff3133ce8933c76350b11f2c01d3dff7fdca81f02a855e`
- installer: `8eb597bfe72f7e71c7edaa1a03467f703bfcf9607e8b2a678b27d56bfea85b40`
- nex-minimal: `fd375b373548d00250a8c1c0d3b76be0b4684849c3b99026bcd2f0711e60c35a`
- nex-systemd: `2d6870c64c01da6cde65a214900ff6f174f9c63184cfbc9c029b331f2247e5e9`
- edgebox-rootfs: `d8c158b77a3c864942af08520cdf8417d5c10703250cc3f05bb91acf4f1e1098`
- flat-podman: `f8feefae4d82452da5dec9f6fe69e5d65c008a7e6a34273184838f0fe9bd31c9`
- desktop-vwl: `2af27d9058ab5c6a54ab88cdab1acb9185a1028c58e2de2a7d87ef9d9c0d5e4e`
- desktop-vwl-nvidia-current: `5385c15750bcdf85cd7f7168148ebad17720dcee519971aeeacbd1c03d741810`
- desktop-vwl-nvidia-580: `0080b15a7cd317b8f1de3904f56b2f720eb4444a7771844ac21b9cc9afbb13ad`
- desktop-dev: `75e4957348ef7c2e7f410ee9a16b52dad18405a315967477018e9492eaae24f5`

The Edgebox smoke read the packaged RPC database through `getent`, found the
OpenSSH and Glibc vendor files, and found no replaced package files below
`/etc`. A desktop checkout contained all three BlueZ defaults and both Glibc
databases without package-owned `/etc` copies. Flat-minimal, nex-minimal, and
the installer also contained both Glibc databases. The direct nex-systemd
QEMU test printed `ASSERT-BOOT-PASS`.

A broad assembly build also checks every declared store ref. `desktop-dev`
could not build until EP011 strictly rebuilt and exercised 19 stale or missing
package refs. Treat such refs as package prerequisites: fix and commit each
package as a working bisect point, then resume the assembly.

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

## Firewall Data

Edgebox selects Nftables' full bundle directly, so its finished root contains
`/usr/lib/nftables/osf/pf.os`. Its smoke runs the installed `nft` against an
`osf` rule inside a private network namespace and verifies that the reader
opens and loads that vendor file. The strict assembly checksum after EP012 is
`8c29d7b5685704b5bbba57d3243fb6df5ac39413029e1613a8e6f229933e89f5`.

Docker's Iptables dependency does not make every Iptables output part of a
finished system. Edgebox receives the Iptables binaries and libraries that
Docker's output metadata names, but not the sibling `conf` output or
`ebtables-translate`. An assembly that needs Ethernet protocol name lookup
must select those Iptables outputs itself.

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

Python GI applications also need their capsule-local typelibs. A wrapper can
derive its installed prefix from its own path, prepend
`<prefix>/lib/girepository-1.0` to `GI_TYPELIB_PATH`, and keep the caller's
existing value after it. Publishing GTK's public runtime fixed Gdk and Gtk
discovery, but `virt-manager --version` then failed on private
`LibvirtGLib`. The prefix-relative wrapper found the flattened private
typelibs, preserved a caller-supplied path, and printed `5.1.0` in the final
desktop root.

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

## ImageMagick layered configuration

After ImageMagick 7.1.2-26 moved fourteen package configuration XML files from
`/etc/ImageMagick-7` to `/usr/share/ImageMagick-7`, rebuild the base desktop
before its child assemblies and run these large strict builds sequentially.
Four concurrent builds exhausted this host's `/tmp` allowance and raced while
refreshing the shared parent ref; each build passed twice when run alone.

The finished base root contains seventeen XML files in total: the fourteen
configuration files plus three locale files that were already package data
below `/usr/share`. It contains no `/etc/ImageMagick-7`. In a checked-out
`systems/desktop-vwl/0.0.1` root, the public `magick -list policy` reports
`/usr/share/ImageMagick-7/policy.xml`, and `magick` plus `identify` can create
and inspect a one-pixel PNG. The reproducible checksums are:

- desktop-vwl: `5805c8a10426ec29b4df0702148c63613822e008bf9a4c7a53fc67c3ad24a9e4`
- desktop-vwl-nvidia-580: `358a03b813a2b7a1946638b0f9d5b189bb35537bbe4089ba77da5fde098c5aed`
- desktop-vwl-nvidia-current: `46da14b8f6da8121ce207efe7980cb6f714edd1e1b3ef343eb6b9267673c08d6`
- desktop-dev: `3808ccac10d339f557a918166f38c140f2c0004b9b748675e0f01f0305055339`

## Copying package templates into factory state

Public paths in a Nex-structured assembly usually point through absolute
links into `/nex/pkg`. A build script that runs outside `/target` cannot use
`cp -L /target/usr/...`, because the host resolves `/nex` outside the target
root. Read the link first. Prefix an absolute target with `/target`; resolve a
relative target from the public path's directory. Then copy the resolved
regular file into `/target/etc` before the base assembly moves that tree to
`/usr/share/factory/etc`.

Desktop-vwl uses this pattern to seed Libvirt's twenty-four nwfilter objects,
default virtual network, and relative autostart link. The checked-out system
contains regular XML files in the factory tree, while the immutable upstream
templates remain in the package capsule.

## Assembly `/etc` overlays

Audit assembly-owned `/etc` entries separately from reusable package files.
An assembly may choose initial accounts, machine identity, network policy,
authentication policy, product settings, and enabled services. Service links
below `/etc/systemd` record explicit enablement choices; `mtab`,
`resolv.conf`, and `localtime` links preserve well-known compatibility paths.

EP012 scanned every declared `/etc` path in the four source overlays and
found 98 entries: 26 in `nex-systemd`, 19 in Edgebox, 48 in desktop-vwl, and
five in the installer. The `nex-systemd`, desktop-vwl, and installer
assemblies set `nex_structure: true`, so the builder moves these initial host
files and links to `/usr/share/factory/etc`. The initramfs later copies only
missing paths into writable host `/etc`. Edgebox extends the flat Systemd
root, so its overlay intentionally writes the appliance's final policy and
service links directly below `/etc`. Nvidia desktop variants and desktop-dev
inherit the audited desktop overlay and add no separate overlay.
