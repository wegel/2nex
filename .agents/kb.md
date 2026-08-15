# Nex Agent Runbook

Keep this file current while working. Add a fact after a command, source file,
or built artifact proves it. Correct or remove a fact when later evidence
disproves it. Keep temporary logs, guesses, and one-off task notes elsewhere.
Git tracks this runbook and the durable `.agents/knowledge/` notes. Git ignores
runtime state such as `.agents/SCRATCH_KNOWLEDGE.md`, waiting state, and paused
local plans.

## Rootfs replication scope

The current task uses the normal `/rootfs/` made by Soniq's Yocto builder as
an assembly target. The source builder lives at:

```text
/var/home/wegel/work/tt/projects/soniq/wegel/apps/tools/rootfs-builder
```

The most useful source files and artifacts are:

```text
meta-soniq/recipes-core/images/soniq-image.bb
build/build/tmp/deploy/images/soundwave/soniq-image-soundwave.rootfs.manifest
build/build/tmp/work/soundwave-poky-linux/soniq-image/1.0/rootfs
```

`soniq-image.bb` lists packages that the image asks for directly. The deployed
manifest lists the full package closure. The built root contains the exact
files, links, units, users, groups, and configuration that later runtime tests
must compare.

### Generic-correct package policy

Soniq and Yocto identify packages, versions, and assembly behavior. They do
not set Nex package policy. A package manifest must install the normal
upstream package with generally useful features and remain suitable for other
assemblies. Start from upstream defaults, then enable broadly useful features
when Nex has their dependencies. A source snapshot is acceptable when the
package version names that snapshot honestly.

Do not copy Yocto-only header renames, alternatives names, feature cuts,
service presets, branding, selected data subsets, cross-compile cache answers,
or system defaults into a package manifest. Do not force an optional feature
off merely because the Soniq image did. Put choices such as enabled services,
the system time zone, and product-specific configuration in an assembly.

Package scope can still be concrete: for example, a terminal-only editor or a
Curses pinentry can omit GUI backends when its name and description say so.
Nex-wide filesystem rules such as installing administrative commands in
`/usr/bin`, and deterministic-build fixes that do not change behavior, still
belong in package manifests.

The agreed work order is:

1. Add every missing package manifest and make every package compile.
2. Update the kernel after the user-space packages compile.
3. Compare and test the assembled rootfs after the kernel work exposes the
   next concrete gaps.

## Manifest checks and builds

Build the Nex CLI once when `src/cli/target/debug/nex` is absent or stale:

```sh
cargo build --manifest-path src/cli/Cargo.toml --bin nex
```

The checked-in Cargo config uses `src/cli/vendor`. If that ignored directory
is absent, populate it before building the CLI:

```sh
cargo vendor --locked --manifest-path src/cli/Cargo.toml src/cli/vendor
```

Format and statically check each changed package manifest:

```sh
./src/cli/target/debug/nex format pkg/path/package.yaml
./src/cli/target/debug/nex check pkg/path/package.yaml
```

Use this strict package build while developing a manifest:

```sh
./src/cli/target/debug/nex build pkg/path/package.yaml \
  --verbose \
  --single \
  --check \
  --update-checksum \
  --force \
  --compute-deps \
  --record-profile \
  --generate-outputs
```

`--check` builds the package twice and compares the results. The other update
flags write the proven checksum, resource profile, dependency data, and output
file lists back to the manifest. Run `nex format` and `nex check` again after
the build writes those fields.

After the strict build packages its result, the retained build root can hold
category subdirectories below `/nex/out`, such as `/nex/out/bin`,
`/nex/out/conf`, or `/nex/out/misc`, rather than one merged output tree. Inspect
the retained tree before writing the smoke command. Run the smoke against the
category that owns the tested file, or check the matching zub bundle out into
a disposable root.

Autoconf `configure` scripts often call `sed` before they finish parsing
options or testing the compiler. If a log prints `sed: command not found`,
then also claims valid options are unrecognized or that the C compiler cannot
create executables, add the explicit `sed` build dependency before diagnosing
the later messages. Libusb 1.0.29 produced this exact failure when its first
manifest omitted `sed`.

Autoconf also probes an `awk` implementation and a `grep` that supports long
lines. Libusb 1.0.29 failed after compiler setup with `no acceptable grep`
and had already reported that `gawk`, `mawk`, `nawk`, and `awk` were absent.
For comparable Autoconf packages, declare Nex's `gawk`, `grep`, and `sed`
build bundles explicitly unless the script demonstrably does not call them.

Do not ignore missing `cmp`, `diff`, or `file` messages merely because an
Autoconf package later compiles. Libtool uses those commands while it probes
binary pipes, compiler flags, and dependent-library handling. Add
`core/userland/diffutils` for `cmp` and `diff`, and `dev/tools/file` for the
`file` command, then rebuild so the final checksum reflects complete probes.

Libusb 1.0.29 passed the strict two-build check with raw checksum
`a510d04413873217e1b4e2b88b420f8faebe51020cffc64be46fceb3503b38c0`
after its manifest declared `diffutils`, `file`, `gawk`, `grep`, and `sed`.
The completed probes found `file` and a working `dd`; no missing `cmp`,
`diff`, or parser tools remained. A C consumer compiled against the packaged
`libusb.h`, linked to the packaged `libusb-1.0.so`, ran inside the retained
build root, and printed `libusb 1.0.29 LIBUSB_ERROR_TIMEOUT`.

Declare the decompressor that matches the source archive. `tar` delegates
`.tar.gz` input to `gzip`, `.tar.bz2` input to `bzip2`, and `.tar.xz` input to
`xz`. USB Utilities 019 failed with `gzip: Cannot exec` until its manifest
declared the `cli/archive/gzip` build bundle.

Glibc's development headers include Linux UAPI headers such as
`linux/limits.h` and `linux/errno.h`, but the glibc bundle does not
automatically materialize the Linux header tree. A C or C++ package that
reaches those includes must declare `libs/system/linux-headers` explicitly.
USB Utilities 019 exposed this after Meson had already found both `libudev`
and `libusb-1.0` successfully.

Libssh 0.11.4 finds MIT Kerberos through `/usr/bin/krb5-config`. The existing
Kerberos manifest originally put that build helper in its `bin` output, while
the `dev` bundle contained only the `dev` and `lib` outputs. Adding `pkgconf`
to libssh was therefore insufficient: CMake found pkgconf, but still disabled
GSSAPI because `krb5-config` was absent. Treat `krb5-config` as a development
tool and keep it in Kerberos's `dev` output so consumers do not need the full
Kerberos application bundle merely to compile.

After moving `krb5-config`, libssh's build compiled `src/gssapi.c`, linked the
MIT Kerberos libraries, and passed the strict two-build check with raw checksum
`822e077babf2292cdd3044ae17310038e683850d6128e04a5b78b683504def87`.
The Kerberos output split changed without changing its raw package files, and
Kerberos itself also passed the strict two-build check.

USB Utilities 019 passed the strict two-build check with raw checksum
`48e578713eff28e954b61ad93ff52d6bca72a3c58517c0686869c99cf5ce71b6`.
Its Meson setup found libudev 257 and libusb 1.0.29. A retained-root smoke
ran the packaged `lsusb --version` and got `lsusb (usbutils) 019`; Bash
parsed the packaged `usb-devices` script and Python compiled the packaged
`lsusb.py` module. The packaged binary lived at
`/nex/out/bin/usr/bin/lsusb`, not `/nex/out/usr/bin/lsusb`, because Nex had
already split the raw output into output categories.

To retain a root for a command-level smoke after a strict build, rebuild one
copy with a concrete ignored build path and `--reuse-rootfs`, for example:

```sh
./src/cli/target/debug/nex build pkg/cli/system/usbutils.yaml \
  --verbose --single --force --compute-deps --generate-outputs \
  --build-dir .nex/tmp/usbutils-smoke --reuse-rootfs
```

This smoke rebuild must produce the same checksum as the strict build. Inspect
`<build-dir>/nex/out` before choosing the executable path.

Bridge Utilities 1.7.1 ships `configure.ac` but no generated `configure`
script. Its manifest must declare the Autoconf runtime tools and run
`autoheader` followed by `autoconf` before `./configure`. The upstream install
defaults `brctl` to `/usr/sbin`, which `nex check` rejects because Nex reserves
`/usr/sbin` for the usrmerge symlink. Pass `--sbindir=/usr/bin` to `configure`;
an uppercase `SBINDIR=/usr/bin` argument to `make install` does not override
the upstream lowercase `sbindir` variable.

Bridge Utilities 1.7.1 passed the strict two-build check with raw checksum
`c553824613c34c8c2addc8da9e0bb17f36888173fc1a49b5877a3e7566c54f3a`.
Its retained-root smoke ran the packaged `brctl --version`, which printed
`bridge-utils, 1.7`.

I2C Tools 4.4 defaults its compiler command to `cc`, but Nex's GCC dev bundle
provides `gcc` without a `cc` alias. Pass `CC=gcc` to both `make` calls. Its
makefiles also default `sbindir` to `/usr/sbin`; pass lowercase
`sbindir=/usr/bin` to both calls to comply with Nex usrmerge checks. All five
installed Perl scripts need manual `/usr/bin/perl` runtime metadata because
ELF scanning cannot infer their shebang. `i2c-stub-from-dump` also calls
`modprobe`, `rmmod`, `udevadm`, and `uname`, while `decode-dimms` calls `cat`,
so preserve those manual needs and provider mappings when regenerating output
metadata.

I2C Tools 4.4 passed the strict two-build check with raw checksum
`2363c4c0b24114b227e453d6104166886f06e1b7aa27ff98be6bb2d4f178f094`.
Its merged-root smoke printed `i2cdetect version 4.4`, ran the packaged Perl
stub far enough to print its usage, then compiled and ran a C consumer against
the packaged header and `libi2c.so`; the invalid-descriptor call returned
`-9` as expected.

Smartmontools' installed `update-smart-drivedb` and `smartd_warning.sh`
scripts use an upstream `/bin/sh` shebang, and its systemd unit uses
`/bin/kill`. Nex packages
must not own files below `/bin`; rewrite those paths to `/usr/bin/sh` and
`/usr/bin/kill`. `kill` comes from Nex's util-linux package, not coreutils.

Ship Smartmontools' upstream `smartd.conf` and unit, but do not copy Yocto's
`/etc/default/smartmontools`, force an `EnvironmentFile` path, or add a package
preset. An assembly can add a unit drop-in or its own environment file and
decides whether `smartd.service` starts. The generic package passed the strict
two-build check with raw checksum
`9c6ad1b96d029388c2e0f19fc100af545c1cbb19b38e8a08ddfa88192437c3e7`
after adding native UAPI main-file lookup.

The Smartmontools merged-root smoke ran `smartctl --version`, `smartd
--version`, `update-smart-drivedb --help`, and the warning script's `--dryrun`
path. `systemd-analyze verify --man=no --root=<root>` accepted the packaged
service. Without `--man=no`, the host command reports a man-page protocol
error against this incomplete package root even when the service's executable
paths are valid.

Tzdata 2026b needs both the `tzcode2026b` and `tzdata2026b` archives. The data
archive's `make tzdata.zi` target depends on files from the code archive and
fails at the missing `asctime.c` if the manifest extracts only tzdata. Nex's
glibc development bundle provides `zic`. The manifest compiles the full
upstream zone set, including leap-second-aware files below `right/`. It does
not create `/etc/localtime` or `/etc/timezone`; the assembly chooses those
values. A smoke should test multiple zones and a file outside Soniq's former
`tzdata-core` subset.

Tini 0.19.0 uses upstream commit
`369448a167e8b3da4ca5bca0b3307500c3371828`. Build its `tini-static` CMake
target and install only the canonical `/usr/bin/tini` command. Docker owns its
`/usr/bin/docker-init -> tini` compatibility link because that name belongs to
Docker's interface, not Tini's generic package. A merged smoke must run the
canonical command and Docker's link.

Vim 9.2.0340 uses upstream commit
`6addd6c101117706bc9b3609d3a418e26e92618f`. The CLI package enables ACL and
native-language messages and omits GUI dependencies. Apply the small pathdef
patch so Vim does not embed compiler flags, the build user, or the build host
in the binary. Keep upstream command names, runtime tools, and default
configuration behavior. Do not add Yocto alternatives names such as
`vim.vim` or `xxd.vim`, remove upstream tools, install an example as the global
vimrc, or add product branding.

Cryptsetup needs libdevmapper, which the original generic gap list missed.
LVM2 2.03.39 supplies the reference libdevmapper version 1.02.213. The
libdevmapper-only build does not need libaio even though the full LVM2 build
does; configure states that only its device-mapper target supports that
combination. Build and install `libdm`, then run the udev makefile's
`install_device-mapper` target so the package includes the reference
`10-dm.rules`, `13-dm-disk.rules`, and `95-dm-notify.rules` files. Use
`--sbindir=/usr/bin` for Nex usrmerge even though Yocto exposes `dmsetup`
through `/usr/sbin`.

Libdevmapper passes the strict two-build check with raw checksum
`af58b78df0201a9521e6b1c94ffa4a7b084e8308b55e0ee08410a5a6af97f0e3`.
The packaged `dmsetup --help` ran, and a C consumer compiled against
`libdevmapper.h`, linked to the package, and reported library version
`1.02.213 (2026-03-13)`.

Cryptsetup's SSH token adds another missed package: libssh 0.11.4. Build
libssh with its normal OpenSSL backend rather than copying Yocto's libgcrypt
choice. Enable generally useful zlib, SFTP, server, GSSAPI, and packet-capture
support, and declare Krb5 and libpcap directly.

Cryptsetup should enable the available UDEV, BLKID, KEYRING, KERNEL_CAPI,
reencryption, integrity, verity, and SSH-token features. Do not copy Yocto's
LUKS PBKDF memory, thread, iteration, XTS key-size, or other machine policy
into the manifest. Let upstream defaults choose those values.

The Cryptsetup smoke formatted a 32 MiB regular file as LUKS2 with a PBKDF2
test key, then read the header back and found LUKS version 2, the expected
`aes-xts-plain64` data segment, and keyslot 0. A plain `unshare --root` smoke
hangs because Nex's retained build root contains empty placeholder files at
`/dev/random` and `/dev/urandom`. Start a user and mount namespace, bind the
host `/dev` tree over the retained root's `/dev`, and then run `chroot` for any
test that needs real device nodes. Cryptsetup logs a harmless device-mapper
permission warning in that namespace, but it can still format and inspect a
regular-file LUKS header without opening a mapped device.

Containerd 2.2.2 declares Go 1.24.3 or newer in its `go.mod`. The Soniq Yocto
layer uses Go 1.26.4, while Nex originally packaged Go 1.23.8, so update Nex's
Go manifest before building the container stack. The upstream Go 1.26.4
linux-amd64 archive has SHA-256
`1153d3d50e0ac764b447adfe05c2bcf08e889d42a02e0fe0259bd47f6733ad7f`.
The updated Go package passes the strict two-build check with raw checksum
`eca1fa3b3fe41367224157d57ab3068e604d917d30a4b05dd3af1b09d57fbcd6`.
A merged-root smoke reported Go 1.26.4, compiled a small program, and ran the
result. Set `GOROOT=/usr/lib/go` when using the packaged command in a root
without `/proc`; the Go command cannot infer its trimmed executable path in
that environment. Set `GOTELEMETRY=off` for isolated package builds.

Containerd 2.2.2 uses source commit
`5957d3334bcaeddc2bd8e665f53cee318c298a2c`. Build the canonical
`containerd`, `ctr`, and `containerd-shim-runc-v2` commands. Do not copy
Yocto's `containerd-ctr` rename, `docker-containerd*` compatibility links, or
feature-cutting `no_btrfs`, `static_build`, and `netgo` tags. Report the real
source revision without Yocto's `.m` suffix. The service needs
`/sbin/modprobe` rewritten to Nex's usrmerged `/usr/bin/modprobe` path.

Go's default action build ID changed between otherwise identical Containerd
builds. Pass `-buildid=` to the Go linker and `-Wl,--build-id=none` to the C
linker to make all three binaries byte-for-byte reproducible. Runtime smokes
reported the exact reference version and revision for the daemon, client, and
shim; `containerd config default` generated a version 3 configuration. The
host's `systemd-analyze verify --man=no` accepted the service after the
systemd full bundle was added to the disposable smoke root.

Docker's build exposed two pre-existing bundle errors. Iptables 1.8.11 had no
`full` bundle at all, and Nftables 1.1.1's `dev` bundle omitted its own `dev`
output, including `libnftables.pc` and the public header. Add the Iptables
runtime bundle, add the missing Nftables development output, and give
Nftables a normal runtime bundle. Both packages pass their strict two-build
checks after those fixes. Iptables' raw checksum before its UAPI database
change was
`171688dc4bb4cc546ab85bb41b7dc30c5cc83387a7fe0b962b9338f86f416d53`;
Nftables' raw checksum is
`d82c8ef9a12067a13ea84db84a90c061820fcecf70c2a66251bb912c634d78ca`.
Do not copy Yocto's explicit libipq enablement. Iptables leaves that obsolete,
upstream-disabled API out. The Docker smoke ran `iptables --version` from the
new runtime bundle, got the nftables backend, and found no libipq library.

Iptables 1.8.11 now installs its upstream Ethernet protocol database at
`/usr/lib/ethertypes`. Its generic reader selects `/etc/ethertypes`, then
`/run/ethertypes`, then `/usr/lib/ethertypes` as one whole file; an empty
selected file masks lower data. The strict test runs the installed
`ebtables-translate` for every tier. The patch SHA-256 is
`b5514bde8c6f49f3c377684f0628aca0344262177d20ac191e0e95cf4f22c187`,
and the package checksum is
`f5382ebd5cd04f3e472ac57ee95124b1f672b4e84f90ced55c7d4171be882417`.
Docker is the only manifest that consumes Iptables' full bundle. Rebuild it
after both the Iptables and Nftables database changes, then rebuild Edgebox.

Docker 29.3.0 uses Moby commit
`1da6517e1a4381297e56862f6f373f265c28d102` and Docker CLI commit
`df016a3a9538efb6d7d6e7c3baf291dbbe28eee0`. The Moby tree
must appear at `github.com/moby/moby/v2` in its synthetic GOPATH. The relative
link from `.gopath/src/github.com/moby/moby/v2` back to the source root needs
five `..` components. Docker CLI treats Nex's exported `TARGET` triplet as an
output directory, so reset `TARGET=build` before its make target.

Build Moby with its normal upstream tag set. Current Moby gets its btrfs
driver from kernel headers and no longer has a device-mapper graph driver, so
Docker does not need Btrfs Progs or libdevmapper as build dependencies. Do not
copy Yocto's old `exclude_graphdriver_btrfs`, `exclude_graphdriver_devicemapper`,
or forced `seccomp` tags. Current upstream build scripts leave
`DOCKER_BUILDTAGS` empty and detect available libraries themselves. Rewrite
the service's `/bin/kill` to `/usr/bin/kill`. Add manual runtime needs for
Containerd, runc, docker-init, Iptables, and util-linux; ELF scanning cannot
discover commands that dockerd calls later. Current Moby does not call
`brctl`; Yocto added Bridge Utilities as downstream runtime metadata. Keep
Bridge Utilities as a separate generic package and let an assembly include it
when wanted. Ship the Docker and Containerd units without service presets so
each assembly chooses whether they start. Install upstream Docker CLI pages
and Moby's `dockerd(8)` page.

Timestamp cleanup must use `touch -h` when walking a package output tree.
Plain `touch` follows relative links. Docker installs `docker-init -> tini`,
so plain `touch` created an empty `/usr/bin/tini` in Docker's output and hid
the real Tini package when bundles were overlaid. The fixed Docker package
contains only the link, passes the strict two-build check with raw checksum
`6f82e2a82020fb2a8701a3ca2818fc4a497ba5c619d3e26600926caa5e84ecdb`,
and leaves `/usr/bin/tini` to the Tini package.

Nex does not infer a symlink target as a runtime need. Docker's
`/usr/bin/docker-init` entry must need `/usr/bin/tini`; `/usr/bin/docker-init`
resolves to `self`, while `/usr/bin/tini` resolves to the Tini dependency.
Generated dependency metadata can retain stale provider entries, so remove
obsolete mappings such as `/usr/bin/brctl` by hand. A fresh Docker Compose
consumer root then pulled Tini automatically. `docker-init --version`
reported Tini 0.19.0, `docker compose version` reported
`5.1.0+gite8c21434`, and the target remained a 697552-byte, mode-0755 static
executable. Docker Compose passed its strict two-build check with raw checksum
`6a17ef75da47077ae49afdc79b3d2f891415e62a63db67e894308147e58e0cd5`.

Docker Compose 5.1.0 uses Soniq's exact source commit
`e8c214349819cc14804e8946adb8ae3576b7139c`. Its source archive has SHA-256
`3000416fec8b64352f977ba0e10361976fcd62c2706fe4ba006e357e0d3e5dbe`.
The upstream archive does not contain `vendor/`, so declare a Nex `go_sum`
source and extract the generated vendor archive into the source tree. Nex
found 138 required modules and produced the deterministic vendor archive
`fe9ca5dc9a9facae0dd53889b4b37a2900285d67fe5c41242f97bccfc02d8f99`.
Compose 5.1.0 has no `replace` directive in `go.mod`, which matters because
Nex's current `go_sum` source reader only parses `require` blocks.

The Soniq Yocto recipe still passes the old
`github.com/docker/compose/v2/internal.Version` linker key, while this source
declares module `github.com/docker/compose/v5` and imports its v5 `internal`
package. Set
`github.com/docker/compose/v5/internal.Version=5.1.0+gite8c21434` so the
installed command reports the release and snapshot commit honestly. Suppress
both Go and ELF build IDs as with Containerd and Docker. Install the plugin at
`/usr/lib/docker/cli-plugins/docker-compose`, not on `PATH`.

Docker Compose passes the strict two-build check with raw checksum
`6a17ef75da47077ae49afdc79b3d2f891415e62a63db67e894308147e58e0cd5`.
After its `full` bundle was overlaid on a retained Docker dependency root,
`docker compose version` printed `5.1.0+gite8c21434`. This proves the Docker
CLI can find the plugin in the packaged path and that the package reports the
snapshot commit honestly.

The GnuPG 2.5.17 stack needs newer low-level libraries than Nex originally
shipped. Libgpg-error 1.59 passes the strict two-build check with raw checksum
`d7b2b074c78e5e539a71d4f0c207f70212903f8100eea59ee5ad04bd23831411`.
Libassuan 3.0.2 passes it with raw checksum
`638d591a701981fe76bf6e28c65f2cb74961bf744b1fec8ff8f306ec5ff0a8bf`.
Both configure scripts probe for `hostname`, `file`, `cmp`, and `diff`, so
their manifests include Inetutils, File, and Diffutils in the build root.
Without those tools, configure continues with incomplete results and hides a
dependency rather than failing.

Libassuan 3.0.2 still generates `src/libassuan-config`, but `make install` no
longer installs that command. Its supported outputs therefore contain the
runtime library, development files, and Info manual, with no `bin` output.
Do not copy the older 2.5.7 bundle list into the 3.0.2 manifest.

Libgcrypt 1.12.1 installs runtime SONAME `libgcrypt.so.20.7.1`, which differs
from the older Nex output name. Keep upstream assembler support and capability
defaults; do not copy Yocto's `--disable-asm` or `--with-capabilities` flags.
The audited manifest passes the strict two-build check with raw checksum
`388442071370d6c234f476b863e3adfa048fbf7d3a6c5834ea765cffe6ce1f2d`.

NPTH 1.8 passes the strict two-build check with raw checksum
`695146ae2a0ec90e2accb471a507dd7e53704eec900ebb7bac8c004758f02632`.
Upstream installs `/usr/include/npth.h` directly. The Soniq Yocto package also
has `npth-64.h` because Yocto renames the architecture-specific header; Nex
does not need that packaging-only alias.

Libksba 1.6.8 passes the strict two-build check with raw checksum
`a274065b866c5d8bc78f82f752cfbd856d5ce565596e043299865586521b893c`.
Upstream generates `ksba-config` in its source tree but does not install it.
The Yocto development IPK adds that command as a packaging choice, so the Nex
manifest does not include a `bin` output for it.

A retained raw build root keeps one package's `bin`, `lib`, and `dev` outputs
separate below `/nex/out`, so a binary in `bin` cannot find that same package's
library in `lib`. Overlay the package's `bundles/full` ref onto the retained
dependency root before a merged-root smoke. The host `zub` on `PATH` may still
predate the checkout ownership fix and fail with `EPERM`; until it is updated,
use the tested binary at `/home/wegel/work/perso/zub/target/debug/zub` for this
overlay.

A successful compile, checksum match, or file-existence test does not prove
that a package works. Run a command-level smoke test against the built package
closure. Use `unshare --root <root>` when absolute package links must resolve
inside a disposable root. Compile and run a tiny consumer for a library when
the package primarily ships headers and libraries. Read `.agents/TESTING.md`
and `.agents/knowledge/package-manifests.md` for package-specific examples.

Do not wrap the whole `nex build` command in `unshare --user
--map-root-user`. Nex runs only the package build script inside its own root
namespace. The host-side Nex process must remain able to read and write the
zub store with the store's saved on-disk user map.

## Zub store ownership

Nex stores build results in the ignored `.nex/repo` zub store. A store created
inside a root-mapped user namespace must map logical root to the host user.
On this host, a fresh probe created these entries:

```toml
[[namespace.uid_map]]
inside_start = 0
outside_start = 1000
count = 1

[[namespace.gid_map]]
inside_start = 0
outside_start = 1000
count = 1
```

Use the current host UID and GID instead of assuming `1000` on another host.
Check `/proc/self/uid_map`, `/proc/self/gid_map`, and `.nex/repo/config.toml`
before changing a store map.

The following failure during dependency checkout points at an ownership-map
problem, not at a package manifest:

```text
io error at .nex/tmp/build_rootfs_<package>/usr/bin: EPERM
```

The failure occurred because zub stored directory owners as logical IDs but
checkout passed those logical IDs directly to `chown`. Blob-backed regular
files and symlinks already use on-disk IDs. Directory and special-file checkout
must translate logical IDs through `repo.config().namespace` first.

The upstream zub worktree lives at:

```text
/home/wegel/work/perso/zub
```

For this rootfs task, the human explicitly authorized agents to fix zub bugs
there, run the zub test suite, commit the fix using that repository's existing
subject style, and push it. Zub subjects usually use a short lowercase
imperative phrase, sometimes with a concrete command prefix such as
`transport:` or `commit:`. Nex pins zub by Git revision in
`src/cli/Cargo.toml`; update the lockfile and rebuild the ignored vendor tree
after pushing a required zub fix.

The checkout ownership fix is zub commit
`8027343ae5ee5db46c0969c2ebeaa721e4a6e0d6` on `origin/dev`. The zub checks
for that commit passed 193 tests across three suites, built the command, and
ran Clippy with no errors. Zub main had 19 pre-existing Clippy warnings. An
integration smoke used the rebuilt zub command to check Bash out of Nex's real
store as the host user and confirmed that the resulting `usr` directory had
owner `1000:1000`.

Zub commit `0db2a06aa0b741eb537ef41e5864226586aced38` makes
independently writable copies the checkout and export default. Use
`--hardlink` only when the caller guarantees that the checked-out tree stays
immutable. Nex follows the same rule: package and assembly materialization
uses copies, while `nex deploy` explicitly requests hardlinks for a finalized
read-only deployment. Zub also compares existing blob bytes and stored
metadata before deduplicating a write, so recommitting the correct source can
repair a blob changed through an older hardlinked checkout.

This rule protects the content-addressed invariant. Before the fix, an
Iptables test changed a hardlinked `ethertypes` checkout and replaced the
store object's bytes without changing its hash-shaped pathname. Ref deletion
could not repair it. The Zub regression suite passed 195 tests; Nex passed all
185 CLI tests, including a test that changes one checkout and reads unchanged
data from a second checkout.

The host's `/home/wegel/.local/bin/zub` can still be older than this fix. Until
that command is reinstalled, put `/home/wegel/work/perso/zub/target/debug`
first in `PATH` for Nex builds, assembly builds, and QEMU harnesses. An
`EPERM` during a direct-initramfs checkout reproduced immediately when the
QEMU wrapper found the stale installed command and disappeared when it found
the rebuilt command.

After Nex pinned that zub commit, all 155 Nex CLI tests passed. The normal
host-side strict build of Bash Completion 2.17.0 then completed both passes
with raw checksum
`9b5ee85942099ee315a1d32912c79e6fea6dcc9f1fbc6a4a7169b283ff483c39`.
Its runtime smoke started the packaged Bash under `unshare --root`, sourced
the packaged `bash_completion` file, verified version `2 17 0`, found the
`_comp_compgen_filedir` function and a registered completion, and checked the
compatibility symlink target. Bash Completion 2.17 renamed older helper names
such as `_filedir`; use the functions that the installed version defines.

## Current direct package gaps

The first direct generic packages found in `soniq-image.bb` or the deployed
rootfs manifest, but not in Nex when this runbook started, were:

```text
bash-completion 2.17.0
bridge-utils 1.7.1
containerd 2.2.2
docker/moby 29.3.0
docker-compose 5.1.0
cryptsetup 2.8.6
i2c-tools 4.4
libusb 1.0.29
smartmontools 7.5
tzdata-core 2026b
usbutils 019
vim 9.2.0340
gnupg 2.5.17
```

The reference root contains `/usr/bin/gpg`, but Nex had no GnuPG manifest when
this runbook started. Smartmontools' `update-smart-drivedb` uses GnuPG for its
normal signed update. The script can run with `--no-verify` without GnuPG, but
that fallback does not match the reference root or the default signed-update
path. Add GnuPG as a separate rootfs package rather than hiding the missing
tool in Smartmontools metadata. The warning script defaults to a `mail`
command, but the reference root does not contain that command, so do not add a
mailer solely for Smartmontools.

Do not substitute Podman for Docker while copying the rootfs. Soniq's OS3
scripts call `docker compose`, inspect `/var/lib/docker`, and repair Docker's
overlay2 state.

The reference image uses Linux 6.18.24. Nex used Linux 6.12.58 when this
runbook started. Update the kernel only after the missing user-space package
manifests compile. Rebuild every out-of-tree module against the new module SDK,
including Nvidia, v4l2loopback, kvmfr, and the Soniq hardware drivers.

## Generic-correct audit discoveries

The Soniq root includes libcap-ng 0.9.1. Smartmontools can use it, but its first
Nex manifest copied the Yocto choice that forced libcap-ng off. Nex now has a
normal libcap-ng package, and Smartmontools declares it and requires the
feature. The libcap-ng Git tag needs Autotools regeneration. Installing its
static archives made the first two builds differ; `--disable-static` and
removing libtool `.la` files produced a reproducible shared-library package
with checksum
`7e5701aa79a54ea5abc6afcd80d0724b52e380069ad5b8b7dae821f7d72c8222`.

When an upstream snapshot follows a release tag, include its commit in the Nex
package version instead of presenting it as the release. The Soniq sources
describe as containerd `v2.2.2-11-g5957d3334`, Docker Moby
`docker-v29.3.0-46-g1da6517e1a`, Docker Compose `v5.1.0-7-ge8c21434`, and
Tini `v0.19.0-15-g369448a`. Their Nex package versions use a `+git<commit>`
suffix, while the program version strings retain useful upstream describe
information.

Libgcrypt 1.12.1 must keep upstream's normal assembler implementations and
CPU feature checks. Do not copy Yocto's `--disable-asm` or capability override.
The package builds its manuals through Texinfo, deletes Libtool `.la` files,
and exposes a `full` bundle. Its strict two-pass build produced raw checksum
`c6f27945a70498f9df530209b58bf451192ab2307c9e8e30d1fd68d9cfb26709`.

Pinentry 1.3.2 does not accept `--disable-static`; configure warns and ignores
that option. The terminal-scoped Nex package enables ncurses, installs the
canonical `pinentry` link to `pinentry-curses`, and includes its Info manual in
the `full` bundle. Its strict two-pass build produced raw checksum
`b20c8a70c77d1433fe0e3ea1ea6978a326642a86b0b969e12d0eb57ba7fd5b16`.
Do not hand-copy dynamic-library needs from a host package: the first Pinentry
manifest claimed `libtinfo.so.6`, but `readelf -d` on the built binary proved
that it only needs `libncursesw.so.6`. `nex --compute-deps` merges existing
needs and does not delete a stale one, so remove disproven entries by hand.

Nex does not recursively expose development files from a package's own build
dependencies. GnuPG's GnuTLS check therefore needs explicit `dev` bundles for
Nettle, libtasn1, libidn2, libunistring, p11-kit, GMP, and libffi in addition
to GnuTLS itself. Without them, configure silently omits `dirmngr`. The GnuPG
manifest asserts `HTTP_USE_GNUTLS` and `BUILD_WITH_DIRMNGR` after configure so
that loss becomes a hard failure. GnuPG 2.5.17 then built reproducibly with
TLS, `dirmngr`, smartcard support, manuals, localization, and checksum
`321c8c84846536f948d41be3e4ff34ce29d752fca4762fda582b57d2f1ea9120`.
LDAP remains auto-detected and absent because Nex has no OpenLDAP package; the
GnuPG manifest does not copy Yocto's explicit LDAP feature cut.

GPGME 2.0.1 installs `libgpgme.so.45`, not the `.so.11` ABI from the older
manifest, and current Libassuan supplies `.so.9`, not `.so.0`. Verify ABI
updates with `readelf -d` because generated needs retain obsolete hand-written
entries. The package keeps the upstream-detected Common Lisp files and command
tools, deletes Libtool `.la` files, and built reproducibly with checksum
`8473799fe1540e39531cac7eca6f943159db0b8b3b8054bb13ad8e9175ff02e4`.

Podman 5.4.1's upstream `docs` target renders each generated page with
`man -l` and checks the rendered text with `grep -P`. The Soniq rootfs has no
`/usr/bin/man`, and its deployed package manifest names no `man-db` or
`mandoc` package, so this is a build-tool need rather than a Soniq runtime
package. Nex's GNU grep 3.11 manifest did not supply PCRE2 and also edited
`src/egrep.sh` to suppress upstream behavior. A generic GNU grep package should
use the existing PCRE2 package, keep upstream's script unchanged, and include
its Info and man pages. Prove the result by matching a Perl-compatible pattern
with the installed `grep -P` command.

GNU grep's configure script also calls `cmp`. Without the bootstrap Diffutils
bundle, configure printed `cmp: command not found` and incorrectly reported
that `perror` did not match `strerror`. Add Diffutils even though the final
binary checksum happens to stay unchanged. The corrected GNU grep 3.11 package
built reproducibly with raw checksum
`2cdcb959123292ae9f1a66848981b854b89bdb42d110f15453b56fa660e93e55`.
After checking the new `full` bundle over the retained build root, an isolated
smoke matched `a(?=b)` with `grep -P`, reported GNU grep 3.11, and confirmed
that upstream's `egrep` obsolescence warning remains.

The root `AGENTS.md` names `RUST_CODE_STYLE.md` at the repository root and
`.agents/MANIFESTS_CODE_STYLE.md` in the tracked agent tree. Read the latter,
`.agents/knowledge/package-manifests.md`, and `.agents/TESTING.md` for manifest
work. Read `RUST_CODE_STYLE.md` before Rust changes.

Portable mandoc 1.14.6 supplies a complete upstream `man` implementation whose
`man -l FILE` mode matches Podman's documentation check. Its configure script
defaults to `cc`, so Nex must declare the documented `CC=gcc` and `AR=gcc-ar`
settings in `configure.local`. Pass Nex's C and linker flags there too. Set the
documented `PREFIX`, `SBINDIR`, and `MANDIR` paths so the package installs under
`/usr`, puts `makewhatis` in Nex's usr-merged `/usr/bin`, and keeps manuals in
`/usr/share/man`. Let the script detect features instead of forcing cache
answers. The package built reproducibly with raw checksum
`4d14ee71d2dcc7dee33fcb26bb27b46d0ccdb600898b29e9ad441ba444795083`.
An isolated smoke rendered plain and gzip-compressed pages with `man -l`, built
a search database with `makewhatis`, and found the page with `whatis`.

Mandoc's probe also exposed a rootfs gap: the current glibc build root reports
only the `C` and `POSIX` locales, so upstream mandoc disables wide-character
output. The Soniq image manifest includes `locale-base-c`,
`locale-base-en-gb`, `locale-base-en-us`, and `glibc-locale-en-gb`. Nex's glibc
package installs locale source definitions but does not compile any locale.
Treat compiled glibc locales as separate, explicitly scoped packages; do not
force mandoc's wide-character probe to pass without a runtime locale that
supports it. The reference root's locale archive contains only `en_US.utf8`,
despite the broader Yocto package names in its deployed package manifest.

`glibc-locale-en-us` 2.39 compiles the normal `en_US.UTF-8` locale as unpacked
locale data. It does not install `/etc/locale.conf`, set `LANG`, or choose a
system default. An assembly can include the package and separately choose the
default locale. Build it against Glibc's existing `outputs/misc` localedata
ref instead of copying Yocto's generated package layout. Glibc compresses its
charmaps, and `localedef` reports them as missing when the build root lacks
`gzip`; declare Gzip as a build dependency. The package passes the strict
two-build check with raw checksum
`1b8725c7a609b9bf174289d90b8771f5df0ee976ffaecbec39c59c652b48bc66`.
An isolated root listed `en_US.UTF-8`, reported the `UTF-8` charmap, and returned
the expected decimal point, thousands separator, and dollar currency symbol.

Podman 5.4.1 must install upstream binaries, remote client, Quadlet generators,
systemd units, completions, and manuals without installing a registry list,
signature policy, storage policy, service preset, or enabled-service symlink.
An assembly owns those product and administrator choices. Let the upstream
Makefile probe Nex's libraries: the resulting binary links GPGME ABI 45,
libseccomp, and libsubid, and the upstream tags omit only the unsupported
device-mapper graph driver. The strict two-pass build produced raw checksum
`67bc3955387b1750880d4192867ee4db491f6c579788e15b6c959ce58bcd3487`.
Runtime checks reported Podman, Podman Remote, and Quadlet version 5.4.1,
rendered `podman(1)` through Mandoc, parsed the installed Podman systemd units,
and converted a sample `.container` file into a complete systemd service.

An isolated runtime root may lack permission to mount `/proc`, which makes Go
programs such as Podman fail while reading `/proc/self/cmdline`. For version and
generator checks on the same CPU, run the target executable through the target
glibc loader, for example `ROOT/usr/lib/ld-linux-x86-64.so.2 --library-path
ROOT/usr/lib ROOT/usr/bin/podman --version`. Do not combine the target libc with
the host loader through `LD_LIBRARY_PATH`; that mix can abort with stack-smashing
errors. A full container-engine smoke still needs a booted or privileged root
where the kernel exposes namespaces, cgroups, and `/proc` normally.

Nftables 1.1.1 can disable manual regeneration and install the generated manual
files shipped in the official release archive. That keeps the complete runtime
manuals without adding a DocBook formatter to the build root. The package keeps
upstream examples, JSON support, readline, OS fingerprint data, and the shared
library; it installs no active firewall rules. Its strict two-pass build produced
raw checksum `d82c8ef9a12067a13ea84db84a90c061820fcecf70c2a66251bb912c634d78ca`.
An isolated smoke reported nftables 1.1.1, accepted an empty file through
`nft --json --check`, and rendered `nft(8)` through Mandoc.

Vim 9.2.0340 uses upstream's default huge terminal build with multibyte text,
syntax files, help, translations, terminal support, ACLs, and `xxd`. It does not
install a system vimrc, user vimrc, branding string, or assembly-specific default.
The local reproducibility patch only clears compiler commands, build user, and
build host from `vim --version`; it leaves Vim's features and paths unchanged.
Its strict two-pass build produced raw checksum
`03cfbed73d35992c81820af919b3781cc186d4ef460641eaab9f94cca1f13495`.
An isolated smoke used Vim in ex mode to write a file and round-tripped that file
through `xxd`; `vim --version` reported the huge, multibyte build with blank
compilation and linking identity fields.

Containerd 2.2.2 snapshot `5957d3334b` follows upstream's `binaries` target
and installs the same three programs as the official runtime archive:
`containerd`, `ctr`, and `containerd-shim-runc-v2`. The package records the
snapshot in both the Nex version and the programs' version output. It installs
upstream's systemd unit with only usr-merge path fixes, and it does not install
a generated `config.toml` or enable the unit. Administrators and assemblies can
generate an upstream default with `containerd config default` and override it
under `/etc/containerd/conf.d`. The strict two-pass build produced raw checksum
`3989b9f2204da1014a248d207bce55b87e08e8ffc2685b21ce140dc30f99c47d`.
Runtime checks reported the exact source revision from all three programs,
generated a complete version 3 default config, and parsed the service unit.
`systemd-analyze verify --root=ROOT` needs Systemd's `full` bundle overlaid into
`ROOT`; a package root that contains only the tested unit lacks `sysinit.target`.

The final generic-correct audit removed forced feature choices that merely
copied downstream recipes. MIT Kerberos now lets upstream choose its bundled
`com_err` and `ss` implementations and auto-detects LDAP and LMDB; Nex has no
OpenLDAP or LMDB development package, so those optional backends stay absent.
Its strict two-build checksum remains
`2ac17a3d9da4923d94c98680a9e68661ecc28517bb01c6c0e04db3ef665e9e95`.
Libdevmapper lets configure auto-detect SELinux and use upstream defaults for
dmeventd and `thin_check_needs_check`; the manifest still builds only `libdm`,
its commands, and its udev rules. Its checksum remains
`af58b78df0201a9521e6b1c94ffa4a7b084e8308b55e0ee08410a5a6af97f0e3`.
Cryptsetup also auto-detects SELinux instead of forcing it off. Its checksum
remains
`7e5a864fc4cac7d849282c77195df3bcb5e9867aead2d105a87e25fd3a4fb590`,
and an isolated runtime root completed a PBKDF2-SHA256 benchmark.

The audit found the words `BitBake` and `bitbake` only in Vim's own upstream
runtime files. Keep those files: a normal full Vim package ships upstream's
syntax and filetype support, including BitBake support. The touched manifests
contain no Soniq branding, Yocto package names, service presets, chosen system
locale, chosen local timezone, or active firewall and container policy. Tzdata
does pass `America/New_York` to `zic -p`; upstream tzdata uses that historical
POSIX-rule default, and the package still creates no `/etc/localtime` or
`/etc/timezone`.

## Kernel builds

Linux 6.18's `CONFIG_IKHEADERS` target calls `md5sum` while it creates
`kernel/kheaders.md5`. The Coreutils `bundles/dev` bundle does not include the
checksum programs, so the Linux manifest must also depend directly on
`x86_64/pkg/core/userland/coreutils/9.5/outputs/checksum`.

The Soniq rootfs uses Linux 6.18.24. Kernel.org publishes the source at
`https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-6.18.24.tar.xz`; its signed
SHA256 list records
`c207c557ce58103b4dda30e26da5203f3d8467c6dadc53d709f6d83ae1d1255f`.

`pkg/core/kernel/linux.yaml` carries hand-curated module outputs and bundles.
Do not pass `--generate-outputs` to an ordinary kernel build because that flag
can replace those names with a generic split. Run `pkg/core/kernel/build-kernel.sh`
when a kernel version changes. The helper temporarily comments out the bundles,
builds only Linux with `--single`, sends the full zub tree listing through
`categorize-modules.py`, restores the bundle names, and refreshes their
metadata. The categorizer needs every non-directory path, not only `.ko`
paths. Otherwise it silently drops the `module-sdk` output, kernel boot files,
depmod metadata, and kernel size reports. The parser accepts a raw `zub
ls-tree -r` listing and skips its directory entries; output file lists cannot
name directories. The temporary YAML has null `bundles` and `outputs` values,
so `nex check` and `nex format` reject it until the helper reaches its restore
step. If a build fails while the YAML is prepared, fix the package and rerun
the helper with `SKIP_PREPARE=1`; do not run `prepare` twice.

Linux 6.18 introduced the `transitional` Kconfig symbol attribute. The pinned
Kconfiglib revision does not parse that keyword, and the current Kconfiglib
heads did not yet contain support when checked on 2026-08-13. Keep the small
`pkg/core/kernel/kconfiglib-transitional.patch` build-time compatibility patch
until upstream Kconfiglib implements the same behavior. The patch parses the
keyword, keeps old config values available while Kconfig resolves renamed
symbols, and omits transitional symbols from new `.config` output. This support
serves every Linux 6.18 build; it does not copy a Soniq or Yocto package choice.

`categorize-modules.py` must derive the kernel release from the supplied
`/usr/lib/modules/<release>/...` paths and reject a stream that mixes releases.
Do not hard-code the release in generated boot or module-metadata paths. Linux
6.18 moved HDA modules from `sound/pci/hda` to `sound/hda`; keep both layouts
mapped to `drv-sound-hda`. After regenerating outputs, compare every bundle
member with the output names before refreshing zub metadata. Linux 6.18 has no
ReiserFS or DCCP output, while `all-modules` must include the new `drv-cpufreq`,
`drv-crypto-hw`, `drv-rtc`, `fs-9p`, `fs-nfs`, and `fs-smb` groups.

RTK wrappers can change command output to make it easier for a human to read.
When a pipeline passes machine-readable text, use `rtk proxy` for commands such
as `awk`, `grep`, and `python3`. In particular, piping a zub tree through `rtk
grep` hid module paths and produced a false zero-module result.

RTK also treats `rtk test` as its own test command rather than the shell's
`test` executable. Use `rtk proxy test` for file and directory predicates.

Linux 6.18.24 passed a strict clean two-build check with raw checksum
`dc165be67c7a0b89b4e4ba36fc0432969b01843047f13554e449bb9513278775`.
The retained module SDK contains a nonempty `.config` and `Module.symvers`,
executable `scripts/mod/modpost` and `tools/objtool/objtool`, and correct
`build` and `source` links. Keep `CONFIG_TRIM_UNUSED_KSYMS` disabled so an
out-of-tree module can use normal exported kernel symbols. The fresh build also
contains `kernel/kheaders.md5`, which proves that the direct Coreutils checksum
dependency supplies the `md5sum` call from `CONFIG_IKHEADERS`.

Every package that builds a module against the prepared SDK must declare
`x86_64/pkg/dev/util/elfutils/0.191/outputs/lib` directly. The SDK ships its
built `tools/objtool/objtool`, and that program needs `libelf.so.1` while an
external module builds. The SDK bundle cannot express that host-tool runtime
dependency by itself.

V4L2Loopback 0.15.4 is the current stable upstream package that builds against
Linux 6.18.24. Its tag archive has SHA256
`21a17702648aa6a937b88a93bd71ef9f547815ead28719cafcfcc247396643dc`.
The current source no longer needs the old local `strlcpy` replacement. Its
userspace control tool still needs Linux userspace headers, and its Makefile
defaults to an absent `cc`; declare the Linux headers bundle and pass `CC=gcc`.
Its strict two-build checksum is
`bfdc1e059313fb5f461edc30c6eab4c70d6dc13f00a286ef35303f39254885cc`.
The packaged control tool prints its complete command help, and the module
reports `name=v4l2loopback` with `vermagic=6.18.24 SMP modversions`.

Kvmfr B7 passed its strict two-build check with checksum
`03e54ea2173dab1a1f087b6953e92c4a0f346495edb8ef70dc25fd8c5afdb9e1`.
Its upstream Makefile can print a harmless missing-user-name message for UID 0
and a `MODULE_DESCRIPTION()` warning. The output remains reproducible, so do
not patch those diagnostics into package behavior. The module reports
`name=kvmfr` with `vermagic=6.18.24 SMP modversions`.

Nvidia 580.159.04 passed its strict two-build check with checksum
`58b3a59dc14fcc68b8d755b4a153d7b5e489548e2b803f4a7f1cc85897d2885a`.
Nvidia Current 595.84 passed with checksum
`1bf7490765156c555325cfd1efa3ff26466e3f20af023677085a88f057b97b06`.
Both packages produce `nvidia`, `nvidia_modeset`, `nvidia_drm`, and
`nvidia_uvm` modules with Linux 6.18.24 vermagic.

External module packages must install only their own `.ko` files under the
kernel release directory. Do not run `depmod` inside one package and do not
ship partial `modules.dep`, `modules.alias`, or related index files. An assembly
that combines the kernel and all external modules runs `depmod` once over the
complete module tree. Direct `zub ls-tree -r` checks proved that V4L2Loopback,
Kvmfr, and both Nvidia packages follow this rule for Linux 6.18.24.

When a package version changes, rebuild it so zub has the new semantic ref and
search every package and assembly for consumers pinned to the old version. A
manifest edit alone does not create the new store ref. The Libgcrypt 1.12.1
update left Gnome Keyring, GCR, and GTK-VNC pinned to 1.11.3; a desktop
assembly caught the stale refs while it flattened shared libraries. Strict
rebuilds after correcting those refs produced checksum
`e1dc7ae0dc2d9f09a095078333ef19ec4237a292a4c524a55a21452168dc691c`
for Gnome Keyring 50.0,
`b521a5205ae2eada12b4a4ecc41a8f40c45509f7d13c7f39c65963ebc707605d`
for GCR 3.41.2, and the unchanged checksum
`94acacb61fd92db377d2f8b79c6bf2ca3123693bdc6f1daded03dc249ed9ec47`
for GTK-VNC 1.3.1.

The Linux 6.18.24 assemblies passed strict two-build checks with these raw
checksums:

- `flat-systemd`: `81ab4534c7d95cdef2efa73811e9e3a0180aeb15ddf50bb64fa53ea770d85416`
- `nex-systemd`: `db42ef0410942ae52d324f2f6ed25bdfe8656f894ec2788e7a4e9d23b95faabc`
- `installer`: `a4c0c1943fb92e8d759e14248508768cafe8bd3e48986091ae98d1ac693c558a`
- `desktop-vwl`: `fbf8b026ba422197b630aa0d309fe464463d8ae25f48f89694a0c00d8d0e77e4`
- `desktop-vwl-nvidia-580`: `7ba45f32f05c2df34a5786673701dd60663eef3b5a42db9be2ecb520ad1e216c`
- `desktop-vwl-nvidia-current`: `9ceff1d69b5c68514be6ac3db7f88d7848651549fb5c326cda8111e5b6691e0b`

Both Nvidia assemblies must consume V4L2Loopback 0.15.4. Their build scripts
run `depmod` over the complete Linux 6.18.24 tree, inspect every external
module with `modinfo`, run `modprobe --show-depends`, and require index entries
for Nvidia, V4L2Loopback, and Kvmfr. A stale V4L2Loopback 0.12.7 assembly ref
made this check fail even though the new package itself had passed.

`scripts/qemu-test-systemd.sh` is a small wrapper around the maintained
installer harness. It defaults to `systems/flat-systemd/0.0.1`, sets the Linux
6.18.24 boot ref, and calls
`qemu-test-installer.sh --direct-initramfs --assert-boot --headless`. The guest
assertion parses the `zub=` kernel argument with shell builtins because the
minimal Flat Systemd root does not contain `sed`. The test booted Linux
6.18.24, selected the requested deployment, mounted root and `/var`, started
Systemd, printed `ASSERT-BOOT-PASS`, and powered QEMU off.

## Edgebox rootfs assembly

Use `asm/flat-systemd.yaml` as the parent for a conventional root filesystem.
It produces merged `/usr`, `/etc`, and `/var` trees without Nex deployment
links, so its artifact is the closest existing base for comparison with the
rootfs-builder image. An `extends` path resolves from the repository root, so
write `asm/flat-systemd.yaml`, not a path relative to the child manifest.

The system builder materializes packages, applies overlays, and then runs the
merged build script. Inherited build scripts run before the child script. A
child overlay therefore cannot replace `/etc/passwd`, `/etc/group`,
`/etc/shadow`, or `/etc/os-release` from `flat-systemd`, because the parent
script rewrites those files afterward. Put service units, links, and ordinary
configuration in the overlay, but write those four files in the child build
script.

Keep package manifests generic. Put the chosen locale, timezone, enabled
services, users, network policy, and system-wide PipeWire policy in
`asm/edgebox-rootfs.yaml` or `asm/edgebox-rootfs-overlay.yaml`. Do not put
product users, service presets, firewall rules, Docker policy, proprietary
modules, or Yocto package splits into reusable package manifests.

The Soniq image installs ACPI event daemon 2.0.34. The official SourceForge
archive has SHA256
`2d095c8cfcbc847caec746d62cdc8d0bff1ec1bc72ef7c674c721e04da6ab333`.
Its source still calls `stat64` and `fstat64`; replace those calls with `stat`
and `fstat`, which work with the package's existing large-file configure probe
and current glibc. The package passed its strict two-build check with raw
checksum
`641601483183db9647d3a757d38136f26d6207310306d36e2c77cc7e00a543db`.
The packaged `acpid --version` prints `acpid-2.0.34`.

OSTree 2026.1 comes from the official libostree release archive with SHA256
`8e77c285dd6fa5ec5fb063130390977be727fe11107335ed8778a40385069e95`.
Its generic Nex build enables Curl, GPGME, Libarchive, libmount, Systemd,
fs-verity, and rofiles-fuse. Nex has no ComposeFS, SELinux, or Avahi development
manifest, so configure leaves those optional integrations off. The strict
two-build checksum is
`ae5ab2c3f341805627a56ea799965fe1e3fa7ed6943687b5a9d02347d3e2e9d2`.
A command smoke initialized an archive repo, committed a tree, and completed
`ostree fsck` with one commit and no errors.

OSTree exposed stale `libarchive.la` metadata in the Libarchive package.
Libarchive must delete installed `.la` files and declare Diffutils and File as
direct configure dependencies. Its corrected strict-build checksum is
`97c0d3099322384b7e5e50284487de50bc5b398e2b9b4a0a41b614de15e61a35`.

The reference root uses `en_GB.UTF-8`. Keep that choice out of glibc itself.
`pkg/libs/system/glibc-locale-en-gb.yaml` builds a reusable locale output with
`localedef --no-archive`; its strict-build checksum is
`99afaf4389f092d5a88e8b09d894b1b11baa8f4289ec77282c9686e03da07ff6`.
The assembly installs that output, writes `LANG=en_GB.UTF-8`, and links
`/etc/localtime` to the tzdata `Universal` zone.

The reference selects a long list of ordinary in-tree kernel modules. Use the
Linux `all-modules` bundle for the first rootfs replica instead of copying
Yocto's split kernel-module package names into Nex. The reference's Phoenix3
and RTL8168H firmware files live inside Nex's generic `gpu-amd` and
`wifi-realtek` Linux firmware outputs. Proprietary Soniq and vendor modules
remain explicit gaps.

Always pass `--single` while strictly rebuilding one package or assembly. A
plain `nex build` also walks the dependency graph and can refresh unrelated
profiles, generated output metadata, or checksums. One broad build refreshed
dmidecode's reproducible checksum to
`34129eb04f07b5591089a00a22f90dfecd5c62d85e4c477c1e5efb430eca04b6`;
a later strict two-build run reproduced that value.

A mistaken broad Edgebox check with `--update-checksum` also added transient
checksums to 20 `pkg/bootstrap/phase0/` seed manifests while it rebuilt the
closure. Those edits were reverted. Use
`./nex build asm/edgebox-rootfs.yaml --single --check --update-checksum
--verbose` for the assembly gate.

The final timestamp walk in a flat assembly must use `touch -h`. Plain `touch`
follows absolute links such as `/etc/resolv.conf` and tries to change a path
outside the target root. After this fix, `flat-systemd` passed its strict
two-build check with the checksum listed above.

Smartmontools' warning helper probes `domainname`, `nisdomainname`, and
`dnsdomainname` when they exist, then falls back to `hostname`. Those three
commands are optional probes, not hard runtime needs. Keep only `hostname` in
the generated needs and provider map. The package reproduced its existing
checksum after removing the false needs.

Systemd's normal upstream tool set includes `systemd-analyze`. Build it in the
generic Systemd package rather than adding an assembly substitute. After the
UAPI vendor-path work, its strict two-build checksum is
`c9ca9f12fa799ce8632962dbb302578e239f1c04c7c1e3471d242af95dedb7ce`.

`nex format` must recognize an overlay whose root key is `files`. The old
package-or-system choice formatted such a file as an empty document. The
formatter now preserves overlay entries, their comments, field order, and the
exact newline suffix of inline content. It rejects unknown root keys and
unknown overlay fields instead of silently dropping them. Focused tests cover
the original erase case and YAML block-scalar newline behavior. After the
third-party repository work, the full CLI suite passes 173 tests.

`asm/edgebox-rootfs.yaml` and `asm/edgebox-rootfs-overlay.yaml` make a generic
appliance-style flat root from reusable packages. The assembly chooses its
locked root account, service accounts and groups, DHCP through networkd, resolved,
timesyncd, SSH, ACPI, container services, system-wide PipeWire wrappers,
`en_GB.UTF-8`, and the Universal timezone. The package manifests retain normal
upstream units and defaults. GNU Grep must be an explicit package because the
rootfs policy checks and ordinary operator scripts call it.

The strict two-build checksum for `edgebox-rootfs` is
`33c4f85c50e257f0aa5869a6f66d8b5fdb79293f88773c94c91a5ca2d85fbe6c`,
stored at `systems/edgebox-rootfs/0.0.1`. Its retained checkout lives at
`.nex/tmp/build_rootfs_edgebox-rootfs_system/target`.
`scripts/test-edgebox-rootfs.sh` enters a user, mount, and PID namespace;
bind-mounts the host `/dev`; mounts `/proc`; and then chroots. A real `/dev/null`
matters for the packaged Docker Compose command.
The script runs packaged commands, verifies policy and unit links, checks the
vendor Shadow and PAM files, asks `systemd-analyze` to parse the added audio
units, inspects Linux 6.18.24 module and firmware metadata, and commits plus
fscks an OSTree repository. Its final run printed `PASS: Edgebox rootfs smoke
test`. Current Zub versions copy checkouts by default, so a checkout below
`/tmp` works across the filesystem boundary and remains safe to modify.

`scripts/qemu-test-systemd.sh systems/edgebox-rootfs/0.0.1` booted Linux 6.18.24
and Systemd 257.5, selected the requested store deployment, mounted root and
`/var`, printed `ASSERT-BOOT-PASS`, and powered off. The direct BIOS-style run
also logged an EFI automount failure, but local files reached their target and
the explicit guest assertion passed.

The Nex root contains 19,857 regular files, 1,047 links, 2,050 directories,
and 1,066,645,501 file bytes. The reference root contains 7,203 regular files,
1,026 links, 990 directories, and 948,352,164 file bytes. Nex is larger mainly
because the first replica keeps the complete Linux `all-modules` bundle and
broader firmware and package outputs.

The direct ordinary package list from `soniq-image.bb` is now covered except
for OpenTelemetry Collector, which remains a generic upstream package worth
adding. The other direct gaps are private or product-specific: the OS3 and
Soniq agents, integrity and boot helpers, firewall and environment policy,
TouchTunes PipeWire and kernel modules, kiosk code, the diagnostic agent, and
the proprietary General Touch driver. Keep those gaps out of generic package
manifests. Most remaining enabled unit-name differences come from those
product packages or from Yocto's package-time links. The Nex assembly uses
dedicated system-level audio wrapper units while the generic PipeWire and
WirePlumber packages retain their upstream user units.

## UAPI.6 configuration audit

UAPI.6 defines a public Linux convention for immutable vendor defaults under
`/usr`, optional temporary overrides under `/run`, and host overrides under
`/etc`. A local main file replaces a lower-priority main file. Drop-ins sort by
filename across all three trees, and a same-name file in the higher-priority
tree shadows the lower one. Empty files and links to `/dev/null` mask files.
Scripts and structured documents may omit drop-ins when combining them is not
safe. Package patches that implement these rules remain generic when they use
no Nex or product paths.

`PHILOSOPHY.md` now makes this convention a Nex principle. A managed machine
keeps persistent host-owned `/etc` outside the selected read-only deployment,
while programs read changing vendor defaults from that deployment's `/usr`.
Upgrades and rollbacks select a `/usr` tree without merging or replacing local
files. Reusable package manifests enable upstream vendor-directory support or
carry generic UAPI patches; immutable assemblies move legacy `/etc` defaults
to `/usr/share/factory/etc` and may populate only missing host files.

Keep a factory fallback for programs that still read only `/etc`. A Nex
immutable assembly can place their pristine defaults under
`/usr/share/factory/etc`, then reconcile those defaults into persistent
host-owned `/etc`. Keep this transformation out of reusable package manifests;
ordinary flat assemblies must still receive upstream's normal files.

The system builder performs that transformation only when an assembly sets
`nex_structure: true`. It runs after package materialization, overlays, and the
assembly script, moves the completed `/etc` tree to
`/usr/share/factory/etc`, and leaves an empty `/etc` mount point. If an
assembly or package already supplied native factory files, the builder merges
directories but fails on every file, link, or type collision instead of
choosing one silently. `nex-systemd` passes its strict two-build check with
raw checksum
`db42ef0410942ae52d324f2f6ed25bdfe8656f894ec2788e7a4e9d23b95faabc`.

When an upstream package already installs a native
`/usr/share/factory/etc` file, an assembly that chooses different initial host
content must write the factory path and set `replace: true`. Writing the same
logical file below `/etc` creates a collision when the builder moves the final
`/etc` tree. The desktop assembly uses this explicit rule for `pam.d/other`
and `pam.d/system-auth`, which Systemd also supplies as upstream factory
defaults.

The initramfs keeps host `/etc` in `/var/etc`, whether `/var` has its own
partition or remains on the root filesystem. On every boot,
`/bin/nex-populate-etc` recursively copies only missing paths from the active
deployment's `/usr/share/factory/etc`. Existing empty files, links, files, and
directories remain host-owned. It falls back to the deployment's `/etc` for
older deployments. Do not replace this helper with BusyBox `cp -a -n
SOURCE/. DEST`: when a destination directory already exists, that command
skips the whole directory and misses new nested vendor files.

The helper regression test starts with a changed SSH file, an empty `hosts`
file, and a directory masked by a `/dev/null` link. It proves those choices
survive, then adds a new nested factory file and proves a second run copies
only that file. The direct QEMU boot test starts persistent `/etc` with only a
host-owned `hosts` file. The guest proved that the initramfs added `passwd`
from the factory tree, kept `hosts` unchanged, mounted `/etc` from `nex-var`,
booted Systemd, and printed `ASSERT-BOOT-PASS`.

The exact versions used by Edgebox have these verified capabilities and gaps:

The first exhaustive 2026-08-14 scan found 41 manifests that declare at least
one output below `/etc`. The first UAPI package batch reduced that count to 36
by removing those outputs from Shadow, Fish, Systemd, Swaylock, and Polkit.
The Bash and e2fsprogs batch reduced it to 35 because Bash did not declare the
assembly-owned profile and e2fsprogs moved its two package defaults. BlueZ,
PulseAudio, OpenSSH, and Glibc reduce the current count to 31. This is an audit
queue, not 31 mechanical moves. It includes
bootstrap copies, two Nvidia versions, generated certificate links,
compatibility links, and files whose external specification still names
`/etc`. Empty directories created for host configuration do not need a
vendor-file conversion.

Do not blanket-change `--sysconfdir=/etc` to a path below `/usr`. That option
usually tells a program where the administrator writes local configuration;
changing it can erase the `/etc` override instead of adding a vendor default.
Prefer an upstream vendor-directory option, keep the administrator directory
as `/etc`, and test both paths. When upstream has no such option, patch the
program's lookup order or leave the file in the assembly factory fallback.

For required XDG application defaults, keep explicit and user paths first,
preserve the ordered absolute entries in `XDG_CONFIG_DIRS`, then add a
temporary `/run/xdg/<package>` path and a vendor
`/usr/share/xdg/<package>` fallback. Waybar 0.15.0 follows this order while
retaining its legacy `$HOME/waybar` path. Its test selects files from every
tier and proves that an empty administrator file masks lower files. An
assembly must select the output that contains these defaults; selecting only
Waybar's binary omitted both files until `desktop-vwl` used `bundles/full`.

Do not treat `/etc/xdg/autostart` like a package's private XDG default
directory. The Freedesktop Autostart Specification tells desktop sessions to
scan `autostart` below the user and system XDG configuration directories, and
the Base Directory Specification defaults the system list to `/etc/xdg`.
Keep standards-defined system autostart entries at `/etc/xdg/autostart` unless
the target desktop has an explicit alternative. Gnome Keyring 50.0 and
AT-SPI2 Core 2.54.0 use that contract.

An AT-SPI2 runtime needs more than `libatspi`. Its full bundle must include
the bus launcher, registry daemon, D-Bus activation files, systemd user
service, autostart entry, and default accessibility setting. Dependency
flattening can make libraries available to another capsule, but it cannot
publish the package's activation programs and metadata in the assembled
root. `desktop-vwl` therefore selects AT-SPI2's corrected `bundles/full`.

Move an upstream configuration sample below
`/usr/share/doc/<package>/examples` when it contains no active package choice,
but do not change the real administrator reader just to move that sample.
p11-kit 0.25.5 explicitly calls its installed file an example and continues
to read `/etc/pkcs11/pkcs11.conf`. FUSE 3.17.4 installs a fully commented
template while `fusermount3` continues to read `/etc/fuse.conf` for
administrator `user_allow_other` and `mount_max` choices. Their strict package
checksums are
`ab9167443ea2da82819d9545033c4db0d3a401ffcacdb47ca283c8129f878670`
and
`9829fa1b95f94bc0a3185e52c6b618b5b1d5e9eb6a26b51edbdd0b5536c79047`.

Do not move shell `profile.d` hooks one package at a time. Bash does not scan
that directory itself; the selected system profile decides which fragment
directories it sources. Bash Completion 2.17.0 documents
`$sysconfdir/profile.d/bash_completion.sh`, and VTE 0.76.4 installs both of
its hooks to `vte_sysconfdir/profile.d`. Keep `/etc/profile.d` until the shell
profile contract adds a vendor directory for every participating package and
assembly.

Bash Completion also reads `/etc/bash_completion.d` itself when
`BASH_COMPLETION_COMPAT_DIR` is unset. Its documentation calls that directory
the first default compatibility path, and user startup files may still source
the historical `/etc/bash_completion` entry point. Keep both paths as
compatibility interfaces. The current package checksum is
`9b5ee85942099ee315a1d32912c79e6fea6dcc9f1fbc6a4a7169b283ff483c39`;
VTE reproduced with checksum
`92157350d5c80cc7991d8166e6187e5e94f5219b4d971e7fc5345bc0ab7cbc27`.

Attr 2.5.2 caches its parsed `xattr.conf` action list for the life of a
process. Test its UAPI tiers with a fresh production-linked process for each
case. The generic reader uses whole-file priority
`/etc/xattr.conf`, `/run/xattr.conf`, `/usr/lib/xattr.conf`; an empty higher
file masks lower rules. The strict package checksum after this patch is
`2fba331aba23c967ea505c421dafcf8292abd130d4b69a37077e2377599e7d1f`.

Coreutils can link libattr through its package closure without publishing
Attr's split `conf` output. Assemblies that supply file-copy tools must select
`x86_64/pkg/libs/system/attr/2.5.2/outputs/conf` explicitly. A checked-out
root can prove the real behavior by setting `user.keep` and
`user.Beagle.*` attributes on a source file, running the packaged
`cp --preserve=xattr` in a rootless chroot, and checking which attributes
reach the destination.

Slsh 2.3.3 loads one system `slsh.rc` before its optional user startup file.
Keep `SLSH_CONF_DIR` and the older `SLSH_LIB_DIR` alias as explicit
single-directory overrides. Without either variable, the generic reader
selects the configured administrator directory, `/run`, then `/usr/lib`; an
empty selected file masks lower files. Run a fresh Slsh process for each
startup-file test. The strict package checksum is
`9dbe12cf16e3dbdf85decc1ba685c5f27dfd8afa7924f02cd10e64947d5cc1be`.

After a strict assembly build commits a system, its temporary `target` tree
may no longer exist. Use `zub cat-file systems/<slug>/<version>:<path>` to
inspect a directory, symlink target, or regular file in the durable system
commit without checking out the full root.

The first high-value batch enabled Shadow's libeconf vendor directory, added
Linux-PAM's transient service-policy directory, moved PAM services from
Shadow, OpenSSH, Swaylock, and Polkit, added Fish's transient fragment
directory, and moved Systemd defaults to its native vendor paths. The current
parser wave has finished BlueZ, PulseAudio, OpenSSH, and Glibc. Keep
account databases, machine identity, and other true host state out of reusable
package defaults.

Netavark 1.14.1 supplies the network helper, not a distribution firewall
choice. The old manifest created
`/etc/containers/containers.conf.d/50-buildroot-nftables.conf` entirely in its
build script even though upstream supplied no such file. The generic package
now ships only Netavark and lets Podman or an assembly choose a firewall
driver. Both pinned inputs and `netavark --version` identify 1.14.1; keep the
manifest version and assembly refs aligned with that source identity.

The OpenCL ICD extension fixes the Linux vendor-file directory at
`/etc/OpenCL/vendors`. Keep Nvidia's `nvidia.icd` there; moving it below
`/usr` would break conforming loaders. Both Nvidia manifests generate a
separate `conf` output, so their `full` and `runtime` bundles must list that
output explicitly. Before EP012 added it, Nvidia desktop images carried
`libOpenCL.so.1` but no discoverable Nvidia ICD. The Khronos contract is
`https://registry.khronos.org/OpenCL/specs/unified/refpages/man/html/cl_khr_icd.html`.

- OpenSSH 9.9p1 now selects its client and server main files from `/etc/ssh`,
  `/run/ssh`, or `/usr/lib/ssh`. It preserves exact `-F` and `-f` paths, user
  client policy, arbitrary `Include` files, and fresh default-path selection
  on SIGHUP. Its first-obtained-value parsers receive standard drop-ins in
  descending filename order after the selector removes shadowed basenames, so
  later filenames retain UAPI priority. Empty files and `/dev/null` links mask
  lower files. The package puts both main defaults under `/usr/lib/ssh`, puts
  moduli under `/usr/share/ssh`, and leaves host keys under `/etc/ssh`. The
  strict two-build checksum is
  `b9b50e17f25c18b3bb3e19f4e4bc4f2ae9e9de278b53daa2bd493b85f3f91558`.
  The test drove `ssh -G`, `sshd -T`, nested Includes, and a live SIGHUP port
  change, then both packaged version commands printed OpenSSH 9.9p1.
- Smartmontools 7.5 sets `smartd`'s default to
  `${sysconfdir}/smartd.conf`, and `-c` selects one alternate file. Its ordered
  device grammar, including `DEVICESCAN` ignoring later lines, makes full-file
  selection safer than automatic drop-in merging. Nex now carries a generic
  patch that searches `/etc/smartd.conf`,
  `/run/smartmontools/smartd.conf`, then
  `/usr/lib/smartmontools/smartd.conf`. It repeats that search on SIGHUP while
  keeping explicit `-c` paths fixed. The package installs its upstream sample
  as the vendor file, not in `/etc`. The runtime regression test exercised all
  three priorities, an empty local mask, the explicit override, and a live
  SIGHUP switch from `/run` to `/etc` inside the built Edgebox root.
- Glibc 2.39 now selects `nsswitch.conf` and `rpc` from `/etc`, `/run`, or
  `/usr/lib`. Its NSS reload cache stores both file metadata and the selected
  tier because Glibc treats every empty or missing file as equivalent content.
  nscd registers all three nsswitch paths for each cache, so creating or
  removing a higher file invalidates cached lookups. The package installs only
  `/usr/lib/nsswitch.conf` and `/usr/lib/rpc`; it discards the generated
  target-root `ld.so.cache`. A long-lived process in a private chroot covered
  no-file compiled defaults, every tier, an empty mask, replacements,
  removals, and RPC selection. Both strict builds matched checksum
  `bf348eabcec257edace3e1e05458bf79ddad1a5164f25e706b7e50d93b25190d`.
- Bash 5.2.21 now carries a generic patch that selects the first existing file
  from `/etc/profile`, `/run/profile`, and `/usr/lib/profile`. Because the file
  is executable shell code, Bash uses one whole file and does not combine
  drop-ins. An empty higher-priority file masks lower files. A private chroot
  smoke exercised all three tiers and the empty mask. It used a small compiled
  `chroot` helper to avoid adding Coreutils and creating a Bash dependency
  cycle. The private root also needed `/lib64 -> usr/lib`, because the packaged
  ELF interpreter uses `/lib64/ld-linux-x86-64.so.2`. The strict two-build
  checksum is
  `51ec6b5cc61775ebee815e5c81ea492105bd4fb632f05162c71894ee3e60c7bb`.
- e2fsprogs 1.47.0 now carries a generic patch that keeps `MKE2FS_CONFIG` as
  the explicit override, then selects `/etc/mke2fs.conf`,
  `/run/mke2fs.conf`, or `/usr/lib/mke2fs.conf`. Its `e2scrub`, `e2scrub_all`,
  and `e2scrub_fail` scripts use the same three tiers for `e2scrub.conf`. The
  package installs both defaults below `/usr/lib`, and its `full` bundle now
  includes the `conf` output. The smoke used 1 GiB sparse images because
  smaller images select the upstream `small` filesystem type and its own inode
  ratio. It proved each tier, empty masks, and valid ext4 output. An empty
  `mke2fs.conf` emits a missing-policy warning, succeeds with compiled defaults,
  and masks lower files. The strict two-build checksum is
  `e58f06a4b3b9438bc06f79a5bc969b58bbfd51a13943d92c351e82a7f4f5475e`.
- BlueZ 5.85 now uses one internal selector in its main, input-manager, HOG,
  and network readers. It checks every colon-separated
  `CONFIGURATION_DIRECTORY` entry in declared order and does not fall back
  outside that explicit list. Without the variable it selects one complete
  file from `/etc/bluetooth`, `/run/bluetooth`, or `/usr/lib/bluetooth`. The
  package puts `main.conf`, `input.conf`, and `network.conf` below `/usr/lib`,
  and an empty higher file masks a lower one. A focused test covered every
  filename, all tiers, the empty mask, and explicit-directory cases in both
  strict builds. The installed daemon printed `5.85`; the two builds matched
  checksum
  `f4b4a8aeac62ad3283a2f61ee7d895964372f09f92c3d72f42f5df2a9e0d7016`.
- PulseAudio 17.0 now keeps `PULSE_CLIENTCONFIG`, `PULSE_CONFIG`,
  `PULSE_SCRIPT`, `PULSE_CONFIG_PATH`, and upstream home paths ahead of system
  policy. It then selects each complete main file from `/etc/pulse`,
  `/run/pulse`, or `/usr/lib/pulse`. Its structured `client.conf.d` and
  `daemon.conf.d` readers merge filenames from user and system trees, let the
  higher tree shadow an equal basename, and parse selected names in lexical
  order. Empty files and links to `/dev/null` mask lower entries. The package
  leaves match tables, restore tables, ALSA data, and explicit startup-script
  includes unchanged; it installs the four main defaults below `/usr/lib` and
  includes them in both public bundles. The production-linked test covered
  every priority and mask in both strict builds, the installed daemon returned
  an asserted `--dump-conf` value, and both builds matched checksum
  `ecd67b81b6a6dfde48df082245e94b837d53624306b65501b8124d2d02506a7d`.
- Shadow 4.15.1 does not need a source patch for `login.defs`. When built with
  libeconf and `--enable-vendordir=DIR`, its `getdef.c` calls
  `econf_readDirs()` for vendor and host files. Nex enables that upstream
  feature and tests vendor and host precedence.
- Linux-PAM 1.7.1 does not need a source patch for the service policy lookup.
  Its Meson `vendordir` option adds a distribution policy directory, and
  `pam_handlers.c` searches the administrator and distribution directories.
  Nex enables the vendor directory and carries a small generic patch for the
  transient `/run/pam.d` tier. PAM modules have their own secondary files, so
  audit those separately before moving every file below `/etc/security`.

Parser patches need tests that match their risk. Smartmontools, Bash, and
e2fsprogs use small whole-file checks. OpenSSH tests both parsers, includes,
and a live SIGHUP. BlueZ tests every reader through one selector. PulseAudio
tests each main file and both structured drop-in families. Glibc tests live
NSS changes and RPC lookup in one private chroot process.

EP011 completed the system proof for the first parser wave. The five
standalone roots, `flat-minimal`, `flat-systemd`, `nex-minimal`, `nex-systemd`,
and the installer, list Glibc's `outputs/conf` under `packages`. An assembly
dependency populates the build root but does not put that output in the final
Nex-structured system. Child assemblies inherit the selected output from
their base.

All 11 affected assemblies built twice with matching checksums, including
`desktop-dev` after strict package builds populated 19 missing or stale store
refs. The Edgebox smoke ran Bash as a login shell, created and inspected an
ext4 filesystem, found the OpenSSH and Glibc vendor files, read the vendor RPC
database through `getent`, and found no replaced package policy under `/etc`.
A desktop checkout contained all three BlueZ files and both Glibc databases
without package-owned copies at the replaced `/etc` paths. Checked
flat-minimal, nex-minimal, and installer roots also contained both Glibc
databases. The direct `nex-systemd` QEMU boot printed `ASSERT-BOOT-PASS`.

A strict build of `2nex-utilities` populated the store ref needed by
`nex-minimal` during the earlier Bash batch, but its generated output update
exposed its legacy `/etc/passwd` and `/etc/group` policy. The final manifest
keeps that unrelated change out of EP011; handle accounts in the separate
assembly-owned account batch.

## Carried patch mechanics

When checking downloaded bytes by hand, use `rtk proxy curl ... --output FILE`
and hash the saved file. Do not pipe a patch or archive through ordinary `rtk`
output into `sha256sum`: RTK may filter the byte stream. A 2026-08-14 Conmon
audit produced a false checksum mismatch this way; the builder's raw curl
download confirmed the manifest's existing SHA-256.

Nex already treats a patch as an ordinary package source. A manifest can name
a repository-local file with `file:` or a downloaded file with `url:` and must
record its SHA-256. The builder verifies those bytes, stages the file, and
exports both positional `SOURCE<n>` and named `SOURCE_<name>` variables. The
package build script chooses where and how to apply it. A failed `patch`
command stops normal `set -e` build scripts, and the strict two-build check
does not infer the patch tool from a local patch source. List the appropriate
`patch` tool bundle explicitly when the build script invokes it. The strict
check proves that the resulting package output is reproducible.

`.agents/MANIFESTS_CODE_STYLE.md` now defines the carried-patch policy. An audit
on 2026-08-14 found 17 package manifests that invoke `patch`. Nex now invokes
all 27 textual patch applications with `--batch --fuzz=0`. Local patches state
their subject, source, upstream status, and rationale. Manifests still download
unchanged patches from durable, immutable LFS, Buildroot, and Arch locations.
Nex now keeps its two personal-gist GCC patches beside the GCC manifest.
Build scripts still use small `sed` substitutions for build paths and versions;
reviewers should turn larger source changes into readable patch files.

Remote patch inputs have integrity but not durable availability. Nex caches a
download by SHA-256 in the host-local `inputs_cache`, but a cold build still
needs the URL. The hash prevents silent replacement; it does not preserve the
file when a moving raw URL or another distribution removes it. An online patch
is still appropriate when its URL identifies immutable bytes on an
authoritative host, the host should retain it for at least as long as the
target source archive, and Nex applies the original patch without changes.
Keep a patch in Git beside its package manifest when Nex authors or modifies
it, its URL follows a moving branch or ephemeral review ref, or its host is
materially less durable than the source archive. Preserve the original URL or
commit in a local patch's header.

`nex check` now requires exactly one primary selector in each source entry,
with `cargo_toml` allowed only beside `cargo_lock`. It requires canonical
lowercase SHA-256 values, opens every `file:` input, and checks its bytes. A
local file must be a tracked regular file inside the Git repository that owns
the manifest. Nex rejects absolute paths, `..` escapes, untracked files, and
symbolic links. `dev:` remains the explicit non-reproducible exception.

`manifest_ref` now names the owning repository's full Git commit, not the
manifest blob alone. When Nex loads a pinned package, it reads the manifest,
local patch files, Cargo manifests and lockfiles, Go sums, and Zig dependency
files from that same commit. Nex stages cached downloads atomically and hashes
remote cache hits again before use. These rules let Git reproduce a pinned
local patch even after the working tree changes and the zub output disappears.

`SystemPackage` now carries `manifest_ref`, and `nex link` writes repository
commits for direct assembly packages as well as package dependencies. It
refuses dirty or untracked manifests, environment files, and local source
inputs, and it rejects inputs outside the repository or reached through a
symbolic link. The link formatter updates the correct YAML section even when
two dependencies use the same commit.

Nex deliberately rejects `manifest_ref` on an assembly manifest itself.
Assembly inheritance and overlays would otherwise require every related file
to come from the same historical tree. A product repository pins its whole
upstream Nex tree with the `upstream/nex/` submodule commit instead. Direct
package entries inside the active product assembly still receive their own
repository commits.

Nex does not need a patch-specific manifest schema to fix these gaps. A patch
can remain an ordinary `url:` or `file:` source, and the build script can apply
patches in an explicit order. The required work belongs in general source
pinning and validation, plus a short maintainer convention for local patch
headers and strict package tests. This keeps the same rules available to any
local source file rather than creating a special patch subsystem.

## Third-party manifest repositories

Nex supports a separate product repository through one conventional layout and
no workspace file. The Git repository that owns the requested top-level
manifest is the product repository. Its `upstream/nex/` Git submodule supplies
the ordinary Nex manifests, and the submodule commit pins the Nex revision.
Nex reads `pkg/` from both repositories. When a command runs inside a product
checkout, it does not also mix in user or system manifest databases from the
host.

`ManifestRepositories::discover` implements this rule. It finds the nearest
`.git` file or directory, so it handles ordinary repositories and real Git
submodule checkouts. Build and check commands discover the two package trees
from the requested manifest. Other commands use the same local repository set
when they run inside the checkout. `--manifest-dir` remains an explicit build
override for unusual cases; do not add a configuration file until a real user
needs a different path or several upstream repositories.

Nex rejects duplicate package identities across roots. Search order does not
let a product manifest shadow an ordinary Nex package. The error names both
files. Product repositories should use their own package namespace for private
agents, proprietary drivers, and patched variants, while ordinary upstream
packages should stay generic and move into Nex when possible.

Nex retains the repository that owns each loaded manifest. Nex manifests
commonly use repository-root paths such as
`pkg/cli/editors/vim-reproducible.patch` and `asm/nex-systemd-overlay.yaml`.
The loader resolves local package sources, inherited assemblies, and overlay
manifest paths from the owning repository root in memory. A `source:` path
inside an overlay remains relative to the overlay file. Nex does not rewrite
YAML paths to host-specific absolute paths.

Pinned `manifest_ref` values use the same repository context. Nex package
snapshots come from the submodule repository, while product package snapshots
come from the outer repository. `ManifestSource::Repository`, graph checks,
and dependency commit resolution carry the owning Git root.

Build orchestration drops manifest write flags when it enters an imported
package, so a strict product build can still check and build upstream packages.
A second guard rejects any imported package that still reaches a build with a
write flag. A Nex-structured assembly copies both manifest trees into
`/nex/db/pkg`; duplicate package identities and destination path collisions
fail before one file can overwrite another.

`src/cli/tests/external_repository_tests.rs` builds a temporary outer Git
repository and nested upstream Git repository without a workspace file. It
checks cross-repository package lookup, an upstream pinned repository commit,
assembly inheritance, per-repository overlays, and duplicate rejection.
Unit tests also cover historical local patches, dirty-source refusal, package
and system-package pins, duplicate commit values in different YAML sections,
invalid refs, and symbolic-link rejection. The full CLI suite has 180 tests.

The 2026-08-14 patch audit rebuilt every touched package twice with strict
checks. Expect required a Tcl rebuild because Tcl carried a stale package
checksum; the refreshed Tcl checksum and outputs passed both builds. Runtime
checks covered Vim, Conmon, Expect, Inetutils, bzip2, and GCC. The GCC check
created and consumed a precompiled header with the packaged compiler. The
bzip2 check compressed and restored a file byte for byte.

Samba's old local patch disabled execution of its Linux credential probe. That
shortcut was not generic-correct. The replacement keeps Samba's full probe for
ordinary root builds. In a user namespace that maps UID 0 but not UID 1, it
detects `EINVAL` from the UID switch and uses Samba's existing syscall-presence
fallback. Samba 4.21.3 passed two strict builds, retained
`HAVE_LINUX_THREAD_CREDENTIALS`, and a compiled program linked against the
packaged `libsmbclient` reported version 4.21.3. Do not solve this by adding
`unshare --map-auto` only around the build: zub currently records root-only
namespace ownership, so extra subordinate owners could enter the store with
incorrect IDs.

The generic embedded-device example is named `edgebox`; its files use the
`edgebox-rootfs` slug. The name describes an appliance-style rootfs without
carrying the Soniq product name into the reusable example.

## UAPI configuration libraries

Nex packages libeconf 0.8.4 as `pkg/libs/system/libeconf.yaml`. Its strict
two-build check passed with package checksum
`451fa06240d6c696adb3ad2dbe520547acf7cd5b7af9332dde0d553765e3585a`, and
the build ran all 71 upstream tests. The `dev` bundle contains both the headers
and shared library, which lets another package detect libeconf at configure
time.

libeconf's current `econf_readConfig` API implements the full UAPI directory
order and accepts `ROOT_PREFIX` for chroot-style tests. Its deprecated
`econf_readDirs` API accepts only vendor and administrator directories in the
actual implementation, despite broader wording in some API comments. Shadow
4.15.1 and 4.19.1 still call `econf_readDirs`, so enabling Shadow's existing
libeconf probe alone does not add `/run/login.defs`. A generic Shadow patch
must switch that call to `econf_readConfig` and preserve `--prefix` through
`ROOT_PREFIX` before Nex can claim full `/etc`, `/run`, `/usr` behavior.

The Shadow manifest now carries that generic patch, configures
`--enable-vendordir=/usr/lib`, and ships the package default as
`/usr/lib/login.defs`. The build uses `useradd --prefix` against four isolated
account databases. It proves the vendor main file, a `/run/login.defs` main
file, an `/etc/login.defs` main file, and an `/etc/login.defs.d` drop-in choose
UID minima 2100, 2200, 2300, and 2400 respectively. Shadow passed the strict
two-build check with package checksum
`90a26bb7253e052daf3e545cb709724afe5395b06d91bc92ff8702e570503a2b`.
Its PAM service files now live below `/usr/lib/pam.d`, and its `runtime` bundle
contains both the programs and vendor policy.

Linux-PAM 1.7.1 always searches `/usr/lib/pam.d` for service policy; its Meson
`vendordir` option instead controls module files below paths such as
`security/`. Nex therefore does not set `-Dvendordir` just to move service
files. The Nex patch inserts `/run/pam.d` between `/etc/pam.d` and
`/usr/lib/pam.d`. The package build links a small PAM client against the newly
built library and proves allow, deny, and allow results from vendor, transient,
and administrator service files. The strict two-build check passed with
package checksum
`76f9607c33b67a06942ce35292350df76062af36d867a4c5f76cad9ad71f0478`.
The files below `/etc/security` remain a separate audit item because each PAM
module has its own lookup and merge behavior.

The first PAM consumer batch installed service files below `/usr/lib/pam.d`.
At that point OpenSSH 9.9p1 enabled PAM but kept its SSH main files below
`/etc/ssh`; those two builds used checksum
`240e0506c3d8b34a88eb7f6d64602386960f9fbc62a959ed983bdd0e44619f1b`.
EP011 later moved the main files below `/usr/lib/ssh`, added the transient
tier, and produced the current checksum
`b9b50e17f25c18b3bb3e19f4e4bc4f2ae9e9de278b53daa2bd493b85f3f91558`.
Swaylock 1.7.2 now enables PAM and passed with checksum
`883fb2383f9c011646571b6651528802b0ce2d6105fa7cc6183ce89e47cf612a`.
Polkit 124 sets Meson's `pam_prefix` to `/usr/lib/pam.d`, exposes a `runtime`
bundle that includes `polkitd` and its policy, and passed with checksum
`43ec084dcc577a49ca9dfcf7ac211f06a9529460668b24bc229171364a786727`.
An assembly does not receive Polkit's daemon or PAM policy merely because it
selects polkit-gnome: dependency flattening follows required shared libraries,
not the provider's whole runtime bundle. Desktop assemblies therefore select
Polkit's runtime bundle explicitly.

Fish 4.7.1 already supports vendor fragments below
`/usr/share/fish/vendor_conf.d`, but it had no `/run` tier. The generic patch
adds `/run/fish/conf.d` between `/etc/fish/conf.d` and the vendor directory
while keeping Fish's per-user directory highest. Its build proves all four
priorities and proves that an empty administrator fragment masks lower files
with the same name. It removes upstream's empty `/etc/fish/config.fish` host
template. The strict two-build checksum is
`cf8110474db59ebf93fe1d4f15da274c982c8ed8693c29ad17d4608d3d3953c8`.

Systemd 257.5 now installs its main configuration files below `/usr/lib` and
its shell, SSH, and X11 fragments in their native vendor directories. The
package keeps `--sysconfdir=/etc` so administrator paths retain their normal
meaning, but it removes upstream's empty host templates. A build-time
`systemd-analyze cat-config` test proves `/etc`, `/run`, `/usr` main-file
priority, same-name drop-in priority, and empty-file masking. The strict
two-build checksum is
`c9ca9f12fa799ce8632962dbb302578e239f1c04c7c1e3471d242af95dedb7ce`.
Meson's custom install step prints a harmless `touch` warning for
`/usr/lib/environment.d/99-environment.conf`; the file exists in the final
output and both strict builds reproduce the checksum.

Foot 1.25.0 and Fuzzel 1.13.1 follow the XDG Base Directory search contract:
they check the user configuration first, then each `XDG_CONFIG_DIRS` entry in
order, and default the system list to `/etc/xdg`. Their upstream `foot.ini`
and `fuzzel.ini` installs contain no active values; they only document built-in
defaults. The manifests therefore move these files to
`/usr/share/doc/<package>/examples` without changing the programs' normal
XDG lookup. Each program's `--check-config` command parses its packaged
example. Keep real product choices in an assembly-owned user or host file; the
desktop-vwl overlay does this for Foot.

SwayNC 0.10.1 requires its JSON configuration, JSON schema, and style sheet.
It now installs those package files below `/usr/share/xdg/swaync` and checks
an explicit path, the user XDG directory, ordered `XDG_CONFIG_DIRS` entries,
`/run/xdg`, and the vendor directory in that order. Its package test adds a
file at each tier and proves every priority change. `ConfigModel` keeps only a
real `--config` argument across reloads; it reruns the layered search when the
caller used defaults. The strict checksum is
`7bff7d16653b7f33fe8774d4fd2a743d78046dd97a68e092792df9d4c7e6e5b1`.

Use `--single` when a package change only requires a consumer image rebuild.
Without it, `nex build <assembly>` follows stale dependency manifests and may
start unrelated package rebuilds. EP012's first desktop-vwl attempt found 43
stale dependencies and entered the phase-zero bootstrap chain; the corrected
commands built each image twice from its declared store refs.

Manifest `extends` paths resolve from the repository root, so `flat-podman`
extends `asm/flat-systemd.yaml`. After EP011 added Glibc's vendor databases,
its strict checksum is
`f8feefae4d82452da5dec9f6fe69e5d65c008a7e6a34273184838f0fe9bd31c9`.
EP011 resolved the earlier missing GN ref by strictly rebuilding GN 0.2289,
then refreshed and exercised every other stale `desktop-dev` ref that the
broad assembly exposed. The final desktop-dev assembly checksum is
`75e4957348ef7c2e7f410ee9a16b52dad18405a315967477018e9492eaae24f5`.

## Ralph loop setup

Nex keeps the Carlos Ralph prompt, plan guide, active and archived ExecPlans,
test rules, manifest rules, and durable knowledge in the tracked `.agents/`
tree. A human starts `carlos` at the repository root and presses Ctrl+R. Carlos
loads `.agents/ralph-prompt.md`, continues turns that lack a terminal marker,
and exits Ralph Mode on `@@AWAITING_HUMAN@@`, `@@BLOCKED@@`, or
`@@COMPLETE@@`.

Ralph works on one plan per run. A completed plan stays active until the human
approves its end. Ralph then moves the unchanged file to
`.agents/execplans/done/` and commits that move. The ignored
`.agents/SCRATCH_KNOWLEDGE.md`, `.agents/local/`, `.agents/waiting/`,
`.agents/bash_history`, and `.agents/execplans/paused/` paths remain local
runtime state.

Nex removed its old tracked `.claude/` tree on 2026-08-14. The useful package
and assembly facts had already moved to `.agents/knowledge/package-manifests.md`
and `.agents/knowledge/system-assemblies.md`. Current `AGENTS.md`,
`RUST_CODE_STYLE.md`, and the `.agents` rules supersede the old commit, coding,
and test advice. In particular, agents must test affected assemblies; the old
Claude note that prohibited those tests was stale.

The unfinished P50 plan remains preserved under
`.agents/execplans/paused/010-p50-personalization-tools.md` and does not block
the active queue. The first active Ralph plan is
`.agents/execplans/011-complete-first-uapi-parser-wave.md`, which covers BlueZ,
PulseAudio, OpenSSH, and Glibc configuration lookup.

Nftables 1.1.1 loads its optional passive OS fingerprint database only when a
rule uses an `osf` expression. Test the real loader with `nft --debug mnl
--check --file RULE` in a private network namespace; parsing the file without
that namespace reaches the host's Netfilter API. The generic reader selects
`/etc/nftables/osf/pf.os`, then `/run/nftables/osf/pf.os`, then
`/usr/lib/nftables/osf/pf.os` as whole files, so an empty selected file masks
lower data. The strict package checksum is
`ed4cd32130e0dc86cc2aedc09d7a455d3646f312770e401f399c0f9153e701c0`.
Edgebox selects the full Nftables bundle, reproduced checksum
`8c29d7b5685704b5bbba57d3243fb6df5ac39413029e1613a8e6f229933e89f5`,
and passes the same installed-reader check from its finished root.

ImageMagick 7.1.2-26 merges every same-name XML file from its search paths.
The package installs its fourteen XML files below `/usr/share/ImageMagick-7`
and adds `/run/ImageMagick-7` before the existing `/etc/ImageMagick-7` tier.
Do not replace this with whole-file selection: authorization policies use the
last match, while resource ceilings cannot be raised by later files. An empty
XML file contributes no rules and is not a mask. Test the real reader with
`magick -list policy`. The strict checksum is
`93c38ea45acc7aefa96ca7635ad18c02813147ea872c98d4db5eb9666fe230fd`.
Desktop-vwl selects the full bundle. Rebuild it before its child assemblies,
and run these large strict assembly builds one at a time on this host because
concurrent builds can exhaust `/tmp` and race on the shared parent ref. A
finished base root contains seventeen public XML files in total, no
`/etc/ImageMagick-7`, and a working installed image conversion path.

P11-kit 0.25.5 reads trust policy from the colon-separated paths configured
at build time and gives the first path highest priority. Nex configures
`/etc/pki/trust:/run/pki/trust:/usr/share/pki/trust`, which lets one normal
trust store merge administrator, transient, and vendor anchors and
blocklists. The installed extraction test covers all three tiers and an
administrator blocklist; the strict package checksum is
`d8524f53a7e4fc14400f74404828eff6059072c1457baab610eac5dc5ba889a6`.

CA Certificates uses p11-kit as the source of truth. The package installs
Mozilla anchors in `/usr/share/pki/trust/anchors`, provides an atomic
`update-ca-certificates` command, pre-generates `/usr/lib/ssl/certs`, and
enables a systemd oneshot that rebuilds `/etc/ssl/certs` from vendor,
transient, and administrator inputs on boot. Its strict checksum is
`b36aa5a04f0b4bf1e6fc67b6bd207c5ed1c5f344135ee0e16700478c53dd825e`.

When a package publishes a shell script, manually list every external command
under that output's `needs`; dependency scanning sees ELF imports but cannot
see shell command names. The CA updater also names p11-kit's module
registration file because `/usr/bin/trust` cannot load trust policy without
it. The updater must also name `/usr/lib/pkcs11/p11-kit-trust.so`, which the
registration file loads dynamically. P11-kit's public `dev` and `runtime`
bundles include its `misc` output for the same reason. A structured assembly
must select p11-kit's runtime bundle explicitly when the root needs a public
`/usr/bin/trust`; flattening that command into another package's capsule does
not create the public link.

Do not use `systemctl --root` alone to judge a unit that Nex stages in factory
`/etc`. The offline command cannot model Nex's live persistent `/etc`. Boot the
system and check the unit plus its generated artifact. EP012's QEMU boot saw
`update-ca-certificates.service` active, a nonempty live CA bundle,
`system-ca=ready`, and `ASSERT-BOOT-PASS`.

P11-kit's PEM-directory extractor creates a mode-`0555` directory. Remove
disposable extracted stores after package assertions and make any retained
output directory owner-writable before Nex starts its second build, or the
runner cannot clean its work root.

File-level runtime closure overlays must unlink a distinct destination before
copying a regular file. Build inputs often use mode `0444` or `0555`, so
opening an existing path in place can fail. It can also follow a destination
symlink and overwrite a file outside the intended path. Nex's materializer
keeps an existing path only when it is the same regular-file inode, otherwise
it removes the path before `fs::copy`. The CLI tests cover read-only files and
symlink targets; the OpenSSL strict build exercises the real overlapping Perl
closure case that exposed this rule.

OpenSSL can keep `--openssldir=/etc/ssl` for the live generated certificate
database without storing package configuration there. The generic OpenSSL
3.3.1 patch selects `openssl.cnf` and `ct_log_list.cnf` as whole files from
`/etc/ssl`, `/run/ssl`, then `/usr/lib/ssl`; exact `OPENSSL_CONF` and
`CTLOG_FILE` values still win. The package installs config copies and helper
scripts below `/usr/lib/ssl`. Its strict two-pass build and copy-mode installed
root smoke covered both readers, empty masks, explicit files, provider module
loading, and retained `OPENSSLDIR: "/etc/ssl"`; the package checksum is
`57252e416e94f89427dadaa413079a456f8015783f0e2bfc317b6a6e91b7bb1d`.

OpenSSL's install target creates empty `certs` and `private` directories below
`OPENSSLDIR`. Remove those known children with `rmdir` before removing their
parents when relocating package files. Do not recursively remove the tree,
because an unexpected package file must fail the build. The configure script
can also continue after it reports a missing `grep`; declare Grep explicitly
when package tests or configure probes need it.

Assembly dependency closure does not include sibling package outputs.
Flat-systemd and nex-systemd already consumed OpenSSL's libraries, but each
base must select `openssl3/outputs/conf` explicitly to publish
`/usr/lib/ssl/openssl.cnf` and `ct_log_list.cnf`. Descendant assemblies inherit
those files after their base rebuilds. EP012 reproduced the two bases and all
six descendants, passed the focused and full Edgebox root tests, and booted
nex-systemd to `system-ca=ready` and `ASSERT-BOOT-PASS`.

OpenSSL 3.3.1 ships a comment-only `ct_log_list.cnf` template with no
`enabled_logs` key, so `CTLOG_STORE_load_default_file` rejects the untouched
vendor file. Test CT path precedence with a valid transient `enabled_logs =`
file, then place invalid and empty administrator files above it to prove
override and mask behavior.
