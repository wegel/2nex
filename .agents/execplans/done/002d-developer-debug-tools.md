# Developer Debug Tools

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Package and assemble the developer and debug tools from the OSTreefy parity
matrix. The end state is that `desktop-dev` exposes the old personal
development commands and GUI debug helpers, or the matrix records a concrete
reason for any remaining gap.

## Progress

- [x] (2026-06-28 12:48Z) Started this sub-EP after moving completed 002b and
  002c subplans to `.agents/execplans/done/`.
- [x] (2026-06-28 12:48Z) Listed knowledge files:
  `agent-workflow.md`, `cli-testing.md`, `kernel-and-boot.md`,
  `ostreefy-parity.md`, `package-manifests.md`, `reproducibility.md`, and
  `system-assemblies.md`.
- [x] (2026-06-28 12:49Z) Read relevant parity, package-manifest, and
  reproducibility knowledge.
- [x] (2026-06-28 12:49Z) Confirmed the 002d matrix rows.
- [x] (2026-06-28 13:08Z) Added and built
  `pkg/cli/system/dmidecode.yaml`; package smoke printed `3.7`.
- [x] (2026-06-28 13:22Z) Added and built
  `pkg/dev/tools/fakeroot.yaml`; package smoke printed `fakeroot version
  1.38.1` and `fakeroot id -u` printed `0`.
- [x] (2026-06-28 12:16Z) Added `dmidecode` and `fakeroot` to
  `asm/desktop-dev.yaml`; the assembly built reproducibly with checksum
  `f0e4075682c44315a395879720f34b0cd29119c88de30edef4754a21447f8f2b`.
- [x] (2026-06-28 12:16Z) Smoked the checked-out
  `systems/desktop-dev/0.0.1` root for `dmidecode`, `fakeroot`, and Python
  pip.
- [x] (2026-06-28 12:16Z) Added and built
  `pkg/libs/system/libxcrypt-compat.yaml`; package smoke loaded
  `libcrypt.so.1` and called `crypt`.
- [x] (2026-06-28 12:16Z) Added `libxcrypt-compat` to
  `asm/desktop-dev.yaml`; the assembly built reproducibly with checksum
  `9447359116ded895625e7baa760492be2d803a0a37a4e0c75883d40b2c90f727`, and
  the system smoke loaded `/usr/lib/libcrypt.so.1`.
- [x] (2026-06-28 12:16Z) Added and built
  `pkg/dev/perl/archive-zip.yaml` and `pkg/dev/perl/archive-cpio.yaml`;
  package smokes imported both modules and ran their helper scripts.
- [x] (2026-06-28 12:16Z) Added and built
  `pkg/dev/tools/strip-nondeterminism.yaml`; package smoke rewrote a ZIP
  member timestamp from `1782649834` to `1704067200`.
- [x] (2026-06-28 12:16Z) Added `strip-nondeterminism`,
  `archive-cpio`, and `archive-zip` to `asm/desktop-dev.yaml`; the assembly
  built reproducibly with checksum
  `51dc353d72ac6d21a8db8928c43a3a94ad74b11f156ce12926e49655b1c91852`, and
  the system smoke rewrote a ZIP member timestamp from `1782649834` to
  `1704067200`.
- [x] (2026-06-28 12:16Z) Fixed Go vendor metadata generation for dependency
  `go.mod` files with inline comments on their `go` directive; unit test
  passed.
- [x] (2026-06-28 12:16Z) Added and built `pkg/dev/tools/gopls.yaml`;
  package smoke printed `golang.org/x/tools/gopls v0.18.1`.
- [x] (2026-06-28 12:37Z) Added `gopls` to `asm/desktop-dev.yaml`; the
  assembly built reproducibly with checksum
  `d82f48fd24b599f61ddad1baa120bc6d86bea855acf3658062db103c2ed530aa`, and
  the checked-out system tree exposed `/usr/bin/gopls` as a Nex package
  symlink.
- [x] (2026-06-28 12:58Z) Added and built `pkg/dev/lang/tk.yaml`; package
  smoke resolved `libtk8.6.so`, verified `wish` files, and confirmed Tk
  reached display initialization in a headless root.
- [x] (2026-06-28 13:05Z) Added `tcl` and `tk` to
  `asm/desktop-dev.yaml`; the assembly built reproducibly with checksum
  `6172ff83da18dee80204cd77e28abcdabc762ad1585eaae37b3cb7679efa4c33`, and
  the checked-out system smoke verified `tclsh`, `wish`, and Tk package
  loading to the expected DISPLAY failure.
- [x] (2026-06-28 13:17Z) Added and built
  `pkg/libs/system/efivar.yaml` as the prerequisite for `efibootmgr`; package
  smoke ran `efivar --help`, `efivar --list-guids`, and `efisecdb --help`.
- [x] (2026-06-28 13:21Z) Added and built
  `pkg/cli/system/efibootmgr.yaml`; package smoke printed `version 18`.
- [x] (2026-06-28 13:32Z) Added `efibootmgr` to
  `asm/desktop-dev.yaml`; the assembly built reproducibly with checksum
  `e0747e151da5ecb85860db843d9c54353d9298cfded23d4248cb28aa4f185b25`, and
  the checked-out system smoke ran `efibootmgr --version` and
  `efibootdump --help`.
- [x] (2026-06-28 13:58Z) Added and built
  `pkg/cli/net/aws-cli.yaml`; package smoke printed
  `aws-cli/1.45.36 Python/3.12.2 ... botocore/1.43.36`.
- [x] (2026-06-28 14:42Z) Added `aws-cli` to
  `asm/desktop-dev.yaml`; the assembly built reproducibly with checksum
  `ea76879618a9fc0536e21de104436c8a6e793f716271359ed77aee0c67d8072b`, and
  the checked-out system smoke ran `aws --version`.
- [x] (2026-06-28 13:41Z) Added and built
  `pkg/dev/tools/gdb.yaml`; package smoke printed `GNU gdb (GDB) 17.2` and
  `GNU gdbserver (GDB) 17.2`.
- [x] (2026-06-28 13:41Z) Added `gdb` to `asm/desktop-dev.yaml`; the
  assembly built reproducibly with checksum
  `4f4cc4b87a336e8919e20a035ad9ca2246983c209e54b6095e2e1fb1140ff2be`, and
  the checked-out system smoke ran `gdb --version` and `gdbserver --version`.
- [x] (2026-06-28 14:20Z) Added and built
  `pkg/dev/python/pycairo.yaml` as a Meld prerequisite; package smoke imported
  `cairo`, printed PyCairo `1.29.0` and Cairo `1.18.4`, and created an image
  surface.
- [x] (2026-06-28 14:38Z) Added and built
  `pkg/libs/text/gtksourceview4.yaml` as a Meld prerequisite; package smoke
  loaded `libgtksourceview-4.so.0` and `pkg-config --modversion
  gtksourceview-4` printed `4.8.4`.
- [x] (2026-06-28 14:54Z) Added and built
  `pkg/dev/python/pygobject.yaml` as a Meld prerequisite; package smoke
  imported `gi`, `gi._gi`, and `gi._gi_cairo`, and `pkg-config --modversion
  pygobject-3.0` printed `3.56.3`.
- [x] (2026-06-28 14:59Z) Added and built
  `pkg/dev/tools/gobject-introspection.yaml` as the scanner and base typelib
  prerequisite for GTK Python programs; package smoke ran
  `g-ir-scanner --version`, `g-ir-compiler --version`, resolved
  `GIRepository` through `GObject` and `GLib`, and imported `giscanner`.
- [x] (2026-06-28 15:31Z) Enabled ATK introspection in
  `pkg/libs/graphics/atk.yaml`; package smoke used `g-ir-inspect` to load
  `Atk-1.0.typelib`, report `GObject-2.0` and `GLib-2.0`, and report
  `libatk-1.0.so.0`.
- [x] (2026-06-28 15:44Z) Enabled GDK Pixbuf introspection in
  `pkg/libs/graphics/gdk-pixbuf.yaml`; package smoke used `g-ir-inspect` to
  load `GdkPixbuf-2.0.typelib` and `GdkPixdata-2.0.typelib`, report their
  dependent typelibs, and report `libgdk_pixbuf-2.0.so.0`.
- [x] (2026-06-28 16:19Z) Enabled HarfBuzz introspection in
  `pkg/libs/text/harfbuzz.yaml` while preserving Cairo integration; package
  smoke used `g-ir-inspect` to load `HarfBuzz-0.0.typelib` and ran
  `hb-view --version`.
- [x] (2026-06-28 16:31Z) Enabled Pango introspection in
  `pkg/libs/text/pango.yaml` while preserving Cairo support; package smoke
  used `g-ir-inspect` to load five Pango typelibs and ran
  `pango-view --version`.
- [x] (2026-06-28 17:06Z) Enabled GTK3 introspection in
  `pkg/libs/graphics/gtk3.yaml`; package smoke used `g-ir-inspect` to load
  `Gdk`, `Gtk`, and `GdkX11` typelibs and report their shared libraries.
- [x] (2026-06-28 17:16Z) Enabled GtkSourceView 4 introspection in
  `pkg/libs/text/gtksourceview4.yaml`; package smoke used `g-ir-inspect` to
  load `GtkSource-4.typelib` and report `libgtksourceview-4.so.0`.
- [x] (2026-06-28 15:51Z) Added and built `pkg/apps/misc/meld.yaml`;
  direct Python smoke imported GTK, GtkSource, and Meld, and a merged runtime
  smoke ran `meld --version` and printed `3.22.2`.
- [x] (2026-06-28 17:43Z) Fixed Meld capsule runtime metadata after the first
  `desktop-dev` smoke found missing Python GI modules and then a missing
  `Gdk-3.0.typelib`; the strict package build passed, direct GI imports from
  the build root printed GTK `3.24.43` and Meld `3.22.2`, and the launcher
  printed `3.22.2`.
- [x] (2026-06-28 17:57Z) Added `HarfBuzz-0.0.typelib` to Meld after the
  checked-out `desktop-dev` GI import reached Pango and failed on HarfBuzz;
  the strict package build passed, direct GI imports from the build root
  printed GTK `3.24.43` and Meld `3.22.2`, and the launcher printed `3.22.2`.
- [x] (2026-06-28 18:10Z) Added `freetype2-2.0.typelib` to Meld after the
  checked-out `desktop-dev` GI import reached Pango/HarfBuzz and failed on
  Freetype; the strict package build passed with raw checksum
  `c896a665380f671a98eaba14b7022eaf130b26245ca6bacd091694565567671f`, direct
  GI imports from the build root printed GTK `3.24.43` and Meld `3.22.2`, and
  the launcher printed `3.22.2`.
- [x] (2026-06-28 18:19Z) Added Meld to `asm/desktop-dev.yaml`; the assembly
  built reproducibly with checksum
  `2c88129d1c41d7a68057b94579b0c4fde39d6859a38e7fb64acc98bc8ea96bed`, and a
  checked-out system smoke ran `meld --version` plus the stronger GTK and
  GtkSource import from the Meld package capsule.
- [x] (2026-06-28 16:43Z) Added and built
  `pkg/dev/tools/lldb.yaml`; the strict package build passed reproducibility
  with raw checksum
  `a693863e2a975122271f45d66dd3b20066d685cffe05534e14ca619a868ce33b`, `nex
  check` passed, and package smokes from the build root printed LLDB
  `21.1.8` for `lldb`, `lldb-server`, and `lldb-dap`.
- [x] (2026-06-28 16:47Z) Added LLDB to `asm/desktop-dev.yaml`; the assembly
  built reproducibly with checksum
  `5fb3603c1ef27ad4629b4ac1207fff336010e41464fd084f36b652468ac8ccd3`, and a
  checked-out system smoke ran `lldb --version`, `lldb-server version`, and
  `lldb-dap --version`.
- [x] (2026-06-28 18:52Z) Added and built
  `pkg/libs/graphics/qt6-5compat.yaml`; the strict package build passed
  reproducibility with checksum
  `27a59ba7cfd646d0e998a74d20e5d413071dd1bf99b7d17a644691f9deb584e7`,
  `nex check` passed, `nex resolve qt6-5compat` returned the needed runtime
  closure, and loader plus `QTextCodec` compile smokes passed.
- [x] (2026-06-28 19:19Z) Added and built
  `pkg/apps/misc/sqlitebrowser.yaml`; the strict package build passed
  reproducibility with checksum
  `074825e1a8785c9887d67c06dc878017ac15a102c3430d46fe23eb1ef78f7697`,
  `nex check` passed, `nex resolve sqlitebrowser` returned a 27-ref runtime
  closure, and a root built with `zub union-checkout` ran
  `sqlitebrowser --version` with Qt's offscreen platform.
- [x] (2026-06-28 17:36Z) Wrapped SQLite Browser so the package capsule
  carries its Qt platform plugins and sets `QT_PLUGIN_PATH` before starting
  the real binary. The strict package build passed reproducibility with
  checksum `c8027c77a2da478d8bc007394c529f8d8b34d0cbdbecfca40b95e5b2fa5f1d47`,
  `nex check` passed, `nex resolve sqlitebrowser` returned a 30-ref runtime
  closure, and a `zub union-checkout` smoke ran the real ELF through the
  loader and ran `sqlitebrowser --version` with `QT_QPA_PLATFORM=offscreen`.
- [x] (2026-06-28 17:37Z) Added SQLite Browser to
  `asm/desktop-dev.yaml`; the assembly built reproducibly with checksum
  `fbef97aa61b8d8afab34f1a4283d4dcb1da03aac32faf6d7c57df4a552c3fce2`, and a
  checked-out system smoke showed `/usr/bin/sqlitebrowser` points into the
  package capsule and ran `sqlitebrowser --version` with both
  `QT_QPA_PLATFORM=offscreen` and `QT_QPA_PLATFORM=wayland`.

## Surprises & Discoveries

- Observation: The matrix assigns more rows to 002d than the main EP examples.
  Evidence: `.agents/ostreefy-parity-matrix.md` assigns `aws-cli`,
  `dmidecode`, `efibootmgr`, `fakeroot`, `gdb`, `gopls`,
  `libxcrypt-compat`, `lldb`, `meld`, `python-pip`, `sqlitebrowser`,
  `strip-nondeterminism`, and `tk` to 002d.
- Observation: Standalone LLDB 21.1.8 needs `Python3_EXECUTABLE` even when
  Python scripting is disabled.
  Evidence: Without `-DPython3_EXECUTABLE=/usr/bin/python3`, CMake generated
  empty Ninja commands for `SBLanguages.h` and `liblldb.exports`. GNU ld later
  rejected the source-tree export pattern file when copied by hand. Passing
  `Python3_EXECUTABLE` made CMake generate the proper files, and the strict
  package build passed reproducibility.
- Observation: The current LLDB configuration generates only `bin`, `dev`,
  and `lib` output buckets.
  Evidence: The first successful install failed during bundle processing while
  `full` referenced absent `outputs/misc` and `outputs/static`; after `full`
  used only `bin`, `dev`, and `lib`, the strict package build completed.
- Observation: `dmidecode` needs package-local build fixes despite its simple
  Makefile.
  Evidence: the first strict build failed because the Makefile used `cc` and
  the build root only exposed `gcc`. Later attempts needed Linux headers for
  glibc includes and `findutils` for the output normalization commands.
- Observation: `fakeroot` needs manual script runtime metadata.
  Evidence: the wrapper uses `/usr/bin/sh`, `sed`, `getopt`, `faked`, and
  `libfakeroot.so`. The manifest rewrites the upstream `/bin/sh` shebang to
  `/usr/bin/sh` and records those manual `needs`; the package smoke passed
  after those runtime refs were checked out.
- Observation: Qt platform plugins need package-local handling for
  `nex_structure` app capsules.
  Evidence: the initial SQLite Browser package smoke passed in a flat
  `zub union-checkout` root, but the checked-out `desktop-dev` system failed
  with `qt.qpa.plugin: Could not find the Qt platform plugin "offscreen" in
  ""`. Moving the real binary to `/usr/libexec/sqlitebrowser/sqlitebrowser`,
  installing a `/usr/bin/sqlitebrowser` wrapper, copying `libqminimal.so` and
  `libqoffscreen.so` into the package, and setting `QT_PLUGIN_PATH` made the
  package smoke run `sqlitebrowser --version`.
- Observation: The current `qt6-base` package does not provide the common
  desktop Qt platform plugins.
  Evidence: SQLite Browser could copy `libqminimal.so` and `libqoffscreen.so`
  from `qt6-base`, but no `libqwayland-generic.so` or xcb platform plugin was
  available. The wrapper falls back from `QT_QPA_PLATFORM=wayland` to
  `minimal` when no Wayland plugin exists; real Wayland support needs a
  future Qt Wayland or xcb platform plugin package.
- Observation: `desktop-dev` already satisfies the `python-pip` parity row via
  the existing Python dev bundle.
  Evidence: a checked-out `systems/desktop-dev/0.0.1` root ran
  `/usr/bin/python3 -m pip --version` and printed pip 24.0 from the packaged
  Python 3.12 site-packages directory.
- Observation: The compat libxcrypt build should stay separate from the main
  libxcrypt package.
  Evidence: the main `pkg/libs/system/libxcrypt.yaml` configures
  `--enable-obsolete-api=no` and outputs `libcrypt.so.2`; the compat manifest
  configures `--enable-obsolete-api=glibc`, deletes headers and linker-name
  files that would collide, and keeps only `libcrypt.so.1`.
- Observation: `strip-nondeterminism` 1.15.1 is available in the Debian pool
  even though search snippets still mentioned 1.15.0.
  Evidence: the Debian pool index listed 1.15.1 source and binary files; the
  downloaded `strip-nondeterminism_1.15.1.orig.tar.bz2` sha256 was
  `b8046b0faf182aff8de68abf8318d2b913637f4c23961cf61c47402af132a237`.
- Observation: `strip-nondeterminism` needs `/usr/bin/file` for real work.
  Evidence: `strip-nondeterminism --version` worked without `file`, but a ZIP
  normalization smoke printed `Can't exec "file"` and left the member
  timestamp unchanged. After the manifest added `file`, the smoke changed the
  member timestamp from `1782649834` to `1704067200`.
- Observation: `desktop-dev` must include the Perl archive modules explicitly
  for `strip-nondeterminism` to work as a global command.
  Evidence: The first system smoke found `strip-nondeterminism --version`, but
  Perl could not locate `Archive/Zip.pm`. After the assembly added
  `archive-cpio` and `archive-zip`, the system smoke normalized the ZIP member
  timestamp.
- Observation: Python GI smokes can expose transitive typelibs one namespace at
  a time.
  Evidence: Meld `--version` worked, but a checked-out `desktop-dev` Python
  smoke that imported `Gtk` and `GtkSource` from the Meld package capsule first
  failed on `Gdk-3.0.typelib`, then on `HarfBuzz-0.0.typelib`, then on
  `freetype2-2.0.typelib`.
- Observation: `gopls v0.18.1` is the newest stable tag compatible with Nex's
  current Go toolchain.
  Evidence: `gopls v0.19.0` through `v0.22.0` require Go versions newer than
  1.23.8; `v0.18.1` declares `go 1.23.4`.
- Observation: Nex can package AWS CLI v1 as one source-only Python package,
  but the manifest must pin compatible setuptools-era dependencies.
  Evidence: `colorama` 0.4.6 needed the unpackaged `hatchling` build backend,
  so the manifest uses compatible `colorama` 0.4.4 and `urllib3` 1.26.20.
  The build copies hash-named Nex inputs to versioned `.tar.gz` filenames
  before invoking pip, rewrites `/usr/sbin/python3` shebangs to
  `/usr/bin/python3`, and adds manual `/usr/bin/python3` needs for generated
  Python entry points.
- Observation: Python console scripts inside package capsules may not see their
  package-local modules without a path shim.
  Evidence: The first `desktop-dev` AWS CLI smoke failed with
  `ModuleNotFoundError: No module named 'awscli'` even though the AWS package
  contained `awscli` under its own `site-packages`. Python searched the
  packaged Python interpreter's `site-packages`. The manifest now injects the
  AWS package's own `usr/lib/python3.12/site-packages` path into generated
  Python entry points based on `realpath(__file__)`.
- Observation: A helper command used only after install is still a build-time
  dependency.
  Evidence: An AWS CLI rebuild used `grep` only to detect shebangs after
  install. Because the manifest did not list `grep`, the build script printed
  `grep: command not found` and silently skipped the intended script patch.
  Replacing the grep test with shell string comparison removed that undeclared
  build input.
- Observation: The Go vendoring helper previously wrote invalid
  `vendor/modules.txt` metadata for modules whose `go.mod` `go` directive has
  an inline comment.
  Evidence: `gopls` build failed with `compile: invalid value "go" for -lang`
  until `src/cli/src/go_vendor/mod.rs` stripped `//` comments from the version.
- Observation: Checked-out `nex_structure` systems expose assembled binaries
  through absolute `/nex/pkg` symlinks.
  Evidence: After adding `gopls`, `systems/desktop-dev/0.0.1` contained
  `/usr/bin/gopls -> /nex/pkg/dev/tools/gopls/0.18.1/7ab2603e/usr/bin/gopls`.
  A host checkout cannot execute that symlink directly unless host `/nex`
  matches the checked-out root, so the smoke checked the symlink and ran the
  target binary under the checked-out tree.
- Observation: Tk can be smoked in a headless root without `Xvfb`.
  Evidence: The host had no `Xvfb` or `xvfb-run`. The smoke used the dynamic
  loader to resolve `/usr/lib/libtk8.6.so`, sourced Tk's `pkgIndex.tcl` with
  `dir=/usr/lib/tk8.6`, verified the registered loader and `wish` files, then
  treated the expected DISPLAY failure from `package require Tk 8.6.14` as
  proof that Tk loaded far enough to initialize the display path.
- Observation: Build-time dependencies are explicit and manual, while many
  runtime library dependencies are generated.
  Evidence: `PHILOSOPHY.md` and `.agents/knowledge/package-manifests.md` now
  state that compilers, headers, configure helpers, interpreters, and build
  tools must appear in package `dependencies`; `--compute-deps` inspects
  installed ELF files after build time and writes runtime library metadata.
- Observation: `efivar` needs upstream-specific build and smoke choices.
  Evidence: The manifest disables docs with `ENABLE_DOCS=0`, installs to
  `/usr/lib` with `LIBDIR=/usr/lib`, and passes `HOST_MARCH=` so upstream's
  host generator avoids `-march=native`. Upstream `efivar` has no `--version`,
  so the smoke uses `--help` and `--list-guids`.
- Observation: `efibootmgr` needs a concrete EFI directory at build time.
  Evidence: Upstream fails unless `EFIDIR` is set. Nex currently installs its
  bootloader at `EFI/BOOT/BOOTX64.EFI`, so the manifest uses `EFIDIR=BOOT`.
- Observation: GDB configure helpers must be explicit build inputs.
  Evidence: Early strict builds needed `file`, `m4`, `pkgconf`, and
  `diffutils`. Without prefixes, `--with-gmp` and `--with-mpfr` were treated
  as the prefix `yes` and link failed with `cd: yes/lib: No such file or
  directory`. The final manifest passes `--with-gmp=/usr` and
  `--with-mpfr=/usr`.
- Observation: PyCairo can build as a direct Meson package.
  Evidence: PyPI source `pycairo-1.29.0.tar.gz` declares the Meson backend, but
  the Nex manifest drives `meson setup` directly with `-Dwheel=false` and
  avoids a separate `meson-python` package. The strict build passed twice with
  checksum `2ad975969b4ba3059eb9906e1777fa1ca39faba83522b8a10609b7ed90a17890`.
- Observation: Cairo's pkg-config file forces explicit build inputs for its
  private dependencies.
  Evidence: The first PyCairo build failed while resolving `dependency('cairo')`
  because `libpng`, `fontconfig`, `freetype2`, X11, XCB, and `pixman-1` `.pc`
  files were missing from the build root.
- Observation: GTK3 consumers that use pkg-config need the `.pc` providers
  behind GTK3's own private dependency graph.
  Evidence: GtkSourceView 4 needed libjpeg, libtiff, Wayland, xkbcommon, EGL,
  DBus, and Xtst providers before Meson could resolve `gtk+-3.0`.
- Observation: GtkSourceView 4 needs `/usr/local/include` to exist in the
  isolated build root.
  Evidence: The compile failed under `-Werror=missing-include-dirs` after
  `libtiff-4.pc` caused pkg-config to emit `-I/usr/local/include`. The
  manifest follows the existing AppStream workaround and creates that directory
  before Meson setup.
- Observation: PyGObject 3.56 uses GLib's current girepository API.
  Evidence: `meson.build` requires `girepository-2.0 >= 2.80.0`, and the
  existing GLib 2.84 package provides `girepository-2.0.pc` and
  `libgirepository-2.0.so`.
- Observation: PyGObject needs its bundled `pythoncapi-compat` subproject.
  Evidence: Meson entered the release tarball's `subprojects/pythoncapi-compat`
  directory during both strict build passes, so this package should not use a
  blanket Meson fallback ban unless that subproject remains allowed.
- Observation: PyGObject alone does not prove Meld can import GTK.
  Evidence: GLib, GTK3, and the new GtkSourceView 4 package currently disable
  GIR or introspection output. The PyGObject smoke imports its own extension
  modules, but `gi.repository.Gtk` and `gi.repository.GtkSource` still need
  the corresponding typelibs.
- Observation: The `./nex` wrapper depends on the local CLI vendor tree.
  Evidence: `./nex build pkg/dev/python/pygobject.yaml ...` failed before
  package work because Cargo could not read `src/cli/vendor`; the compiled
  CLI binary ran the same build command successfully.
- Observation: GNOME introspection consumers need the older
  `g-ir-scanner` toolchain, not only GLib 2.84's `gi-*` tools.
  Evidence: Enabling GLib introspection failed during Meson configure because
  `g-ir-scanner` was missing. The existing GLib package already shipped
  `gi-compile-repository`, `gi-decompile-typelib`, `gi-inspect-typelib`, and
  `libgirepository-2.0.so`, but it shipped no `*.typelib` files and no
  `g-ir-scanner`.
- Observation: GObject Introspection needs explicit parser-generator helpers
  and manual script runtime fixes.
  Evidence: Strict builds first failed for missing `flex`, then Flex and
  Bison failed without `m4`. The installed Python tools used
  `#!/usr/bin/env /usr/sbin/python3`; the package smoke failed until the
  manifest rewrote them to `/usr/bin/python3` and recorded manual
  `/usr/bin/python3` needs.
- Observation: The scanner package must force and install base GLib typelibs.
  Evidence: The default build installed `GIRepository-2.0.typelib`, but
  `g-ir-inspect --print-typelibs GIRepository` failed because
  `GObject-2.0.typelib` was missing. The manifest now forces and installs
  GLib, GObject, GModule, and Gio GIR and typelib files.
- Observation: `g-ir-scanner` needs a local `distutils` package with Python
  3.12.
  Evidence: The first ATK introspection build reached `/usr/bin/g-ir-scanner`
  and failed with `ModuleNotFoundError: No module named 'distutils'`. The
  scanner package now copies setuptools' vendored `distutils` tree into its
  own output, and a runtime smoke without the setuptools package imported
  `distutils.cygwinccompiler`.
- Observation: The scanner package needs GLib's transitive runtime libraries
  named directly for consumer builds.
  Evidence: A runtime smoke that used only `nex resolve gobject-introspection`
  output failed first on `libpcre2-8.so.0`, then on `libz.so.1`. The scanner
  manifest now maps `pcre2`, `zlib`, and `util-linux`, matching the runtime
  closure that `nex resolve glib2` reports.
- Observation: ATK introspection needs `util-linux` as a build input.
  Evidence: ATK produced `Atk-1.0.gir` and `Atk-1.0.typelib` before
  `util-linux` was explicit, but `g-ir-scanner` warned that `mount.pc` was
  missing while processing `gio-2.0`. Adding the `util-linux` dev bundle
  removed that warning while preserving the same package checksum.
- Observation: GTK-family Python packages need lower GTK typelibs built before
  their own runtime smokes can load.
  Evidence: Meld failed with `Meld requires Gtk+ 3.20 or higher` until the
  plan started enabling typelibs bottom-up. ATK now installs
  `Atk-1.0.typelib`, and GDK Pixbuf now installs `GdkPixbuf-2.0.typelib` and
  `GdkPixdata-2.0.typelib`.
- Observation: HarfBuzz introspection must preserve its existing Cairo feature
  set.
  Evidence: A first HarfBuzz introspection build completed reproducibly but
  Meson reported `Cairo integration: NO`, removed `hb-view`, and removed
  `libharfbuzz-cairo.so`. Adding Cairo's X11 pkg-config providers and passing
  `-Dcairo=enabled` made configure report `Cairo integration: YES` and kept
  the existing Cairo outputs while adding `HarfBuzz-0.0.typelib`.
- Observation: Pango needs a separate HarfBuzz GIR input even when it already
  depends on HarfBuzz's dev bundle.
  Evidence: Pango's GIR target failed with `Couldn't find include
  'HarfBuzz-0.0.gir'` until the manifest added
  `x86_64/pkg/libs/text/harfbuzz/10.1.0/outputs/misc`; HarfBuzz's `dev`
  bundle provides libraries and typelibs, but the generated `.gir` file lives
  in `outputs/misc`.
- Observation: GTK3's existing build script was using undeclared `sed`.
  Evidence: The first GTK3 introspection build failed with
  `/nex/tmp/build_script.sh: line 10: sed: command not found` before Meson
  configure. Adding `x86_64/pkg/cli/text/sed/4.9/bundles/dev` fixed that
  script input.
- Observation: GTK3 cannot update the checked-out shared MIME database in
  place.
  Evidence: `update-mime-database /usr/share/mime` failed with `Failed to
  write XML file` because dependency checkouts are not writable build outputs.
  The manifest now copies `/usr/share/mime` into `${WORK_DIR}/mime-root`, runs
  `update-mime-database` on that copy, and sets `XDG_DATA_DIRS` for build
  tools.
- Observation: GTK3 introspection needs the lower GTK-family `.gir` files, not
  just their libraries.
  Evidence: The passing GTK3 build depends on the `outputs/misc` refs for
  ATK, GDK Pixbuf, HarfBuzz, and Pango; its generated outputs include
  `Gdk-3.0.typelib`, `GdkX11-3.0.typelib`, `Gtk-3.0.typelib`, and their
  matching `.gir` files.
- Observation: GtkSourceView 4 follows the same GIR dependency pattern as
  GTK3.
  Evidence: Enabling `-Dgir=true` passed only after the manifest declared
  `gobject-introspection` plus the `outputs/misc` refs for GTK3, ATK, GDK
  Pixbuf, HarfBuzz, and Pango. The generated outputs include
  `GtkSource-4.typelib` and `GtkSource-4.gir`.
- Observation: PyGObject script packages need manual typelib runtime edges.
  Evidence: `nex resolve meld` reported only Meld, Python, GTK3,
  GtkSourceView4, `gobject-introspection`, PyCairo, and PyGObject. The hand
  smoke still had to make GTK and GtkSource typelibs available, and GTK's
  typelib load also needed `xlib-2.0.typelib` from `gobject-introspection`.
- Observation: A version command for a GTK GUI may need to run before display
  setup.
  Evidence: The first merged Meld smoke imported GTK and GtkSource, but
  `/usr/bin/meld --version` reached `Gtk.Settings.get_default()` in a
  headless root and failed because GTK returned `None`. The manifest now
  patches `main()` so `--version` prints `meld.conf.__version__` immediately
  after logging setup.
- Observation: A build-output root can keep package outputs split by output
  name.
  Evidence: The Meld build root placed the script at
  `/nex/out/bin/usr/bin/meld` and modules under
  `/nex/out/lib/usr/lib/python3.12/site-packages`. The package-local path
  shim expects a merged `/usr/bin` and `/usr/lib`, so the final launcher smoke
  overlaid the current Meld outputs into a merged smoke root before running
  `/usr/bin/meld --version`.
- Observation: `meld --version` is not enough proof for a Python GI GUI app.
  Evidence: A checked-out `desktop-dev` root ran `/usr/bin/meld --version`,
  but a direct import from the resolved Meld capsule first failed because
  `gi` was missing, then failed because `Gdk-3.0.typelib` was missing. The
  manifest now vendors PyGObject and PyCairo modules into Meld's output and
  records the GTK-family typelibs that `Gtk` and `GtkSource` load.
- Observation: GTK imports can reveal lower typelibs one at a time.
  Evidence: After Meld carried `Gdk-3.0.typelib`, the next checked-out
  `desktop-dev` smoke failed with `Typelib file for namespace 'HarfBuzz',
  version '0.0' not found`; Pango's typelib metadata pulls HarfBuzz while
  loading GTK.

## Decision Log

- Decision: Treat the matrix as the source of truth for this sub-EP's target
  list.
  Rationale: Sub-EP 002b froze the matrix from the live OSTreefy source files,
  and the main EP says later sub-EPs should handle rows assigned by the matrix.
  Date/Author: 2026-06-28 / Ralph

## Outcomes & Retrospective

The 002d matrix rows are covered or explicitly deferred. `desktop-dev` now
exposes the developer/debug tools assigned to this sub-EP: `aws`,
`dmidecode`, `efibootmgr`, `fakeroot`, `gdb`, `gdbserver`, `gopls`, legacy
`libcrypt.so.1`, `lldb`, `lldb-server`, `lldb-dap`, `meld`,
`python3 -m pip`, `sqlitebrowser`, `strip-nondeterminism`, Tcl, and Tk.
Nushell remains deferred because the human said to skip it after strict
reproducibility attempts failed and they do not use it. The parity matrix row
for SQLite Browser now records the checked-out `desktop-dev` smoke.

`dmidecode` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/system/dmidecode.yaml
./src/cli/target/debug/nex build pkg/cli/system/dmidecode.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d .nex/tmp/dmidecode-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/system/dmidecode/3.7/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
ln -s usr/lib "$tmpdir/lib64"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/dmidecode --version
```

The strict build passed reproducibility with checksum
`235268d44fbcd8ff8f2a60ef2ce620efb7ba9e8c7c653715bdcfe56db14ecc54`, and the
smoke printed `3.7`.

`fakeroot` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/tools/fakeroot.yaml
./src/cli/target/debug/nex build pkg/dev/tools/fakeroot.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d .nex/tmp/fakeroot-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/dev/tools/fakeroot/1.38.1/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/cli/shells/bash/5.2.21/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/ncurses/6.4-20230520/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/userland/coreutils/9.5/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/cli/text/sed/4.9/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/userland/util_linux/2.40/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/acl/2.3.2/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/attr/2.5.2/outputs/lib "$tmpdir"
ln -s usr/lib "$tmpdir/lib64"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/fakeroot --version
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/fakeroot /usr/bin/id -u
```

The strict build passed reproducibility with checksum
`7baaa9f1437b5c3819d97eb4470a248c27f56233a5603317983f2df3a06e6fe2`. The
smoke printed `fakeroot version 1.38.1` and `0`.

`pycairo` is packaged as a prerequisite for Meld. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/python/pycairo.yaml
./src/cli/target/debug/nex build pkg/dev/python/pycairo.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

The strict build passed reproducibility with checksum
`2ad975969b4ba3059eb9906e1777fa1ca39faba83522b8a10609b7ed90a17890`. A
checked-out package smoke ran `/usr/bin/python3 -c 'import cairo; ...'` inside
`unshare --root` and printed:

```text
1.29.0
1.18.4
1
```

`gtksourceview4` is packaged as a prerequisite for Meld. Package proof:

```bash
./src/cli/target/debug/nex check pkg/libs/text/gtksourceview4.yaml
./src/cli/target/debug/nex build pkg/libs/text/gtksourceview4.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

The strict build passed reproducibility with checksum
`509a79f89211afeeaa08618aab893ddf29adfb09e0fd5040665598256f97162b`. A
checked-out smoke root loaded `/usr/lib/libgtksourceview-4.so.0` with the
dynamic loader and ran:

```bash
/usr/bin/pkg-config --modversion gtksourceview-4
```

`gtk3` now ships GTK typelibs for Python GI consumers such as Meld. Package
proof:

```bash
./src/cli/target/debug/nex format pkg/libs/graphics/gtk3.yaml
./src/cli/target/debug/nex check pkg/libs/graphics/gtk3.yaml
./src/cli/target/debug/nex build pkg/libs/graphics/gtk3.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

The strict build passed reproducibility with checksum
`bd7137aedad5794cd7f3fdd6524b96b04f8ec11fa8fa19832f5456ead2c4c25d`. A
checked-out smoke root ran:

```bash
for ns in Gdk Gtk GdkX11; do
  unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/g-ir-inspect --version=3.0 --print-typelibs "$ns"
  unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/g-ir-inspect --version=3.0 --print-shlibs "$ns"
done
```

The smoke reported `libgdk-3.so.0` for `Gdk` and `GdkX11`, `libgtk-3.so.0`
for `Gtk`, and dependent typelibs including `GdkPixbuf-2.0`, `Pango-1.0`,
`HarfBuzz-0.0`, `Atk-1.0`, `GObject-2.0`, and `Gio-2.0`.

`gtksourceview4` now ships the `GtkSource-4` typelib for Python GI consumers
such as Meld. Package proof:

```bash
./src/cli/target/debug/nex format pkg/libs/text/gtksourceview4.yaml
./src/cli/target/debug/nex check pkg/libs/text/gtksourceview4.yaml
./src/cli/target/debug/nex build pkg/libs/text/gtksourceview4.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

The strict build passed reproducibility with checksum
`0a6dad55ccf8c7cb02c235e130941004330c57e22662233a387d836f20f73a8d`. A
checked-out smoke root ran:

```bash
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/g-ir-inspect --version=4 --print-typelibs GtkSource
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/g-ir-inspect --version=4 --print-shlibs GtkSource
```

The smoke reported `libgtksourceview-4.so.0` and dependent typelibs including
`Gtk-3.0`, `Gdk-3.0`, `Pango-1.0`, `Atk-1.0`, `GObject-2.0`, and `Gio-2.0`.

The command printed `4.8.4`.

`pygobject` is packaged as a prerequisite for Meld. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/python/pygobject.yaml
./src/cli/target/debug/nex build pkg/dev/python/pygobject.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

The strict build passed reproducibility with checksum
`1ac0d5e6e101e685f50e64152682fe772ae885ef4220a69bec40177a21c6f095`. A
checked-out smoke root ran:

```bash
/usr/bin/python3 -c 'import gi; import gi._gi; import gi._gi_cairo; print(gi.__version__); print(gi._gi.pygobject_version); print("pygobject-ok")'
/usr/bin/pkg-config --modversion pygobject-3.0
```

The commands printed `3.56.3`, `(3, 56, 3)`, `pygobject-ok`, and `3.56.3`.

`gobject-introspection` is packaged as the scanner and base typelib
prerequisite for GTK Python programs. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/tools/gobject-introspection.yaml
./src/cli/target/debug/nex build pkg/dev/tools/gobject-introspection.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

The strict build passed reproducibility with checksum
`09843f502d39f8775ecca8a63b17d7d493dcacf6e6bce7a6ef943c8a8df8bb67`. A
checked-out smoke root showed `/usr/bin/g-ir-scanner` starts with
`#!/usr/bin/python3`, listed base typelibs under
`/usr/lib/girepository-1.0`, printed `g-ir-scanner 1.84.0`, printed
`g-ir-compiler 1.84.0`, resolved `GIRepository` through `GObject` and `GLib`,
and imported `giscanner` through Python. A consumer-style runtime smoke used
only `nex resolve gobject-introspection` output, confirmed no setuptools tree
was present, imported `distutils.cygwinccompiler` from the scanner package's
copied `distutils` tree, and ran `g-ir-scanner --version`.

`atk` now builds and ships its GIR and typelib files. Package proof:

```bash
./src/cli/target/debug/nex check pkg/libs/graphics/atk.yaml
./src/cli/target/debug/nex build pkg/libs/graphics/atk.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

The strict build passed reproducibility with checksum
`8a7ed17b10f173d438a2a58a04403a7dc63ee8ca299135b29d76797c9cba93c1`. A
checked-out smoke root with ATK and the scanner runtime closure verified
`/usr/lib/girepository-1.0/Atk-1.0.typelib` and
`/usr/share/gir-1.0/Atk-1.0.gir`, then ran:

```bash
/usr/bin/g-ir-inspect --version=1.0 --print-typelibs Atk
/usr/bin/g-ir-inspect --version=1.0 --print-shlibs Atk
```

The commands printed `GObject-2.0`, `GLib-2.0`, and `libatk-1.0.so.0`.

`gdk-pixbuf` now builds and ships its GIR and typelib files. Package proof:

```bash
./src/cli/target/debug/nex check pkg/libs/graphics/gdk-pixbuf.yaml
./src/cli/target/debug/nex build pkg/libs/graphics/gdk-pixbuf.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

The strict build passed reproducibility with checksum
`304e4165e52ac3d98ee48315981ce2f72b360ef6dc66100ec609159413c462a6`. A
checked-out smoke root with GDK Pixbuf and the scanner runtime closure verified
`/usr/lib/girepository-1.0/GdkPixbuf-2.0.typelib` and
`/usr/lib/girepository-1.0/GdkPixdata-2.0.typelib`, then ran:

```bash
/usr/bin/g-ir-inspect --version=2.0 --print-typelibs GdkPixbuf
/usr/bin/g-ir-inspect --version=2.0 --print-shlibs GdkPixbuf
/usr/bin/g-ir-inspect --version=2.0 --print-typelibs GdkPixdata
```

The commands printed dependent typelibs including `GObject-2.0`, `Gio-2.0`,
`GLib-2.0`, and `GModule-2.0`, and printed `libgdk_pixbuf-2.0.so.0`.

`harfbuzz` now builds and ships its GIR and typelib files while keeping Cairo
integration. Package proof:

```bash
./src/cli/target/debug/nex check pkg/libs/text/harfbuzz.yaml
./src/cli/target/debug/nex build pkg/libs/text/harfbuzz.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

The strict build passed reproducibility with checksum
`7956e7f1c0eb3dff719a61c9c1dd80df42c3402a682d0d1ddfb269c7e44d5b5b`. A
checked-out smoke root verified `/usr/lib/girepository-1.0/HarfBuzz-0.0.typelib`,
`/usr/share/gir-1.0/HarfBuzz-0.0.gir`, `/usr/bin/hb-view`, and
`/usr/lib/libharfbuzz-cairo.so.0`, then ran:

```bash
/usr/bin/g-ir-inspect --version=0.0 --print-typelibs HarfBuzz
/usr/bin/g-ir-inspect --version=0.0 --print-shlibs HarfBuzz
/usr/bin/hb-view --version
```

The commands printed `GObject-2.0`, `GLib-2.0`, `freetype2-2.0`,
`libharfbuzz-gobject.so.0`, `libharfbuzz.so.0`, and `hb-view (HarfBuzz)
10.1.0`.

`pango` now builds and ships its GIR and typelib files while keeping Cairo
support. Package proof:

```bash
./src/cli/target/debug/nex check pkg/libs/text/pango.yaml
./src/cli/target/debug/nex build pkg/libs/text/pango.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

The strict build passed reproducibility with checksum
`07d206a47a17b5e191c9da1593762e51b4006a41a4bb1b673498d71364d7d9ad`. A
checked-out smoke root verified five typelibs and five GIR files for `Pango`,
`PangoCairo`, `PangoFT2`, `PangoFc`, and `PangoOT`, then ran
`g-ir-inspect --version=1.0 --print-typelibs` and
`g-ir-inspect --version=1.0 --print-shlibs` for each namespace. The commands
reported dependent typelibs including `HarfBuzz-0.0`, `cairo-1.0`,
`freetype2-2.0`, and `fontconfig-2.0`, and shared libraries
`libpango-1.0.so.0`, `libpangocairo-1.0.so.0`, and
`libpangoft2-1.0.so.0`. The same root ran `pango-view --version` and printed
`pango-view (pango) 1.57.0`.

`desktop-dev` now assembles `dmidecode` and `fakeroot`, and the existing
Python dev bundle covers `python-pip`. Assembly proof:

```bash
./src/cli/target/debug/nex check asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
tmpdir=$(mktemp -d .nex/tmp/desktop-dev-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy systems/desktop-dev/0.0.1 "$tmpdir"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/dmidecode --version
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/fakeroot --version
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/fakeroot /usr/bin/id -u
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/python3 -m pip --version
```

The assembly build passed reproducibility with checksum
`f0e4075682c44315a395879720f34b0cd29119c88de30edef4754a21447f8f2b`. The
system smoke printed `3.7`, `fakeroot version 1.38.1`, `0`, and pip 24.0.

`libxcrypt-compat` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/libs/system/libxcrypt-compat.yaml
./src/cli/target/debug/nex build pkg/libs/system/libxcrypt-compat.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d .nex/tmp/libxcrypt-compat-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/libs/system/libxcrypt-compat/4.4.36/bundles/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/lang/python3/3.12.2/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/libffi/3.4.6/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/userland/coreutils/9.5/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/acl/2.3.2/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/attr/2.5.2/outputs/lib "$tmpdir"
ln -s usr/lib "$tmpdir/lib64"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/python3 -c 'import ctypes; lib = ctypes.CDLL("/usr/lib/libcrypt.so.1"); lib.crypt.argtypes = [ctypes.c_char_p, ctypes.c_char_p]; lib.crypt.restype = ctypes.c_char_p; print(lib.crypt(b"password", b"xx").decode())'
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/test '!' -e /usr/lib/libcrypt.so
```

The strict build passed reproducibility with checksum
`2203a79c62d344f1e52a1fd10aba3c73494aada6bd75043a1fbbe3e871992a64`. The
smoke printed `xxj31ZMTZzkVA` and confirmed no unversioned `libcrypt.so` file.

`desktop-dev` now assembles `libxcrypt-compat`. Assembly proof:

```bash
./src/cli/target/debug/nex check asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
tmpdir=$(mktemp -d .nex/tmp/desktop-dev-libxcrypt-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy systems/desktop-dev/0.0.1 "$tmpdir"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/python3 -c 'import ctypes; lib = ctypes.CDLL("/usr/lib/libcrypt.so.1"); lib.crypt.argtypes = [ctypes.c_char_p, ctypes.c_char_p]; lib.crypt.restype = ctypes.c_char_p; print(lib.crypt(b"password", b"xx").decode())'
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/test -e /usr/lib/libcrypt.so.1
```

The assembly build passed reproducibility with checksum
`9447359116ded895625e7baa760492be2d803a0a37a4e0c75883d40b2c90f727`, and the
system smoke printed `xxj31ZMTZzkVA`.

`Archive::Zip` and `Archive::Cpio` are packaged and built reproducibly as
prerequisites for `strip-nondeterminism`. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/perl/archive-zip.yaml
./src/cli/target/debug/nex check pkg/dev/perl/archive-cpio.yaml
./src/cli/target/debug/nex build pkg/dev/perl/archive-zip.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
./src/cli/target/debug/nex build pkg/dev/perl/archive-cpio.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d .nex/tmp/perl-archive-full-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/dev/perl/archive-zip/1.68/bundles/full "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/perl/archive-cpio/0.10/bundles/full "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/lang/perl/5.38.2/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/compression/bzip2/1.0.8/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/zlib/1.3.1/outputs/lib "$tmpdir"
ln -s usr/lib "$tmpdir/lib64"
mkdir -p "$tmpdir/dev"
: > "$tmpdir/dev/null"
printf 'abc' > "$tmpdir/tmp.input"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/perl -MArchive::Zip -MArchive::Cpio -e 'print "zip=$Archive::Zip::VERSION cpio=$Archive::Cpio::VERSION\n"'
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/crc32 /tmp.input
mkdir -p "$tmpdir/cpio-src"
printf 'abc' > "$tmpdir/cpio-src/file.txt"
(cd "$tmpdir/cpio-src" && printf 'file.txt\n' | cpio -o -H newc > ../sample.cpio)
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/cpio-filter /sample.cpio > /tmp/cpio-filter-smoke.out
```

The strict builds passed reproducibility with checksums
`464b8a5cc58610081ebcbfacbc22e41f4d048d9d905ac7075b0ed0149e73e4ec` for
`Archive::Zip` and
`9a0ebb1bd17bb1d1005461c6d4c92fbd5aef4d84085c70e8a436abdbf4868a95` for
`Archive::Cpio`. The smoke printed `zip=1.68 cpio=0.10`, `352441c2`, and
processed a valid CPIO archive with `cpio-filter`.

`strip-nondeterminism` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/tools/strip-nondeterminism.yaml
./src/cli/target/debug/nex build pkg/dev/tools/strip-nondeterminism.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d .nex/tmp/strip-nondeterminism-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/dev/tools/strip-nondeterminism/1.15.1/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/tools/file/5.45/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/perl/archive-zip/1.68/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/perl/archive-cpio/0.10/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/lang/perl/5.38.2/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/compression/bzip2/1.0.8/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/zlib/1.3.1/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/compression/xz/5.4.6/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/acl/2.3.2/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/attr/2.5.2/outputs/lib "$tmpdir"
ln -s usr/lib "$tmpdir/lib64"
mkdir -p "$tmpdir/dev"
: > "$tmpdir/dev/null"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/strip-nondeterminism --version
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/perl -MArchive::Zip -e 'my $zip = Archive::Zip->new; my $member = $zip->addString("abc", "file.txt"); $member->setLastModFileDateTimeFromUnix(1782649834); $zip->writeToFileNamed("/sample.zip") == 0 or die "write failed\n";'
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/strip-nondeterminism -T 1704067200 --normalizers +zip /sample.zip
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/perl -MArchive::Zip -e 'my $zip = Archive::Zip->new("/sample.zip"); my ($member) = $zip->members; print $member->lastModTime . "\n";'
```

The strict build passed reproducibility with checksum
`9d055b11556f52b0a8d4aa47c89f5e29039e52c3b6bf67e902e695c7175a9f07`. The
smoke printed `strip-nondeterminism version 1.15.1`, `Normalized /sample.zip`,
and the final timestamp `1704067200`.

`desktop-dev` now assembles `strip-nondeterminism` and the Perl archive
modules needed for ZIP and CPIO normalization. Assembly proof:

```bash
./src/cli/target/debug/nex check asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
tmpdir=$(mktemp -d .nex/tmp/desktop-dev-strip-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy systems/desktop-dev/0.0.1 "$tmpdir"
mkdir -p "$tmpdir/dev"
: > "$tmpdir/dev/null"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/strip-nondeterminism --version
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/perl -MArchive::Zip -e 'my $zip = Archive::Zip->new; my $member = $zip->addString("abc", "file.txt"); $member->setLastModFileDateTimeFromUnix(1782649834); $zip->writeToFileNamed("/sample.zip") == 0 or die "write failed\n";'
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/strip-nondeterminism -T 1704067200 --normalizers +zip /sample.zip
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/perl -MArchive::Zip -e 'my $zip = Archive::Zip->new("/sample.zip"); my ($member) = $zip->members; print $member->lastModTime . "\n";'
```

The assembly build passed reproducibility with checksum
`51dc353d72ac6d21a8db8928c43a3a94ad74b11f156ce12926e49655b1c91852`. The
system smoke printed `strip-nondeterminism version 1.15.1`,
`Normalized /sample.zip`, and the final timestamp `1704067200`.

The Go vendoring helper now strips inline comments from dependency `go.mod`
`go` directives before writing `vendor/modules.txt`. CLI proof:

```bash
cargo test --manifest-path src/cli/Cargo.toml go_vendor
cargo build --manifest-path src/cli/Cargo.toml
```

The targeted test run passed 2 tests, including
`go_vendor::tests::read_go_version_strips_inline_comments`, with 62 filtered
out.

`gopls` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/tools/gopls.yaml
./src/cli/target/debug/nex build pkg/dev/tools/gopls.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d .nex/tmp/gopls-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/dev/tools/gopls/0.18.1/bundles/dev "$tmpdir"
HOME="$tmpdir/home" GOTELEMETRY=off LC_ALL=C LANG=C "$tmpdir/usr/bin/gopls" version
```

The strict build passed reproducibility with checksum
`ca38c1f75de9de13f9a5a29c2c780fba62166e0c42bc2532e02442a8be9ad976`. The
smoke printed `golang.org/x/tools/gopls v0.18.1`.

`desktop-dev` now assembles `gopls`. Assembly proof:

```bash
./src/cli/target/debug/nex check asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
tmpdir=$(mktemp -d .nex/tmp/desktop-dev-gopls-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy systems/desktop-dev/0.0.1 "$tmpdir"
test "$(readlink "$tmpdir/usr/bin/gopls")" = "/nex/pkg/dev/tools/gopls/0.18.1/7ab2603e/usr/bin/gopls"
HOME="$tmpdir/home" GOTELEMETRY=off LC_ALL=C LANG=C "$tmpdir/nex/pkg/dev/tools/gopls/0.18.1/7ab2603e/usr/bin/gopls" version
```

The assembly build passed reproducibility with checksum
`d82f48fd24b599f61ddad1baa120bc6d86bea855acf3658062db103c2ed530aa`. The
smoke confirmed the `/usr/bin/gopls` symlink and printed
`golang.org/x/tools/gopls v0.18.1`.

`Tk` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/lang/tk.yaml
./src/cli/target/debug/nex build pkg/dev/lang/tk.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d .nex/tmp/tk-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/dev/lang/tk/8.6.14/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/lang/tcl/8.6.14/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/zlib/1.3.1/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/x11/libx11/1.8.10/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/x11/libxcb/1.17.0/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/x11/libxau/1.0.12/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/x11/libxdmcp/1.1.5/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/ncurses/6.4-20230520/outputs/lib "$tmpdir"
ln -sfn usr/lib "$tmpdir/lib64"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/lib/ld-linux-x86-64.so.2 --list /usr/lib/libtk8.6.so
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/tclsh <<'EOF'
proc check {condition message} {
    if {![uplevel 1 [list expr $condition]]} {
        puts stderr $message
        exit 1
    }
}
set dir /usr/lib/tk8.6
source /usr/lib/tk8.6/pkgIndex.tcl
set loader [package ifneeded Tk 8.6.14]
check {[string match "*libtk8.6.so*" $loader]} "Tk loader did not point at libtk8.6.so: $loader"
foreach path {/usr/bin/wish /usr/bin/wish8.6 /usr/lib/libtk8.6.so /usr/lib/tk8.6/pkgIndex.tcl} {
    check {[file exists $path]} "missing $path"
}
set rc [catch {package require Tk 8.6.14} msg]
check {$rc == 1} "Tk unexpectedly loaded without a display: $msg"
check {[string match "*DISPLAY*" $msg] || [string match "*display*" $msg]} "Tk failed for an unexpected reason: $msg"
puts "tk smoke ok: loader registered and Tk reached display initialization"
EOF
```

The strict build passed reproducibility with checksum
`0a0a4108b80b363e9d5a56d99f1bc4c815db39c730dbd14e915092b45ac72d38`. The smoke
resolved `libtk8.6.so`, verified `wish`, `wish8.6`, `libtk8.6.so`, and
`pkgIndex.tcl`, then confirmed `package require Tk 8.6.14` reached the
expected DISPLAY failure in a headless root.

`desktop-dev` now assembles Tcl and Tk. Assembly proof:

```bash
./src/cli/target/debug/nex check asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
tmpdir=$(mktemp -d .nex/tmp/desktop-dev-tk-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy systems/desktop-dev/0.0.1 "$tmpdir"
readlink "$tmpdir/usr/bin/tclsh"
readlink "$tmpdir/usr/bin/wish"
test -e "$tmpdir/nex/pkg/dev/lang/tcl/8.6.14/3b0c548c/usr/bin/tclsh"
test -e "$tmpdir/nex/pkg/dev/lang/tk/8.6.14/b254b510/usr/bin/wish"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/tclsh <<'EOF'
proc check {condition message} {
    if {![uplevel 1 [list expr $condition]]} {
        puts stderr $message
        exit 1
    }
}
set dir /nex/pkg/dev/lang/tk/8.6.14/b254b510/usr/lib/tk8.6
source /nex/pkg/dev/lang/tk/8.6.14/b254b510/usr/lib/tk8.6/pkgIndex.tcl
set loader [package ifneeded Tk 8.6.14]
check {[string match "*libtk8.6.so*" $loader]} "Tk loader did not point at libtk8.6.so: $loader"
foreach path {/usr/bin/wish /usr/bin/wish8.6 /nex/pkg/dev/lang/tk/8.6.14/b254b510/usr/lib/libtk8.6.so /nex/pkg/dev/lang/tk/8.6.14/b254b510/usr/lib/tk8.6/pkgIndex.tcl} {
    check {[file exists $path]} "missing $path"
}
set rc [catch {package require Tk 8.6.14} msg]
check {$rc == 1} "Tk unexpectedly loaded without a display: $msg"
check {[string match "*DISPLAY*" $msg] || [string match "*display*" $msg]} "Tk failed for an unexpected reason: $msg"
puts "desktop-dev tk smoke ok"
EOF
```

The assembly build passed reproducibility with checksum
`6172ff83da18dee80204cd77e28abcdabc762ad1585eaae37b3cb7679efa4c33`. The
system smoke confirmed `/usr/bin/tclsh` and `/usr/bin/wish` symlink targets
and printed `desktop-dev tk smoke ok`.

`efivar` is packaged and built reproducibly as an `efibootmgr` prerequisite.
Package proof:

```bash
./src/cli/target/debug/nex check pkg/libs/system/efivar.yaml
./src/cli/target/debug/nex build pkg/libs/system/efivar.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d .nex/tmp/efivar-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/libs/system/efivar/39/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
ln -sfn usr/lib "$tmpdir/lib64"
unshare --user --map-root-user --mount --root "$tmpdir" /usr/lib/ld-linux-x86-64.so.2 --list /usr/bin/efivar
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/efivar --help >/dev/null
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/efivar --list-guids >/dev/null
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/efisecdb --help >/dev/null
test -e "$tmpdir/usr/lib/pkgconfig/efiboot.pc"
test -e "$tmpdir/usr/include/efivar/efiboot.h"
test -e "$tmpdir/usr/lib/libefiboot.so.1"
```

The strict build passed reproducibility with checksum
`b911d7f84846fea201fd75d74da4bfdeb90e3d2cb6b7bf31c2e71c9df6543a6f`. The
smoke resolved `efivar` libraries and verified the command, GUID table, secure
boot database command, `efiboot` pkg-config file, header, and library.

`efibootmgr` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/system/efibootmgr.yaml
./src/cli/target/debug/nex build pkg/cli/system/efibootmgr.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d .nex/tmp/efibootmgr-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/system/efibootmgr/18/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/efivar/39/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/popt/1.19/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
ln -sfn usr/lib "$tmpdir/lib64"
unshare --user --map-root-user --mount --root "$tmpdir" /usr/lib/ld-linux-x86-64.so.2 --list /usr/bin/efibootmgr
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/efibootmgr --version
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/efibootdump --help >/dev/null
```

The strict build passed reproducibility with checksum
`d2de4fc669673a1dd80cb43a9e8622422fcfed132e8a689164a5daf3f5a7c43e`. The
smoke resolved runtime libraries, printed `version 18`, and verified
`efibootdump --help`.

`efibootmgr` is included in `desktop-dev`. Assembly proof:

```bash
./src/cli/target/debug/nex check asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
tmpdir=$(mktemp -d .nex/tmp/desktop-dev-efibootmgr-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy systems/desktop-dev/0.0.1 "$tmpdir"
readlink "$tmpdir/usr/bin/efibootmgr"
readlink "$tmpdir/usr/bin/efibootdump"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/efibootmgr --version
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/efibootdump --help >/dev/null
```

The assembly build passed reproducibility with checksum
`e0747e151da5ecb85860db843d9c54353d9298cfded23d4248cb28aa4f185b25`. The
system smoke resolved both public symlinks into the `efibootmgr` package,
printed `version 18`, and verified `efibootdump --help`.

`aws-cli` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/net/aws-cli.yaml
./src/cli/target/debug/nex build pkg/cli/net/aws-cli.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d .nex/tmp/aws-cli-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/net/aws-cli/1.45.36/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/lang/python3/3.12.2/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/zlib/1.3.1/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/ncurses/6.4-20230520/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/expat/2.6.2/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/libs/openssl3/3.3.1/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/libffi/3.4.6/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/gmp/6.3.0/bundles/dev "$tmpdir"
ln -sfn usr/lib "$tmpdir/lib64"
head -3 "$tmpdir/usr/bin/aws"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/aws --version
```

The strict build passed reproducibility with checksum
`aefc0443290ef0e75a65d79453a838bd55a8ba797c683311f11fe7a894ef0a87`. The
smoke verified the `#!/usr/bin/python3` shebang, the package-local
`site-packages` path shim, and printed
`aws-cli/1.45.36 Python/3.12.2 ... botocore/1.43.36`.

`aws-cli` is included in `desktop-dev`. Assembly proof:

```bash
./src/cli/target/debug/nex check pkg/cli/net/aws-cli.yaml
./src/cli/target/debug/nex check asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
tmpdir=$(mktemp -d .nex/tmp/desktop-dev-aws-cli-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy systems/desktop-dev/0.0.1 "$tmpdir"
readlink "$tmpdir/usr/bin/aws"
head -3 "$tmpdir/nex/pkg/cli/net/aws-cli/1.45.36/b53836b6/usr/bin/aws"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/aws --version
```

The assembly build passed reproducibility with checksum
`ea76879618a9fc0536e21de104436c8a6e793f716271359ed77aee0c67d8072b`. The
system smoke resolved `/usr/bin/aws` to
`/nex/pkg/cli/net/aws-cli/1.45.36/b53836b6/usr/bin/aws`, verified the path
shim, and printed `aws-cli/1.45.36 Python/3.12.2 ... botocore/1.43.36`.

`gdb` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/tools/gdb.yaml
./src/cli/target/debug/nex build pkg/dev/tools/gdb.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
./src/cli/target/debug/nex resolve gdb
tmpdir=$(mktemp -d .nex/tmp/gdb-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/dev/tools/gdb/17.2/bundles/full "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/ce80d6a13e70d5b91f217da6af3d078d10c85d2a609e7dcd732b753e5ecc1cac/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/expat/2.6.2/8e5ef00f8ec615c4f0c88b4b28b7272b6552d328a763056138c94ed73efdae07/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/toolchain/gcc/13.2.0/553fbd55598d795e833e38ffe4d3793a040d59f5c4c00ed950eff54d6695c78b/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/gmp/6.3.0/a1539c827b5c17d0f2fd28170204173ec7251ae076cfab06306fb838d454afee/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/compression/xz/5.4.6/f1ded8aa72dab5b5bb2253f5e83edb9ab96053ffc2d483f61f5f146cdddfa69d/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/mpfr/4.2.1/6f9cffcdf9709a4dfe78dabecb2622273f69dc6a0297804fa7d0f1d39f9b1fdc/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/ncurses/6.4-20230520/496defe5a75988c1f1fc0c1f92dd499b283495aaab992f42e4845d0de142665c/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/readline/8.2/9c04640d49d53f7682345748fc802c081113813eaf9b8b8039b6949604ab9130/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/zlib/1.3.1/6d7155108c951b13b1fd3ccb199f788bf19e635637e5722dbc957ffdb0fd38e2/files "$tmpdir"
ln -sfn usr/lib "$tmpdir/lib64"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/gdb --version
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/gdbserver --version
```

The strict package build passed reproducibility with checksum
`e1afaf71917dcdf41139f40112dbfcfe50bb3c6071cea1661656304d1882b4ca`.
The smoke printed `GNU gdb (GDB) 17.2` and `GNU gdbserver (GDB) 17.2`.

`desktop-dev` now assembles `gdb`. Assembly proof:

```bash
./src/cli/target/debug/nex check pkg/dev/tools/gdb.yaml
./src/cli/target/debug/nex check asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
tmpdir=$(mktemp -d .nex/tmp/desktop-dev-gdb-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy systems/desktop-dev/0.0.1 "$tmpdir"
readlink "$tmpdir/usr/bin/gdb"
readlink "$tmpdir/usr/bin/gdbserver"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/gdb --version
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/gdbserver --version
```

The assembly build passed reproducibility with checksum
`4f4cc4b87a336e8919e20a035ad9ca2246983c209e54b6095e2e1fb1140ff2be`.
The system smoke resolved `/usr/bin/gdb` and `/usr/bin/gdbserver` to
`/nex/pkg/dev/tools/gdb/17.2/0165e0e9/...` and printed the GDB and gdbserver
17.2 version banners.

`meld` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex format pkg/apps/misc/meld.yaml
./src/cli/target/debug/nex check pkg/apps/misc/meld.yaml
./src/cli/target/debug/nex build pkg/apps/misc/meld.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
./src/cli/target/debug/nex check pkg/apps/misc/meld.yaml
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root .nex/tmp/build_rootfs_meld_apps_misc /usr/bin/python3 - <<'PY'
import sys
sys.path.insert(0, '/nex/out/lib/usr/lib/python3.12/site-packages')
import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk
gi.require_version('GtkSource', '4')
from gi.repository import GtkSource
from meld.conf import __version__
print('gtk', Gtk.MAJOR_VERSION, Gtk.MINOR_VERSION, Gtk.MICRO_VERSION)
print('meld', __version__)
print('gtksource-ok')
PY
PYTHONPATH=/nex/out/lib/usr/lib/python3.12/site-packages LC_ALL=C LANG=C unshare --user --map-root-user --mount --root .nex/tmp/build_rootfs_meld_apps_misc /nex/out/bin/usr/bin/meld --version
```

The strict build passed reproducibility with checksum
`c896a665380f671a98eaba14b7022eaf130b26245ca6bacd091694565567671f`.
The generated package metadata uses manifest content hash
`46eb030aa87570c94305e857efdf7943a32d814ff9f94fddd74f88e85f158f58`.
The direct Python smoke printed `gtk 3 24 43`, `meld 3.22.2`, and
`gtksource-ok`. The launcher smoke printed `3.22.2`.

`desktop-dev` now includes Meld. Assembly proof:

```bash
./src/cli/target/debug/nex check pkg/apps/misc/meld.yaml
./src/cli/target/debug/nex check asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
```

The assembly build passed reproducibility with checksum
`2c88129d1c41d7a68057b94579b0c4fde39d6859a38e7fb64acc98bc8ea96bed`.
A checked-out `systems/desktop-dev/0.0.1` root exposed `/usr/bin/meld` as
`/nex/pkg/apps/misc/meld/3.22.2/46eb030a/usr/bin/meld`. The smoke ran
`meld --version`, which printed `3.22.2`, then inserted the Meld capsule's
`usr/lib/python3.12/site-packages` into `sys.path` and imported `Gtk`,
`GtkSource`, and `meld.conf`. The smoke printed `gtk 3 24 43`,
`meld 3.22.2`, and `gtksource-ok`.

## Context and Orientation

Relevant durable notes:

- `.agents/knowledge/agent-workflow.md`
- `.agents/knowledge/cli-testing.md`
- `.agents/knowledge/ostreefy-parity.md`
- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/reproducibility.md`
- `.agents/knowledge/system-assemblies.md`

Target rows from the matrix:

```text
aws-cli
dmidecode
efibootmgr
fakeroot
gdb
gopls
libxcrypt-compat
lldb
meld
python-pip
sqlitebrowser
strip-nondeterminism
tk
```

Likely assembly target:

- Put developer-only commands and GUI developer tools in `asm/desktop-dev.yaml`.
- Put boot maintenance or hardware inspection tools in `desktop-dev` unless
  evidence shows the daily runtime system needs them.

## Plan of Work

1. Inspect existing manifests and store refs for nearby packages: Go, Python,
   LLVM, SQLite, GTK, libxcrypt, util-linux, and boot tools.
2. Start with small native packages that have clear non-privileged smokes:
   `dmidecode`, `efibootmgr`, `fakeroot`, `gopls`, `python-pip` or its
   equivalent, and `libxcrypt-compat`.
3. Then handle medium or language-specific packages:
   `strip-nondeterminism`, `tk`, `gdb`, and `aws-cli`.
4. Handle heavier GUI and LLVM packages last: `meld`, `sqlitebrowser`, and
   `lldb`. Split a new sub-EP if this slice becomes too large or if LLVM GUI
   work would block independent smaller tools.
5. For each package, run:

   ```bash
   ./src/cli/target/debug/nex check <manifest>
   ./src/cli/target/debug/nex build <manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
   ```

6. Smoke each package from a checked-out output or from a built system with a
   non-network, non-privileged version or help command.
7. Add completed package refs to `asm/desktop-dev.yaml`, build the assembly
   with `--check`, and smoke the new commands from a checked-out
   `systems/desktop-dev/0.0.1` tree.
8. Update `.agents/ostreefy-parity-matrix.md` after each package reaches its
   final row state.
9. Commit checked work in small coherent commits with exact path staging.

## Validation and Acceptance

This sub-EP is complete when every target row either:

- has a built package manifest, assembly ref, and smoke proof from a package
  output or `desktop-dev`, or
- moves to a later sub-EP with a concrete reason recorded here and in the
  matrix.

Before each package commit, run `nex check`, the strict package build command,
and a package smoke check. Before an assembly commit, run `nex check` for
`asm/desktop-dev.yaml`, build `desktop-dev` with `--check`, and smoke the
new commands from the checked-out system.

## Idempotence and Recovery

Package builds may update checksums, generated outputs, resolution data, and
profiles. Keep those generated fields with the package manifest commit that
required them.

If a strict build updates a checksum, rerun the same command until the
reproducibility check passes. If the second pass fails, compare first and
second outputs and record candidate durable knowledge in
`.agents/SCRATCH_KNOWLEDGE.md`.

If a package is too large or blocked by a missing lower-level design, update
this sub-EP, update the matrix, and continue with independent rows unless
AGENTS.md defines the blocker as hard.

## Artifacts and Notes

Update these files during the sub-EP:

- Package manifests under `pkg/`
- `asm/desktop-dev.yaml`
- `.agents/ostreefy-parity-matrix.md`
- `.agents/SCRATCH_KNOWLEDGE.md`
