# Explain every remaining package-owned `/etc` path

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

After this work, every package manifest that currently declares a file below
`/etc` will have an explicit, tested reason for its layout. Programs that own
vendor defaults will find those defaults below `/usr`, temporary machine
overrides below `/run`, and lasting administrator choices below `/etc`.
Packages will stop creating product accounts or importing Buildroot, Yocto,
Edgebox, or another product's policy. Files whose external specification
requires `/etc` will remain there only when the evidence supports that choice.

The plan covers all 31 package manifests left by the 2026-08-14 output scan.
This is one large audit and implementation batch, but not one large commit.
Each package or tightly coupled package group must pass its own behavior test
and strict two-build check before its commit lands. A reader can inspect the
final inventory in this plan and rerun the package, assembly, and boot checks
named below.

## Progress

- [x] (2026-08-15 00:27Z) Audited the clean worktree, read the repository
  rules, listed `.agents/knowledge/`, and read the package, assembly,
  reproducibility, builder, and agent workflow notes that touch this plan.
- [x] (2026-08-15 00:47Z) Removed Netavark's Buildroot-specific firewall
  choice, corrected its package version to match both pinned 1.14.1 sources,
  strictly rebuilt it and all five direct or inherited assemblies, and tested
  the installed executable in flat and Nex-structured roots.
- [x] (2026-08-15 01:00Z) Retained the OpenCL specification's fixed Nvidia
  ICD path, exposed it through both driver runtime bundles, strictly rebuilt
  both packages and images, and called the OpenCL loader in each image.
- [x] (2026-08-15 01:13Z) Moved Foot's and Fuzzel's fully commented sample
  files from `/etc/xdg` to their documentation trees, strictly rebuilt both
  packages and all four desktop images, and parsed each installed example
  with the matching program.
- [x] (2026-08-15 01:35Z) Moved SwayNC's required package defaults to
  `/usr/share/xdg/swaync`, added `/run/xdg` below the normal XDG system
  directories, fixed layered reloads, strictly rebuilt the package and four
  desktop images, and ran the new path-order test in both package builds.
- [x] (2026-08-15 02:20Z) Moved Waybar's required defaults to
  `/usr/share/xdg/waybar`, implemented the complete XDG, transient, and vendor
  search order, selected its full bundle in the desktop assembly, and rebuilt
  and inspected all four affected system commits.
- [x] (2026-08-15 02:45Z) Retained the three specification-defined XDG
  autostart paths from Gnome Keyring and AT-SPI2, fixed AT-SPI2's incomplete
  runtime bundle, and rebuilt and exercised all four desktop systems.
- [x] (2026-08-15 03:02Z) Moved p11-kit's explicit example and FUSE's fully
  commented template into normal documentation paths, strictly rebuilt both
  packages and all four affected desktop systems, and exercised p11-kit's
  parser plus the installed FUSE helper.
- [x] (2026-08-15 03:12Z) Retained Bash Completion's documented compatibility
  and login-hook paths plus VTE's upstream system login hooks, loaded both
  Bash scripts with the packaged shell, and strictly rebuilt VTE twice.
- [x] (2026-08-15 03:38Z) Moved Attr's extended-attribute copy policy to
  `/usr/lib/xattr.conf`, added whole-file `/etc`, `/run`, `/usr` selection,
  rebuilt all eleven affected assemblies, exercised the policy through the
  installed Coreutils `cp`, passed the Edgebox smoke, and booted nex-systemd
  in QEMU.
- [x] (2026-08-15 03:46Z) Moved Slsh's startup file to `/usr/lib/slsh.rc`,
  added whole-file system lookup without weakening its existing environment
  override, and strictly rebuilt the package with six reader cases.
- [x] (2026-08-15 05:36Z) Fixed writable Zub checkouts after an Iptables
  override test exposed store-object corruption, pinned the repaired Zub
  revision, kept hardlinks only for immutable deployments, passed all 185 CLI
  tests, and used a strict Iptables rebuild to repair and verify the damaged
  database blob.
- [x] (2026-08-15 05:42Z) Moved Iptables' Ethernet protocol database to
  `/usr/lib/ethertypes`, added whole-file administrator and transient lookup,
  and strictly rebuilt the package with the installed `ebtables-translate`
  reader covering all tiers and an empty mask.
- [x] (2026-08-15 05:46Z) Moved Nftables' OS fingerprint database to
  `/usr/lib/nftables/osf/pf.os`, added whole-file administrator and transient
  lookup, and strictly rebuilt the package with the installed `nft` reader
  covering all tiers and an empty mask.
- [x] (2026-08-15 06:14Z) Strictly rebuilt Docker and Edgebox after the two
  firewall database changes, then extended the Edgebox smoke to run its
  installed Nftables reader against the assembled vendor fingerprints.
- [x] (2026-08-15 06:47Z) Moved ImageMagick's fourteen package XML files to
  `/usr/share/ImageMagick-7`, added its transient `/run` source without
  changing upstream merge rules, and strictly rebuilt it with the installed
  policy reader covering every source tier.
- [x] (2026-08-15 07:14Z) Rebuilt the four ImageMagick-consuming desktop
  assemblies twice, checked the finished base root's seventeen XML files and
  absence of `/etc/ImageMagick-7`, and used its installed tools to read policy
  and create and identify a one-pixel PNG.
- [x] (2026-08-15 07:33Z) Prepared the shared trust source by configuring
  p11-kit to merge administrator, transient, and vendor trust paths, then
  strictly rebuilt it with real extraction and blocklist tests.
- [x] (2026-08-15 09:55Z) Reworked CA Certificates as immutable vendor trust
  input plus a generic updater, strictly rebuilt it with real layered anchor
  and blocklist tests, rebuilt all six affected assemblies twice, exercised
  the merged trust policy in Nex and Edgebox roots, and booted nex-systemd in
  QEMU with the CA refresh service active.
- [x] (2026-08-15 10:15Z) Fixed file-level runtime materialization so an
  overlapping closure file replaces a read-only regular file or symlink
  instead of opening it in place, passed all 187 CLI tests, and resumed the
  OpenSSL strict build past the dependency checkout that exposed the bug.
- [x] (2026-08-15 11:05Z) Moved OpenSSL's package-owned configuration and
  helper scripts to `/usr/lib/ssl`, added administrator, transient, and vendor
  whole-file lookup without changing its certificate directory, strictly
  rebuilt the package and all eight affected assemblies, exercised both
  readers in the finished Edgebox root, passed the full Edgebox smoke, and
  booted nex-systemd in QEMU.
- [x] (2026-08-15 11:25Z) Removed D-Bus's obsolete empty `/etc` stubs, added
  transient system and session drop-in directories between its existing
  vendor and administrator directories, and strictly rebuilt it with a live
  policy reload test across all three tiers.
- [x] (2026-08-15) Removed the RPC database, NSS policy, and generated loader
  cache from the two private bootstrap roots, rebuilt both toolchains, built
  the next-phase test package twice, and ran its binary through the cleaned
  phase-zero loader in an `/etc`-free capsule.
- [ ] Freeze the exact 31-manifest inventory and record every installed
  `/etc` path, reader, override, reload path, upstream vendor-path feature,
  and governing external specification.
- [ ] Resolve the vendor-data, XDG, example, compatibility-link, and database
  group, with one checked commit per package or inseparable package pair.
- [ ] Give D-Bus, Slang, and Libvirt correct vendor-file lookup without
  weakening their administrator paths or explicit overrides.
- [ ] Resolve OpenSSL, CA certificates, Fontconfig, Linux-PAM secondary files,
  and the account databases with focused security and trust tests.
- [x] Audit both bootstrap manifests and keep only paths required to build the
  next phase in their private bootstrap roots.
- [ ] Audit every affected assembly overlay, rebuild every affected root
  twice, run focused root tests, and boot the affected Nex system in QEMU.
- [ ] Rerun exhaustive package and assembly scans, record every justified
  remaining `/etc` path, promote durable knowledge, and complete the human
  review gate.

## Surprises & Discoveries

- Observation: Large desktop assembly checks must run one at a time on this
  host.
  Evidence: four concurrent strict builds filled the task's available `/tmp`
  space and reported `Disk quota exceeded`; after removing only this plan's
  disposable audit roots, each assembly built twice and reproduced when run
  sequentially. The concurrent builds also raced while refreshing the shared
  `systems/desktop-vwl/0.0.1` parent ref.

- Observation: A literal search finds `/etc` in build-time tests and scratch
  build-root setup after a package stops declaring `/etc` output.
  Evidence: `pkg/cli/net/openssh.yaml` still creates temporary passwd, group,
  and SSH files to test its reader, but its generated outputs contain no
  package-owned SSH default below `/etc`. Count output entries, not every
  build-script string, for the final package inventory.

- Observation: The RTK `find` wrapper omits GNU `find` features needed for the
  knowledge and ExecPlan listings.
  Evidence: `rtk find ... -printf` rejected `-printf` and compound predicates;
  `rtk proxy find ... -printf` returned the exact filenames.

- Observation: Netavark's only `/etc` output came entirely from the Nex build
  script and even carried another distribution's name.
  Evidence: `pkg/apps/containers/netavark.yaml` wrote
  `50-buildroot-nftables.conf` itself. Both pinned source inputs are version
  1.14.1, while the old package metadata and assembly refs said 1.14.0.

- Observation: A package build dependency does not publish that dependency's
  sibling outputs through the built package's bundle.
  Evidence: Docker declares Iptables `bundles/full`, and its strict build root
  contained the new `/usr/lib/ethertypes`, but Docker's computed output needs
  publish only the Iptables binaries and libraries that Docker uses. Edgebox
  receives no `ethertypes` database or `ebtables-translate` through Docker.

- Observation: A sparse root can run Netavark directly but cannot initialize
  Podman's engine far enough for `podman info`.
  Evidence: both flat and Nex-structured assembly roots printed `netavark
  1.14.1`; the flat root's `podman info` stopped with `Error: no such file or
  directory` even with a private VFS root, runroot, and mounted `/proc`.

- Observation: Both Nvidia packages generated their OpenCL ICD correctly but
  omitted the `conf` output from both public bundles.
  Evidence: the strict package roots contained `nvidia.icd` and
  `libOpenCL.so.1`, while the old `full` and `runtime` bundle lists contained
  `config` but not `conf`. The two Nvidia assembly roots lacked the ICD until
  this audit added `conf` to both bundles.

- Observation: Foot and Fuzzel use standard XDG search order, but the files
  their build systems install below `/etc/xdg` contain only comments and empty
  section headings.
  Evidence: the pinned `config.c` files search the user directory before the
  ordered `XDG_CONFIG_DIRS` list and default that list to `/etc/xdg`; both
  shipped example files leave every built-in value unchanged.

- Observation: The desktop assembly already owns Foot's actual product choice
  as `/home/testuser/.config/foot/foot.ini` and selects only the Foot and
  Fuzzel binary outputs.
  Evidence: `asm/desktop-vwl/desktop-vwl.yaml` names both `outputs/bin` refs,
  while `desktop-vwl-overlay.yaml` creates the user-specific Foot file.

- Observation: SwayNC remembered the file that won its startup search and
  passed that selected path back as an explicit path during reload.
  Evidence: `ConfigModel.reload_config()` called
  `Functions.get_config_path(_path)`, so a vendor file selected at startup
  kept winning after an administrator created a higher-priority file.

- Observation: An assembly build without `--single` tried to rebuild 43 stale
  dependencies before it reached the image.
  Evidence: the first desktop-vwl command reported a 44-node build graph and
  started the phase-zero bootstrap chain. The same command with `--single`
  built only the image twice from its declared store refs.

- Observation: Waybar declared its two required defaults in a split output,
  but the desktop assembly selected only its binary output.
  Evidence: `pkg/desktop/wayland/waybar.yaml` exposed the old `conf` output,
  while `asm/desktop-vwl/desktop-vwl.yaml` named `outputs/bin`; none of the
  finished images could receive the defaults until the assembly selected
  `bundles/full`.

- Observation: Strict assembly builds remove their temporary `target` trees
  after they commit the finished system.
  Evidence: path tests below `.nex/tmp/build_rootfs_*_system/target` failed
  after successful builds, while `zub cat-file systems/<slug>/0.0.1:<path>`
  found the finished Waybar links and their package targets in all four system
  commits.

- Observation: AT-SPI2's old `full` bundle contained only its headers and
  libraries, even though the manifest declared two runtime daemons, two D-Bus
  activation files, a systemd user service, and an XDG autostart entry.
  Evidence: the old bundle selected only `dev` and `lib`; a desktop assembly
  received AT-SPI libraries through dependency flattening but omitted every
  activation path.

- Observation: XDG autostart entries differ from ordinary application
  defaults below `/etc/xdg/<package>`.
  Evidence: the Freedesktop Autostart Specification tells desktop sessions to
  scan `autostart` below `XDG_CONFIG_HOME` and every `XDG_CONFIG_DIRS` entry.
  The Base Directory Specification defaults `XDG_CONFIG_DIRS` to `/etc/xdg`,
  which makes `/etc/xdg/autostart` the standard system path.

- Observation: p11-kit labels its installed `pkcs11.conf.example` as a file
  that an administrator must copy before use.
  Evidence: the pinned example says it has no effect until copied to
  `/etc/pkcs11/pkcs11.conf`; the package's `test-conf` test passed after the
  manifest moved only the example and left the real administrator reader
  unchanged.

- Observation: FUSE installs a fully commented `fuse.conf`, but
  `fusermount3` still reads the administrator's fixed `/etc/fuse.conf` when a
  machine enables `user_allow_other` or changes `mount_max`.
  Evidence: the pinned `util/fusermount.c` opens `/etc/fuse.conf`, while the
  installed template contains no active line. Moving the template does not
  change the helper's administrator interface or its built-in behavior.

- Observation: Bash itself does not scan `profile.d`; the selected system
  profile decides whether to source that conventional directory.
  Evidence: Bash Completion's pinned README tells systems to use its
  `$sysconfdir/profile.d/bash_completion.sh` hook or source it from another
  startup file, while VTE's Meson build installs both shell hooks directly in
  `vte_sysconfdir/profile.d`. Moving either package alone would silently stop
  login-shell activation on systems whose profile reads only `/etc/profile.d`.

- Observation: Bash Completion has two separate legacy interfaces below
  `/etc`, and its program implements one of them directly.
  Evidence: `doc/configuration.md` documents `/etc/bash_completion.d` as the
  first default compatibility directory; the main `bash_completion` script
  searches it before its prefix-relative fallback. The Nex-created
  `/etc/bash_completion` link preserves the older source path named in user
  startup files.

- Observation: Attr reads `xattr.conf` once per process and caches the parsed
  action list.
  Evidence: the pinned `attr_parse_attr_conf()` returns immediately when its
  static action list is nonempty. The package test therefore starts a fresh
  production-linked consumer for each tier and mask case; short-lived callers
  such as Coreutils `cp` naturally see the selected file on each invocation.

- Observation: Coreutils receives libattr through its dependency closure but
  does not receive Attr's separate `conf` output.
  Evidence: assembled `cp --preserve=xattr` linked and ran before this change,
  but no finished base root contained `xattr.conf`. Adding the `conf` output
  to the five standalone base assemblies made the policy explicit and carried
  it into all six descendants.

- Observation: Slsh treats `SLSH_CONF_DIR` and its older `SLSH_LIB_DIR` alias
  as explicit single-directory overrides, then loads one system `slsh.rc`
  before a user's optional startup file.
  Evidence: the pinned `load_startup_file()` checks those environment
  variables first and returns after the first system file loads. The package
  test proved that an empty selected system file also stops the search.

- Observation: Zub's old default checkout shared writable file inodes with
  its content-addressed blob store.
  Evidence: an early Iptables test overwrote a checked-out `ethertypes` file;
  later strict builds calculated the correct 1362-byte file hash, but
  `zub cat-file` returned the corrupted 14-byte `UAPITEST` blob. Deleting refs
  did not help because the corrupted object remained at the expected hash.

- Observation: The corrected Zub writer can repair an object whose bytes no
  longer match its hash inputs.
  Evidence: Zub commit `0db2a06aa0b741eb537ef41e5864226586aced38`
  passed 195 tests. Nex then passed 185 tests, rebuilt Iptables twice, and
  `zub cat-file .../outputs/conf:usr/lib/ethertypes` returned the complete
  upstream database with SHA-256
  `ed38f9d644befc87eb41a8649c310073240d9a8cd75b2f9c115b5d9d7e5d033c`.

- Observation: Both Iptables Ethernet protocol lookups share the same
  process-global database stream.
  Evidence: pinned `libxtables/getethertype.c` opens `XT_PATH_ETHERTYPES` from
  both `setethertypeent()` and `getethertypeent()`. The patch routes both
  callers through one whole-file selector, so lookup by name and by number
  cannot disagree about the active tier.

- Observation: P11-kit's `trust` executable needs module registration data
  that no ELF dependency scanner can discover.
  Evidence: the first CA build had the executable and shared library but
  reported no module containing trust policy until p11-kit's `dev` bundle
  also selected its `misc` output with
  `/usr/share/p11-kit/modules/p11-kit-trust.module`.

- Observation: P11-kit's registered trust module also loads a dynamically
  named shared object that the CA updater's ELF and shell-command scans cannot
  discover.
  Evidence: the finished flat root found the module registration but could not
  load trust policy until the CA updater named
  `/usr/lib/pkcs11/p11-kit-trust.so` as a manual need. The final resolver
  closure and both finished-root tests contain both files.

- Observation: A command copied into a Nex structured root by another
  package's dependency closure does not automatically get a public
  `/usr/bin` link.
  Evidence: CA Certificates contained p11-kit's `trust` command in its capsule,
  but the nex-systemd root could only call it after the assembly selected
  p11-kit's `runtime` bundle explicitly.

- Observation: `systemctl --root` cannot prove that a unit enabled in Nex's
  factory `/etc` will run from the live persistent `/etc` after boot.
  Evidence: the offline query reported the CA updater disabled, while the QEMU
  serial log showed the service start and finish followed by
  `system-ca=ready` and `ASSERT-BOOT-PASS`.

- Observation: `trust extract --format=pem-directory-hash` writes its target
  directory without owner write permission.
  Evidence: the CA package's first strict checks passed every trust assertion
  but Nex could not remove the generated work directories before its second
  build. The build now removes its disposable extracted stores and restores
  owner write permission only on the immutable output copy before Nex stores
  it.

- Observation: File-level closure materialization copied regular files over
  existing paths without unlinking them first.
  Evidence: OpenSSL's build root first received Perl's full development
  bundle, then tried to overlay one read-only Perl runtime file and failed
  with `Permission denied`. After `copy_regular_file` unlinked a distinct
  destination first, the same strict build materialized all twenty closure
  commits and began compiling OpenSSL. Dedicated tests also prove that a
  regular file replaces a symlink without modifying the symlink target.

- Observation: OpenSSL's install target creates empty `certs` and `private`
  directories below `OPENSSLDIR`, even after the package moves every config
  file and helper script away from that directory.
  Evidence: the first relocation build left only those two directories below
  `/nex/out/etc/ssl`. The manifest now removes each known empty directory with
  `rmdir`, so an unexpected file still stops the build.

- Observation: OpenSSL's configure script can continue after reporting that
  `grep` is absent.
  Evidence: the earlier isolated configure reached compilation without Grep,
  but the package's focused provider tests could not run. Declaring phase-one
  Grep gave the build and its tests the command as an explicit input.

- Observation: The phase-zero toolchain published `/etc/rpc` through its
  development bundle even though no bootstrap command needed RPC lookup.
  Evidence: removing the file and its `conf` bundle member still built the
  complete phase-one Glibc package and the separate phase-one test package.
  A combined toolchain and test capsule ran `hello` through the packaged
  loader without an `/etc` directory.

- Observation: Phase-one Glibc declared `nsswitch.conf` and `rpc` in a sibling
  `conf` output that its development bundle did not select, while its `lib`
  output leaked the generated `/etc/ld.so.cache`.
  Evidence: the old `dev` bundle named only `dev`, `lib`, and `static`.
  Removing all three files still let the phase-one build compile, link, and
  run a program with the new sysroot and its `/usr/lib` loader path.

- Observation: Phase zero explicitly sets `stable_checksum: false`.
  Evidence: the standard strict command accepted `--check` but performed one
  build and published the semantic toolchain refs without a manifest
  checksum. Phase-one Glibc and the separate phase-one test package retained
  normal two-build reproducibility checks.

## Decision Log

- Decision: Cover all 31 remaining manifests in this ExecPlan.
  Rationale: The human asked for at least 20 items per plan and approved the
  full remaining package inventory. One plan also lets the final scan prove
  that no package escaped classification.
  Date/Author: 2026-08-15 / Codex

- Decision: Land small, independently useful commits inside this large plan.
  Rationale: This branch will merge without squashing. Every commit must build,
  pass the changed behavior test, and remain a useful `git bisect` point.
  Date/Author: 2026-08-15 / Codex

- Decision: Require an evidenced result instead of forcing the final count to
  zero.
  Rationale: Some external loaders and specifications may require a stable
  `/etc` path. Moving such a file without checking its consumer would break a
  normal package. The final acceptable state has zero unexplained entries,
  even if a small justified set remains.
  Date/Author: 2026-08-15 / Codex

- Decision: Keep product choices in assemblies and generic source changes in
  packages.
  Rationale: A Nex package must remain useful in flat roots, Nex-structured
  roots, and third-party assemblies. A package patch may implement normal
  Linux `/etc`, `/run`, and `/usr` lookup, but it may not name Nex or encode a
  jukebox, desktop, installer, Yocto, or Buildroot choice.
  Date/Author: 2026-08-15 / Codex

- Decision: Materialize package and assembly build roots as independently
  writable copies; reserve hardlinked checkouts for finalized immutable
  deployments.
  Rationale: a hardlink shares the store object's inode, so any build script
  that writes through it can silently corrupt the cache. Deployment roots are
  mounted read-only and retain the space-saving hardlink contract.
  Date/Author: 2026-08-15 / Codex

- Decision: Give Iptables whole-file `/etc/ethertypes`, `/run/ethertypes`, and
  `/usr/lib/ethertypes` lookup.
  Rationale: the file is a replaceable Ethernet protocol database, not a
  fragment directory. One selected file preserves administrator control,
  supports transient machine data, provides packaged defaults, and lets an
  empty higher-priority file mask lower data.
  Date/Author: 2026-08-15 / Codex

- Decision: Package Netavark without a default firewall driver.
  Rationale: Netavark upstream does not install the removed fragment. The Nex
  manifest created the whole `50-buildroot-nftables.conf` file and therefore
  imported another distribution's product choice. Podman and each assembly
  can choose a firewall driver when they need one.
  Date/Author: 2026-08-15 / Codex

- Decision: Retain Nvidia's `/etc/OpenCL/vendors/nvidia.icd` and expose it in
  both public bundles.
  Rationale: Khronos defines `/etc/OpenCL/vendors` as the Linux ICD directory
  in `https://registry.khronos.org/OpenCL/specs/unified/refpages/man/html/cl_khr_icd.html`.
  Moving this file would make the package incompatible with conforming ICD
  loaders. A public runtime bundle must contain it so an installed loader can
  discover the packaged vendor library.
  Date/Author: 2026-08-15 / Codex

- Decision: Install Foot's and Fuzzel's reference configurations below
  `/usr/share/doc/<package>/examples`.
  Rationale: Both upstream files only document built-in defaults. Installing
  either one in `/etc/xdg` claims an administrator choice without changing
  program behavior. The programs still honor the XDG Base Directory
  specification, including its default `/etc/xdg` administrator path.
  Date/Author: 2026-08-15 / Codex

- Decision: Install SwayNC's required defaults below `/usr/share/xdg/swaync`
  and search `/run/xdg` between the XDG administrator directories and that
  vendor directory.
  Rationale: SwayNC exits when it cannot find its JSON file and style sheet,
  so these files are package data rather than examples. User files and ordered
  `XDG_CONFIG_DIRS` entries still win. A temporary machine file can now
  override the package without writing persistent `/etc`.
  Date/Author: 2026-08-15 / Codex

- Decision: Keep only the caller's explicit SwayNC path across reloads.
  Rationale: SwayNC must rerun the layered search when the caller did not pass
  `--config`; otherwise a new user, administrator, or transient file cannot
  replace the fallback until the process restarts.
  Date/Author: 2026-08-15 / Codex

- Decision: Install Waybar's required defaults below
  `/usr/share/xdg/waybar` and search the caller override, user and legacy home
  paths, ordered XDG system directories, `/run/xdg`, and the vendor directory
  in that order.
  Rationale: Waybar needs a configuration and style to provide a useful bar,
  but package files must not occupy the administrator's `/etc` tree. Keeping
  the standard XDG directories and Waybar's compatibility paths ahead of the
  vendor files preserves existing user and machine choices.
  Date/Author: 2026-08-15 / Codex

- Decision: Retain Gnome Keyring's and AT-SPI2's autostart files below
  `/etc/xdg/autostart`.
  Rationale: Desktop sessions discover system autostart entries through
  `XDG_CONFIG_DIRS/autostart`, whose specified default is
  `/etc/xdg/autostart`. Moving these files to a private `/usr` directory would
  make standards-compliant sessions miss them. An administrator can still
  override or disable an entry by placing the same basename in a higher-priority
  XDG directory.
  Date/Author: 2026-08-15 / Codex

- Decision: Make AT-SPI2's `full` bundle contain all runtime outputs and add
  that bundle to the desktop package list.
  Rationale: A full AT-SPI2 install needs its launchers, activation metadata,
  libraries, service, autostart entry, and default accessibility setting.
  Dependency flattening supplied libraries to consumers but could not supply
  the package-owned daemons and metadata as public files.
  Date/Author: 2026-08-15 / Codex

- Decision: Install the p11-kit and FUSE reference files below
  `/usr/share/doc/<package>/examples` while preserving their real `/etc`
  readers.
  Rationale: p11-kit calls its file an example, and FUSE's file contains only
  comments for built-in defaults. Neither package should claim an
  administrator choice merely to ship instructions. A machine can still
  create `/etc/pkcs11/pkcs11.conf` or `/etc/fuse.conf` when it needs one.
  Date/Author: 2026-08-15 / Codex

- Decision: Retain Bash Completion's three and VTE's two system shell paths
  below `/etc`.
  Rationale: These files implement externally used shell integration points,
  not package defaults that the programs can search below `/usr`. Bash
  Completion reads its compatibility directory itself; existing startup files
  source its compatibility link; and system profiles source both packages'
  login hooks. An immutable assembly can place them in its factory tree and
  populate them only when the host has no same-name administrator file.
  Date/Author: 2026-08-15 / Codex

- Decision: Give Attr whole-file `/etc/xattr.conf`, `/run/xattr.conf`, and
  `/usr/lib/xattr.conf` lookup in that order, and select its vendor policy in
  every standalone base assembly.
  Rationale: `xattr.conf` controls which extended attributes file-copy tools
  preserve. A lasting administrator file and a temporary machine file must
  replace the packaged default without modifying a read-only deployment. An
  empty higher-priority file intentionally masks every lower rule. Libraries
  do not cause a split policy output to appear in a system, so each base that
  supplies file-copy tools must choose that output explicitly.
  Date/Author: 2026-08-15 / Codex

- Decision: Give Slsh whole-file system startup lookup through its configured
  administrator directory, `/run`, and `/usr/lib`, while retaining
  `SLSH_CONF_DIR` and `SLSH_LIB_DIR` as explicit overrides.
  Rationale: Slsh loads exactly one machine startup file, so the UAPI model
  maps directly to its existing reader. A lasting administrator file, a
  temporary file, or an empty mask can now replace the packaged startup file
  without modifying `/usr`; callers that name a directory keep the old exact
  behavior.
  Date/Author: 2026-08-15 / Codex

- Decision: Preserve ImageMagick's multi-file merge semantics while adding
  `/run/ImageMagick-7` between package and administrator data.
  Rationale: ImageMagick deliberately loads every same-name XML file, and its
  policy domains do not share one winner rule. Authorization uses the last
  matching rule, while resource ceilings cannot be raised by a later file.
  The package must provide every source in the documented order instead of
  replacing that model with whole-file selection. An empty XML file adds no
  rules; ImageMagick does not define it as a mask.
  Date/Author: 2026-08-15 / Codex

- Decision: Use p11-kit's ordered trust store as the source for system trust
  and generate compatibility databases from it.
  Rationale: trust anchors and blocklists form one merged policy, not a
  whole-file override. P11-kit already defines input priority and extraction
  formats for OpenSSL and other consumers. `/etc/pki/trust` holds lasting
  administrator choices, `/run/pki/trust` holds transient choices, and
  `/usr/share/pki/trust` holds package anchors.
  Date/Author: 2026-08-15 / Codex

- Decision: Treat `/etc/ssl/certs` as a generated compatibility database, not
  as the CA package's vendor input.
  Rationale: the CA package installs Mozilla anchors below
  `/usr/share/pki/trust/anchors`, ships a normal `update-ca-certificates`
  command that extracts p11-kit's current merged policy, and supplies a
  systemd oneshot that refreshes `/etc/ssl/certs` each boot. The package also
  publishes a pre-generated immutable copy below `/usr/lib/ssl/certs` for
  consumers that need trust before the first boot refresh.
  Date/Author: 2026-08-15 / Codex

- Decision: Keep OpenSSL's certificate directory at `/etc/ssl`, but install
  package-owned configuration and helper scripts below `/usr/lib/ssl` and
  select `/etc/ssl`, `/run/ssl`, then `/usr/lib/ssl` as whole files.
  Rationale: the generated live certificate database remains at the interface
  OpenSSL and administrators expect. `openssl.cnf` and `ct_log_list.cnf` are
  package defaults, so lasting and transient machine files must replace them
  without changing the read-only package tree. `OPENSSL_CONF` and
  `CTLOG_FILE` retain exact explicit-file behavior.
  Date/Author: 2026-08-15 / Codex

- Decision: Keep both private bootstrap sysroots free of host policy and
  generated loader state.
  Rationale: no next-phase command consumes their RPC or NSS databases, and
  the loader finds the bootstrap libraries directly below `/usr/lib` without
  a cache. Compile, link, and execution tests now prove those facts after the
  build removes `/etc`, while the normal runtime Glibc package continues to
  provide layered NSS and RPC defaults for real systems.
  Date/Author: 2026-08-15 / Codex

## Outcomes & Retrospective

Not started.

## Context and Orientation

UAPI.6 gives programs three places for machine-wide configuration. A program
uses lasting administrator files from `/etc`, temporary machine files from
`/run`, and package defaults from `/usr`. For one main file, the first existing
file in that order wins. An existing empty file masks lower files. For a
drop-in directory, the program sorts filenames, lets a higher-priority tree
shadow the same basename in a lower tree, and treats an empty file or a link
to `/dev/null` as a mask when its format allows that behavior.

`PHILOSOPHY.md` defines that model and keeps persistent host `/etc` outside a
read-only Nex deployment. `.agents/MANIFESTS_CODE_STYLE.md` requires normal,
reusable upstream packages and reviewable patches. `.agents/TESTING.md`
requires built behavior tests rather than path-existence checks alone.
`.agents/kb.md` and `.agents/knowledge/package-manifests.md` record the first
UAPI package waves. `.agents/knowledge/system-assemblies.md` explains how to
inspect split package outputs and Nex-structured roots. The ignored
`tmp/UAPI_TODO.md` mirrors the live checklist for this shared worktree, but
this tracked plan contains the authoritative scope.

The 31 manifest rows are:

| # | Manifest | Initial file class to verify |
|---:|---|---|
| 1 | `pkg/apps/containers/netavark.yaml` | Buildroot-named containers policy |
| 2 | `pkg/apps/graphics/imagemagick.yaml` | XML policy and type data |
| 3 | `pkg/apps/misc/ca-certificates.yaml` | generated trust links and bundle |
| 4 | `pkg/apps/security/gnome-keyring.yaml` | XDG autostart files |
| 5 | `pkg/apps/terminal/foot.yaml` | system XDG configuration |
| 6 | `pkg/bootstrap/phase0/toolchain.yaml` | bootstrap RPC database |
| 7 | `pkg/bootstrap/phase1/glibc.yaml` | bootstrap NSS, RPC, and loader cache |
| 8 | `pkg/cli/shells/bash-completion.yaml` | compatibility links and profile file |
| 9 | `pkg/core/ipc/dbus.yaml` | system and session main files |
| 10 | `pkg/core/userland/2nex-utilities.yaml` | passwd and group host state |
| 11 | `pkg/desktop/wayland/fuzzel.yaml` | system XDG configuration |
| 12 | `pkg/desktop/wayland/swaync.yaml` | XDG configuration, schema, and style |
| 13 | `pkg/desktop/wayland/waybar.yaml` | XDG configuration and style |
| 14 | `pkg/dev/libs/openssl3.yaml` | OpenSSL configuration and helper scripts |
| 15 | `pkg/dev/virt/libvirt.yaml` | daemon, client, network, filter, QEMU, and lock files |
| 16 | `pkg/libs/audio/pipewire.yaml` | PAM limits fragment |
| 17 | `pkg/libs/crypto/p11-kit.yaml` | example configuration |
| 18 | `pkg/libs/graphics/at-spi2-core.yaml` | XDG autostart file |
| 19 | `pkg/libs/graphics/fontconfig.yaml` | vendor aliases and local font policy |
| 20 | `pkg/libs/graphics/gtk3.yaml` | input-method data |
| 21 | `pkg/libs/graphics/nvidia-580.yaml` | OpenCL loader ICD |
| 22 | `pkg/libs/graphics/nvidia-current.yaml` | OpenCL loader ICD |
| 23 | `pkg/libs/net/libnl.yaml` | class and packet-location databases |
| 24 | `pkg/libs/net/libtirpc.yaml` | netconfig and reserved-port data |
| 25 | `pkg/libs/security/linux-pam.yaml` | secondary PAM module files |
| 26 | `pkg/libs/system/attr.yaml` | xattr policy |
| 27 | `pkg/libs/system/fuse3.yaml` | example or local policy |
| 28 | `pkg/libs/text/vte.yaml` | shell profile fragments |
| 29 | `pkg/libs/tui/slang.yaml` | slsh main file |
| 30 | `pkg/net/firewall/iptables.yaml` | ethertypes database |
| 31 | `pkg/net/firewall/nftables.yaml` | OS fingerprint database |

For each row, add an audit result to `Artifacts and Notes`. The result must
name all declared `/etc` outputs and choose exactly one primary outcome:

1. Move a vendor default below `/usr` through an existing upstream option.
2. Add a generic source patch that reads `/etc`, then `/run`, then `/usr`.
3. Remove true host state or product policy from the package and create the
   needed initial state in the relevant assembly.
4. Move a sample to `/usr/share/doc/<package>` or another normal example path.
5. Retain an externally fixed `/etc` path and cite the loader, specification,
   or upstream contract that requires it.

When one manifest contains several file classes, record and test one outcome
for each class. Empty directories for administrator configuration do not count
as vendor files, but the audit must say why the package creates them.

## Plan of Work

### Milestone 1: freeze the readers and standards map

Generate the 31-manifest list from declared output paths and compare it with
the table above. Inspect each manifest's build script and output groups.
Inspect the pinned upstream source for every program that reads one of those
files. Record command-line arguments, environment variables, reload paths,
include rules, vendor-directory support, and file-format ordering. Check the
authoritative upstream documentation or external specification for loader
paths such as OpenCL ICDs and trust databases.

Do not edit a parser until the notes name all readers of its packaged files.
Do not change `--sysconfdir=/etc` mechanically: that option often tells a
program where the administrator writes local files.

### Milestone 2: resolve vendor data, XDG files, examples, links, and databases

Start with packages whose consumers already search a standard vendor data or
XDG directory. Move package defaults through upstream build options when they
exist. Move genuine examples below `/usr/share/doc`. Remove the
Buildroot-named Netavark fragment unless an upstream-neutral package default
has a documented purpose. Check both Nvidia manifests together against the
OpenCL loader contract, but preserve separate reproducible package commits if
their artifacts differ.

Each commit must include a focused test that invokes the installed consumer or
parses the installed data through the real library. A link-only test is not
enough when the loader can exercise the file.

### Milestone 3: convert the remaining main-file readers

Give D-Bus, Slang, and Libvirt a normal vendor path. Preserve explicit file
arguments, environment overrides, user files, administrator fragment
directories, reload behavior, and parser ordering. Prefer a released upstream
feature. When the pinned release lacks it, carry a generic patch beside the
manifest with the header and zero-fuzz rules from
`.agents/MANIFESTS_CODE_STYLE.md`.

Test every tier, an empty higher-tier mask, same-basename drop-in shadowing
where supported, explicit overrides, and reload behavior where supported.
Start a built daemon and query its real interface when a service supplies the
only meaningful proof.

### Milestone 4: resolve trust, authentication, font policy, and accounts

Treat OpenSSL, CA certificates, Fontconfig, and Linux-PAM as security-sensitive
packages. Preserve `OPENSSL_CONF`, OpenSSL module lookup, `/etc/ssl` as an
administrator interface, certificate update behavior, Fontconfig's include
graph, PAM module semantics, and every supported local override.

Remove `passwd` and `group` creation from `2nex-utilities`. Put each product's
initial accounts in its assembly or a reusable assembly overlay, and prove
that flat and Nex-structured roots still resolve required users and groups.
Package manifests must not choose product users, passwords, shells, or IDs.

### Milestone 5: audit bootstrap-only paths

Inspect `pkg/bootstrap/phase0/toolchain.yaml` and
`pkg/bootstrap/phase1/glibc.yaml` as bootstrap programs, not normal runtime
packages. Record which next-phase command consumes each `/etc` file and whether
the file leaves the private bootstrap root. Reuse the final Glibc source
behavior where practical. Retain a bootstrap-local path only when a concrete
next-phase tool requires it and no built package exposes it to assemblies.

### Milestone 6: rebuild assemblies and close the scans

Audit these overlays and every child that inherits them:

- `asm/nex-systemd-overlay.yaml`
- `asm/edgebox-rootfs-overlay.yaml`
- `asm/desktop-vwl/desktop-vwl-overlay.yaml`
- `asm/installer/installer-overlay.yaml`

Classify their `/etc` files as initial host state or misplaced vendor policy.
Keep assembly choices explicit. Rebuild every assembly whose selected package
ref or initial host state changed. Test relevant commands and services inside
checked-out roots. Run the Edgebox smoke, desktop and installer checks, and a
direct QEMU boot for `nex-systemd` when their roots changed.

Finally, regenerate both package-output and assembly-overlay inventories.
Every remaining path must have an evidence entry in this plan. The final scan
must contain zero unexplained package-owned `/etc` files and zero product
choices in reusable package manifests.

## Concrete Steps

Run commands from `/var/home/wegel/work/wegelcorp/nex`. Every shell command in
this worktree starts with `rtk`; use `rtk proxy` when RTK shadows a required
GNU command feature.

Start or resume with:

    rtk git status --short --untracked-files=all
    rtk proxy find .agents/knowledge -maxdepth 1 -type f -printf '%f\n' | sort
    rtk semeja search 'relevant package or reader concept' .

For every changed package manifest, run:

    rtk proxy env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./src/cli/target/debug/nex check <manifest>
    rtk proxy env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./nex build <manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

The build command performs two builds because `--check` compares their
checksums. Run the focused behavior test from the package build root or a
checked-out package capsule. Record the exact command and its asserted output
under `Artifacts and Notes` before committing.

For a changed assembly, run:

    rtk proxy env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./nex build <assembly> --single --check --update-checksum --verbose

Check out a Nex-structured root with:

    rtk /home/wegel/work/perso/zub/target/debug/zub --repo .nex/repo checkout --copy systems/<slug>/<version> <temporary-directory>

Enter it with `unshare --user --map-root-user --mount --pid --fork chroot` when
absolute `/nex/pkg` links or root identity matter. Use the repository's
existing Edgebox, desktop, installer, and QEMU scripts that the audit finds;
record their exact paths before the first assembly commit.

Before every commit:

    rtk git diff --check
    rtk git status --short --untracked-files=all
    rtk git diff --cached --check
    rtk git diff --cached

Stage exact files only. Push each clean, checked commit to the current upstream
branch because the human authorized branch pushes during this work.

## Validation and Acceptance

This ExecPlan is complete only when all of these statements hold:

- `Artifacts and Notes` contains a final result for all 31 manifest rows and
  names every declared `/etc` path in each row.
- Every changed manifest passes `nex check`, the strict two-build command, and
  a test that exercises its installed consumer or service.
- Every parser change proves `/usr`, `/run`, and `/etc` priority, masks,
  explicit overrides, and live reload where those features apply.
- Every retained `/etc` file cites an authoritative loader, specification, or
  upstream contract. The final inventory contains no unexplained entry.
- No changed package manifest or package patch contains Nex, Edgebox, Soniq,
  Yocto, Buildroot, or another product's policy.
- Every affected assembly builds twice with one checksum and passes a root,
  service, installer, or boot test that can fail when the changed behavior is
  broken.
- The final Edgebox smoke passes if Edgebox changes. The desktop and installer
  runtime checks pass if their roots change. A changed `nex-systemd` root boots
  in QEMU and prints `ASSERT-BOOT-PASS`.
- `.agents/kb.md`, `.agents/SCRATCH_KNOWLEDGE.md`, the relevant files under
  `.agents/knowledge/`, and `tmp/UAPI_TODO.md` agree with the final evidence.
- Every commit on this branch remains a clean, buildable mainline commit.

### Completion Check

Not started. Before human review, compare every acceptance item above with the
finished commits and record exact commands, checksums, assertions, skipped
checks, and remaining risks here.

## Idempotence and Recovery

The strict package and assembly commands may be rerun. They refresh checksums,
profiles, outputs, and dependency maps, so inspect all generated manifest
changes before staging. Always pass `--single` to avoid refreshing an unrelated
dependency graph.

If a build stops, inspect its retained `.nex/tmp/build_rootfs_*` root and log,
fix the package, then rerun the same strict command. Do not commit a stale
checksum or one successful half of a two-build check. If a broad assembly build
reveals a missing or stale package ref, rebuild and test that package in its
own commit before resuming the assembly. Preserve unrelated human edits and
leave ignored local artifacts unstaged.

## Artifacts and Notes

### Initial inventory

The prior completed parser wave reduced the declared package-output inventory
from 35 to 31. The 31 rows in `Context and Orientation` are the frozen starting
set. `tmp/UAPI_TODO.md` has the longer historical 41-row list and records which
ten manifests earlier plans already cleared.

### Per-manifest results

Add one numbered result for each inventory row here. Each result must name the
paths, readers, chosen outcome, focused behavior test, strict build checksum,
affected assemblies, and commit.

1. `pkg/apps/containers/netavark.yaml`

   The old `conf` output declared
   `/etc/containers/containers.conf.d/50-buildroot-nftables.conf`. Podman reads
   that fragment as administrator or distribution policy; Netavark itself
   does not install or read it. Outcome 3 applies: the package no longer
   creates the Buildroot-named choice, and an assembly can add a firewall
   driver when its product policy needs one. Both pinned upstream inputs and
   the installed executable identify version 1.14.1, so the manifest and its
   two assembly refs now use 1.14.1.

   The strict package command built twice with checksum
   `c648f6c25191dabf9ec46602309ef5ea112e2ecce44c3570241383915374c56c`.
   The package build script also asserts that its new binary reports 1.14.1.
   `unshare --user --map-root-user --mount --pid --fork chroot ...
   /usr/bin/netavark --version` printed `netavark 1.14.1` in both the
   `flat-podman` and `desktop-dev` roots. Exact-file scans found no
   `50-buildroot-nftables.conf` in either root. A YAML assertion found no
   declared `/etc` output or `buildroot` text in the package manifest.

   The affected assemblies reproduced with these checksums: `flat-podman`
   `f8feefae4d82452da5dec9f6fe69e5d65c008a7e6a34273184838f0fe9bd31c9`,
   `desktop-vwl`
   `43515783ba6831b2f1bdbca64b32f85f13595e53b126c89a9d9506060c00cbc4`,
   `desktop-vwl-nvidia-580`
   `fc237af4027a16ad8cb4333803ac36eeac5f8635f80b6f60493c55c027fb4c76`,
   `desktop-vwl-nvidia-current`
   `63328e5de39d6649842c7b486398b03b5f5d94ace843c2edbbff1efb4ff62f4d`,
   and `desktop-dev`
   `3f45970d91af64016e788e93897e9ded96aa05d650764f7ded9e83bc664653ab`.
   Commit: `pkg: package netavark without distribution policy`.

2. `pkg/apps/graphics/imagemagick.yaml`

   The old `conf` output declared fourteen package XML files below
   `/etc/ImageMagick-7`: `colors.xml`, `delegates.xml`, `log.xml`, `mime.xml`,
   `policy.xml`, `quantization-table.xml`, `thresholds.xml`,
   `type-apple.xml`, `type-dejavu.xml`, `type-ghostscript.xml`,
   `type-urw-base35-type1.xml`, `type-urw-base35.xml`, `type-windows.xml`, and
   `type.xml`. Outcome 2 applies. ImageMagick already searches its configured
   package data before `/etc` and then user configuration, and it merges every
   same-name XML file it finds. The package now installs all fourteen files
   below `/usr/share/ImageMagick-7`.

   The generic patch with SHA-256
   `1ff57909d868eb927d4b10e556cbfcd50dd115d33ce2343fd116684f99a18198`
   adds `/run/ImageMagick-7` between the installed package directories and
   `/etc/ImageMagick-7`. It retains the documented `MAGICK_CONFIGURE_PATH`,
   XDG user, home, and installed paths. It also retains ImageMagick's merge
   rules: an empty XML file contributes no rules but does not mask lower
   files.

   The strict package command built twice with checksum
   `93c38ea45acc7aefa96ca7635ad18c02813147ea872c98d4db5eb9666fe230fd`.
   Both builds ran the installed `magick -list policy`, parsed synthetic
   package, transient, administrator, empty, explicit-environment, and
   restored-package files, and checked their load order. Store inspection
   found all fourteen complete files below `/usr/share` and no `/etc` output.
   Desktop-vwl selects the full bundle. Its four affected images built twice
   and matched: desktop-vwl
   `5805c8a10426ec29b4df0702148c63613822e008bf9a4c7a53fc67c3ad24a9e4`,
   Nvidia 580
   `358a03b813a2b7a1946638b0f9d5b189bb35537bbe4089ba77da5fde098c5aed`,
   Nvidia current
   `46da14b8f6da8121ce207efe7980cb6f714edd1e1b3ef343eb6b9267673c08d6`,
   and desktop-dev
   `3808ccac10d339f557a918166f38c140f2c0004b9b748675e0f01f0305055339`.
   The checked-out base root exposed all seventeen installed XML files, no
   `/etc/ImageMagick-7`, and a working public `magick`. `magick -list policy`
   reported `/usr/share/ImageMagick-7/policy.xml`; a one-pixel PNG round trip
   printed `1x1 srgb(255,0,0)`. Commits: `pkg: layer imagemagick
   configuration` and `asm: refresh imagemagick desktop roots`.

3. `pkg/apps/misc/ca-certificates.yaml`

   The old `certs` output declared `/etc/ssl/certs/ca-certificates.crt` and
   146 hash links below `/etc/ssl/certs`. Outcome 1 applies to the vendor
   input: the package now installs the Mozilla bundle only as
   `/usr/share/pki/trust/anchors/ca-certificates.crt`. P11-kit merges that
   input with `/run/pki/trust` and `/etc/pki/trust` anchors and blocklists.
   The package owns no `/etc` output.

   The generic `/usr/bin/update-ca-certificates` command extracts the merged
   `server-auth` anchors as a PEM bundle, individual PEM files, and subject
   hash links. It writes a sibling staging directory and swaps the complete
   target atomically. Its default target is the administrator-visible
   generated database `/etc/ssl/certs`; `--output-dir` lets packages and
   image builders generate another root. The package supplies a systemd
   oneshot enabled from `sysinit.target` so each boot refreshes the writable
   database from current vendor, transient, and administrator trust inputs.
   It also pre-generates `/usr/lib/ssl/certs` as an immutable compatibility
   copy for use before that refresh.

   The strict package command built twice with checksum
   `b36aa5a04f0b4bf1e6fc67b6bd207c5ed1c5f344135ee0e16700478c53dd825e`.
   Each build extracted all 148 Mozilla anchors, added one synthetic
   administrator and one transient anchor, verified both with the installed
   OpenSSL, then used an administrator blocklist to suppress one vendor
   certificate. `nex resolve ca-certificates --verbose` found the updater's
   shell, Coreutils, p11-kit executable, module registration, dynamically
   loaded trust module, and shared-library closure. A copy-mode checkout of
   `bundles/full` contained 148 certificates, 296 hash links, the updater,
   enabled unit, vendor input, and no `/etc` file.

   Nex-systemd, all four desktop assemblies, and Edgebox each reproduced on
   two strict builds. Their final checksums are nex-systemd
   `2944c5b44ffb1ec6024373dfd5e147b3923affedbdd3b80969a83c633cd24fef`,
   desktop-vwl
   `f4b9d7b2eef370e75bba5d27e35f6d97fc3fe957f3a5b1029209f9eba8895641`,
   Nvidia 580
   `69653abab8c4ed7cd14a632850f712824f6f65114d8d8d2279a964dc5ad41f23`,
   Nvidia current
   `a08bef42a0bd8a4931a07071bb1db2238db218ebc0ee7d13b7b5437108051f43`,
   desktop-dev
   `19ba38dc5df1a4c7df9f5039f73d1835408f3e62baabfcb679f5c5ba9cb35131`,
   and Edgebox
   `b9cc72998d7f224981c05b141103c5c95237bb4f0566dd795db4b68944ecbc5e`.
   A writable Nex root added an administrator anchor, regenerated 149
   certificates, listed that anchor through the installed `trust`, and
   verified it through the installed OpenSSL. The full Edgebox root smoke
   proved administrator blocklisting and restoration. A nex-systemd QEMU boot
   reported `system-ca=ready` and `ASSERT-BOOT-PASS`, which proves that the
   enabled oneshot regenerated the live compatibility store. This row is
   complete. Package commit: `pkg: separate system trust inputs`.

4. `pkg/apps/security/gnome-keyring.yaml`

   The `conf` output declares
   `/etc/xdg/autostart/gnome-keyring-pkcs11.desktop` and
   `/etc/xdg/autostart/gnome-keyring-secrets.desktop`. Outcome 5 applies.
   The Freedesktop Autostart Specification defines system entries below each
   `XDG_CONFIG_DIRS/autostart` directory, and the Base Directory
   Specification defaults `XDG_CONFIG_DIRS` to `/etc/xdg`. Both files keep
   their upstream paths so a normal desktop session can discover them. The
   governing contracts are
   `https://specifications.freedesktop.org/autostart-spec/latest/` and
   `https://specifications.freedesktop.org/basedir/latest/`.

   The strict package command built twice with checksum
   `e1dc7ae0dc2d9f09a095078333ef19ec4237a292a4c524a55a21452168dc691c`.
   `desktop-file-validate` accepted both installed entries, and their `Exec`
   commands name `/usr/bin/gnome-keyring-daemon` with the expected `secrets`
   and `pkcs11` components. In the checked-out desktop system,
   `gnome-keyring-daemon --version` printed `50.0`; direct store inspection
   also found its D-Bus service, systemd user service, and PAM module.

   All four affected images built twice and matched: desktop-vwl
   `3a62bb8d05a769f6a33a7671b52a1bded2cc049c74259684aa94b48478236d2d`,
   Nvidia 580
   `a9eeab8928ddc5fcec4c0b7f99c12ac0085f66ea7cc092105931ee7edbd6a2b3`,
   Nvidia current
   `1e5cfeff66808de2c2cfce8a7e9390e4c8262bbf2777fccce60f08cf8a22fb09`,
   and desktop-dev
   `25864e0b5fdb323397d674be022f29f6b480d5f32bf43beccfc098f03a417a7a`.
   Each system commit contains both files in its factory `/etc` tree.
   Commit: `pkg: complete desktop autostart runtimes`.

5. `pkg/apps/terminal/foot.yaml`

   The old `conf` output declared `/etc/xdg/foot/foot.ini`. Foot's pinned
   `config.c` checks `XDG_CONFIG_HOME`, then the ordered `XDG_CONFIG_DIRS`
   entries, and uses `/etc/xdg` when the system list is unset, as required by
   `https://specifications.freedesktop.org/basedir/`. The installed file has
   no active setting: it contains comments and empty section headings that
   document built-in defaults. Outcome 4 applies. The package now installs it
   as `/usr/share/doc/foot/examples/foot.ini`, and its `dev` bundle exposes
   the new `doc` output instead of creating system policy.

   The strict package command built twice with checksum
   `2f5a687e9128eb8dc9ce410f14d2dfe5bffefb6b8674a0aa920725d9497fef8b`.
   The packaged example passed Foot's own `--check-config` parser. Separate
   XDG tests proved user configuration wins over system directories and that
   earlier `XDG_CONFIG_DIRS` entries win over later entries. The rebuilt
   desktop-dev root ran `foot --version`, kept its assembly-owned user file,
   and contained no Foot file below `/etc/xdg` or the factory `/etc` tree.
   Commit: `pkg: move XDG config samples to docs`.

6. `pkg/bootstrap/phase0/toolchain.yaml`

   The old `conf` output declared `/etc/rpc`, and the public `dev` bundle
   selected that output. Glibc's RPC NSS functions can read this database,
   but no concrete phase-zero or phase-one build command uses an RPC lookup.
   Outcome 3 applies. The build now removes Glibc's generated file and the
   empty `/etc` directory before publishing the private toolchain root; the
   `dev` bundle no longer names a `conf` sibling.

   The standard strict command completed successfully. This seed manifest
   explicitly sets `stable_checksum: false`, so Nex performed one build
   rather than a two-build checksum comparison. After removing `/etc`, its
   build script compiled a program with the newly built cross compiler and
   sysroot, then ran it through the newly built loader and `/usr/lib` library
   path. Store scans of both `files` and `bundles/dev` returned no `etc/`
   path. The separate `bootstrap/phase1/test` package then built twice with
   checksum
   `15f1fde50324be3d478cf7c0dca895c12a9912ae09142bceef97e1c25cdf4a23`.
   A union checkout of that test and the phase-zero `dev` bundle had no
   `/etc` directory and printed `Hello from bootstrap!` through the packaged
   loader. No assembly selects this private bootstrap package. Commit:
   `pkg/bootstrap: keep toolchain roots free of host policy`.

7. `pkg/bootstrap/phase1/glibc.yaml`

   The old `conf` output declared `/etc/nsswitch.conf` and `/etc/rpc`; its
   `lib` output also declared `/etc/ld.so.cache`. The manifest itself created
   the NSS file, Glibc installed the RPC database, and `ldconfig` generated
   the cache. Outcome 3 applies to all three. The phase-one `dev` bundle never
   selected `conf`, so no later bootstrap package consumed the NSS or RPC
   files. All installed bootstrap libraries use the loader's normal
   `/usr/lib` path, so the generated cache is unnecessary too.

   The build now removes `ld.so.cache` and `rpc`, stops creating the NSS
   policy, and removes the empty `/etc` directory before it tests the result.
   Its two strict builds each compiled a program with the phase-zero compiler
   and the new phase-one sysroot, then ran the program through the new
   phase-one loader. They matched manifest checksum
   `5dc0e877627c7bd922858d74b9a4d4f4c87a7fb18a140c2ba1f1cebc531264a9`.
   Store scans of both `files` and `bundles/dev` returned no `etc/` path. No
   assembly selects this private bootstrap package; the normal runtime Glibc
   manifest still supplies layered NSS and RPC defaults for systems. Commit:
   `pkg/bootstrap: keep toolchain roots free of host policy`.

8. `pkg/cli/shells/bash-completion.yaml`

   The `conf` output declares `/etc/bash_completion`,
   `/etc/bash_completion.d/000_bash_completion_compat.bash`, and
   `/etc/profile.d/bash_completion.sh`. Outcome 5 applies to all three. The
   first path is a compatibility link for startup files that source the
   historical entry point. Bash Completion 2.17.0 itself searches the second
   path first when `BASH_COMPLETION_COMPAT_DIR` is unset, as documented by the
   pinned `doc/configuration.md`. The pinned README names the third path as
   the system login hook and explains how another startup file can source it.

   The existing strict build produced checksum
   `9b5ee85942099ee315a1d32912c79e6fea6dcc9f1fbc6a4a7169b283ff483c39`.
   A fresh checkout of the finished Edgebox system started the packaged Bash,
   sourced `/etc/profile.d/bash_completion.sh`, asserted version `2 17 0`,
   found the current `_comp_compgen_filedir` function and legacy `_filedir`
   wrapper, and printed the registered default completion loader. The same
   root's `/etc/bash_completion` link resolves to
   `/usr/share/bash-completion/bash_completion`. No manifest or assembly
   content changed. Commit: `pkg: document system shell integration paths`.

9. `pkg/core/ipc/dbus.yaml`

   The old `conf` output declared `/etc/dbus-1/session.conf` and
   `/etc/dbus-1/system.conf` alongside the real main files at
   `/usr/share/dbus-1/session.conf` and `system.conf`. D-Bus 1.16 labels the
   two installed `/etc` files obsolete, empty compatibility stubs and says
   they may be removed. Outcome 3 applies to those stubs. The daemon already
   starts from the `/usr/share` files and retains the administrator's legacy
   main-file includes, `session.d`, `system.d`, and `*-local.conf` paths.

   Outcome 2 applies to transient policy. The generic patch with SHA-256
   `5236950595379b17b634f59a2d901df06114e8b1c836b563cc1d33d116a0cdd3`
   adds `/run/dbus-1/session.d` and `/run/dbus-1/system.d` after the relative
   vendor drop-in directories and before `/etc` drop-ins. D-Bus loads every
   fragment and does not define same-basename shadowing or empty-file masks,
   so those semantics do not apply to this XML format. An explicit
   `--config-file` remains independent of the standard main files.

   The strict command built twice with checksum
   `43f55e0c3866202b868e500faf2c286becc77d4e98adf19bf4e50ef0791db9eb`.
   Both builds started the installed `dbus-daemon`, queried it with the
   installed `dbus-send`, and proved an administrator deny over a transient
   allow over a vendor deny. Removing each higher file and sending SIGHUP
   changed the live policy in the expected order. A separate explicit main
   file started and answered the same query. The stored `conf` output now
   contains only the two complete `/usr/share` files and no `/etc` file. No
   assembly selects this sibling `conf` output; current systems use
   dbus-broker's service configuration and D-Bus's library closure, so no
   finished root changed. Commit: `pkg: layer dbus policy directories`.

11. `pkg/desktop/wayland/fuzzel.yaml`

   The old `conf` output declared `/etc/xdg/fuzzel/fuzzel.ini`. Fuzzel uses
   the same XDG user-then-system search contract and defaults the system list
   to `/etc/xdg`. Its installed file also contains only comments and empty
   section headings, so outcome 4 applies. The package now installs it beside
   its other documentation as
   `/usr/share/doc/fuzzel/examples/fuzzel.ini`; it does not create a system
   choice merely to ship the sample.

   The strict package command built twice with checksum
   `78d7ee3d18cc65f70a0908f2a81fa6823c704c3c3dba0e80cf74b3a81676ebab`.
   Fuzzel's own `--check-config` parser accepted the packaged example. The
   desktop-dev root ran `fuzzel --version` and contained no Fuzzel file below
   `/etc/xdg` or the factory `/etc` tree. All four inherited desktop images
   reproduced after both sample moves: desktop-vwl
   `146092fe5c94c50a9441ad67811d7ffc105079f7343cb3ebe787a3d044029c95`,
   Nvidia 580
   `a62e10f7fd97b04d88039ca6468ee59447723c5e930b448ccef2cac664fd237d`,
   Nvidia current
   `7770f9719d3f698d501a9474913fc0475de74de2e3f4be9998856f290febc2b5`,
   and desktop-dev
   `5a1291abece4a8ff9a5b4ee5ca8b9c0b14b135578012a627ff2a6574384167ae`.
   Commit: `pkg: move XDG config samples to docs`.

12. `pkg/desktop/wayland/swaync.yaml`

   The old `conf` output declared `/etc/xdg/swaync/config.json`,
   `configSchema.json`, and `style.css`. Unlike the Foot and Fuzzel examples,
   SwayNC requires its JSON file and style sheet and exits when neither a
   caller nor the package supplies them. Outcome 1 applies. The package now
   installs all three files below `/usr/share/xdg/swaync`. The reader checks
   an explicit path, the user's XDG directory, each ordered
   `XDG_CONFIG_DIRS` entry, `/run/xdg`, and the compiled vendor directory.
   Style lookup uses the same layers while its packaged base style omits the
   user layer.

   The patch also keeps the caller's explicit `--config` path separately from
   the selected file. Reload now reruns the layer search when the caller did
   not specify a path, so a new higher-priority file wins without a process
   restart. A new Meson test creates files one layer at a time and proves
   vendor, transient, both ordered system directories, user, and explicit
   priority. Both strict builds passed that test and the upstream schema test,
   with package checksum
   `7bff7d16653b7f33fe8774d4fd2a743d78046dd97a68e092792df9d4c7e6e5b1`.
   The built executable contains `/run/xdg` and `/usr/share/xdg/swaync`, and
   the installed JSON schema path names
   `/usr/share/xdg/swaync/configSchema.json`.

   All four affected images built twice and matched: desktop-vwl
   `e3232eeedb410b7c68c2a309e71b0413c0ad7dd6eae6c77601564acbcb539075`,
   Nvidia 580
   `2d18167c0c57feffc940ba92ba86ddcb844f59e2bb142a16d0cf62abb0242947`,
   Nvidia current
   `631a6104ac9c59b17c89ec6735f5558080a96a9d47ddfe6936d02a6bf9b01168`,
   and desktop-dev
   `11fb5bcff706e44ea51cd1d49408dbf9f5aec2e21bc58f7d774686882e4f31ed`.
   Each retained root contains the three vendor files below `/usr/share/xdg`
   and no SwayNC file below `/etc/xdg`. Commit: `pkg: layer swaync system
   configuration`.

13. `pkg/desktop/wayland/waybar.yaml`

   The old `conf` output declared `/etc/xdg/waybar/config.jsonc` and
   `/etc/xdg/waybar/style.css`. Waybar uses these files as working defaults,
   not merely commented examples, so outcome 2 applies. A generic source
   patch installs both files below `/usr/share/xdg/waybar` and searches
   `WAYBAR_CONFIG_DIR`, the XDG user directory, the legacy `$HOME/waybar`
   directory, each ordered absolute `XDG_CONFIG_DIRS` entry, `/run/xdg`, the
   compiled vendor directory, and the source-tree fallback. Empty or relative
   XDG entries cannot escape that contract.

   The new Meson test checks the complete directory vector and selects a
   distinct file from the explicit, user, legacy, two administrator,
   transient, and vendor tiers. It also proves that an empty administrator
   file masks the lower transient and vendor files. Both strict builds passed
   that test and produced checksum
   `a8858f0342b4945abdb26fbe7327b040abdcab8563cd781b94e7555c640df41f`.
   The packaged executable contains `XDG_CONFIG_DIRS`, `/run/xdg/waybar`, and
   `/usr/share/xdg/waybar`. The patch applies to the pinned source with
   `git apply --check --cached` and has SHA-256
   `d0633fbb8510c5c9b261adb24879d256d917ba3b87c2bdc93ceac45aa92b9024`.

   The desktop assembly previously selected only `outputs/bin`, which omitted
   both required files. It now selects `bundles/full`. All four affected
   images built twice and matched: desktop-vwl
   `1c3835d17c39324dc2654ecd651272c79795c4d0f1b04919d9b681addca91bc0`,
   Nvidia 580
   `e7d24eb30b488cfb8b24a22da03215bc73e5213310dd51107efa229fa658f18e`,
   Nvidia current
   `c20f396b338c1264040e5ae5a27391271a4ba23f847ccee400e0fe033a45ebd3`,
   and desktop-dev
   `5f88f6be624f270a7e0017e836c701ccfd7a82eef22c24cfc5e6aef49971c5d5`.
   Direct `zub cat-file` checks found both public links and both package files
   in every system commit and found no `/etc/xdg/waybar`. Commit: `pkg: layer
   waybar system configuration`.

14. `pkg/dev/libs/openssl3.yaml`

   The old `conf` output declared `openssl.cnf`, `ct_log_list.cnf`, their
   distribution copies, and three helper scripts below `/etc/ssl`. Outcome 2
   applies to the two active configuration files, and outcome 1 applies to
   the remaining package files. The generic patch selects the first existing
   `openssl.cnf` or `ct_log_list.cnf` from `/etc/ssl`, `/run/ssl`, and
   `/usr/lib/ssl`. A non-ENOENT or non-ENOTDIR error selects that path instead
   of silently falling through, and an empty selected file masks lower files.
   Exact `OPENSSL_CONF` and `CTLOG_FILE` values still win. The package installs
   every config file and helper only below `/usr/lib/ssl` while retaining
   `--openssldir=/etc/ssl` for the generated certificate database.

   The patch applies with `patch -Np1 --batch --fuzz=0` and has SHA-256
   `21e1b7e249eb3463370843cf94e1a2ef76e692dbd81cdb8b3d9aeb59956808a3`.
   The strict command built twice, passed its provider, config precedence,
   empty-mask, explicit-file, and CT log API tests, and reproduced checksum
   `57252e416e94f89427dadaa413079a456f8015783f0e2bfc317b6a6e91b7bb1d`.
   A copy-mode root contained all seven package paths below `/usr/lib/ssl`, no
   package `/etc`, and a working OpenSSL 3.3.1 executable. The installed
   executable reported `OPENSSLDIR: "/etc/ssl"`, loaded the vendor default,
   rejected an invalid transient file, accepted an empty administrator file
   as the higher mask, and honored an explicit config that activated the
   legacy provider. The installed CT API probe likewise selected vendor,
   transient, administrator-mask, and explicit `CTLOG_FILE` cases.

   Flat-systemd and nex-systemd now select the sibling `conf` output
   explicitly, which supplies the configuration to their existing OpenSSL
   library consumers without adding the command-line tools. Both bases and
   all six descendants built twice and matched. Their final checksums are
   flat-systemd
   `a69b1dbbb5cf138dc3b5fb8ecad29c64815a114f97e1845a04ddcd16f45462bb`,
   flat-podman
   `4d09d72f7a7fa592f9183d6b70adcb1b3ce70b189ee9b75366658187794b283a`,
   Edgebox
   `90a6295f1c7f5fe3a823dc64c961b258d729529602421e6be923a3ed25f609d8`,
   nex-systemd
   `b25e5ad96c14ae4d7d1f196aa752a30b533596340e6482c9add9fa132301d852`,
   desktop-vwl
   `db2289b058f776e232fea1526cf1895dc384bd15854461ae712950102e1cfcea`,
   Nvidia 580
   `9326e0cb5b3b03c09a65da44d938e10aa4822b96211b0a831e79f0f40781a7b5`,
   Nvidia current
   `39974275fd144236035356692fb3cc760c3805ea2a5d2ec0ccbae65e6a9dcd9f`,
   and desktop-dev
   `8f2d83f00eb70c0eb226ddd988e4388c974a2e1a5de58fd0ab5d51dc28af531a`.

   The checked-out Edgebox root contained both vendor files below
   `/usr/lib/ssl` and neither policy file below `/etc`. Its assembled OpenSSL
   library retained `OPENSSLDIR: "/etc/ssl"`, loaded the default provider,
   rejected an invalid transient main file, and accepted an empty
   administrator mask. Its CT reader selected a valid transient file, then
   let invalid and empty administrator files override or mask that file. The
   full Edgebox root smoke passed. A direct nex-systemd QEMU boot refreshed
   the live CA database, printed `system-ca=ready` and `ASSERT-BOOT-PASS`, and
   powered off. Commits: `pkg: layer openssl configuration` and `asm: include
   openssl vendor configuration`.

17. `pkg/libs/crypto/p11-kit.yaml`

   The old `conf` output declared
   `/etc/pkcs11/pkcs11.conf.example`. The file itself says that p11-kit does
   not use it until an administrator copies it to
   `/etc/pkcs11/pkcs11.conf`, so outcome 4 applies. The package now installs
   it as `/usr/share/doc/p11-kit/examples/pkcs11.conf` and exposes that path
   through a `doc` output. The program's real administrator path remains
   `/etc/pkcs11/pkcs11.conf`.

   P11-kit now also loads trust policy from `/etc/pki/trust`,
   `/run/pki/trust`, and `/usr/share/pki/trust` in descending priority. Both
   strict builds passed its real `test-conf` parser, extracted three anchors
   placed across those tiers, and proved that an administrator blocklist entry
   removed the same vendor certificate. They matched package checksum
   `d8524f53a7e4fc14400f74404828eff6059072c1457baab610eac5dc5ba889a6`.
   The generated package output contains the documentation file and declares
   no `/etc` path. Its public `dev` bundle includes the module registration
   file as well as the `trust` command and library, so a consumer can use the
   installed trust policy. The CA package names the command and registration
   file as runtime needs. Commits: `pkg: move configuration samples to docs`,
   `pkg: layer p11 trust sources`, and `pkg: separate system trust inputs`.

18. `pkg/libs/graphics/at-spi2-core.yaml`

   The `conf` output declares
   `/etc/xdg/autostart/at-spi-dbus-bus.desktop`. Outcome 5 and the same
   Freedesktop specifications from row 4 apply. The entry runs
   `/usr/libexec/at-spi-bus-launcher --launch-immediately`; moving it outside
   `XDG_CONFIG_DIRS/autostart` would prevent a normal desktop session from
   finding it. The same output also declares the vendor file
   `/usr/share/defaults/at-spi2/accessibility.conf`, which already lives
   outside `/etc` and needs no move.

   The audit found that `bundles/full` selected only `dev` and `lib`. It now
   includes `bin`, `conf`, `lib`, and `misc`, and the desktop assembly selects
   that bundle explicitly. The strict package command built twice with
   checksum
   `0f72c43c2ed8b776f7defda22ccd90dc539937900e6705591e2a37f41a482fd8`.
   `desktop-file-validate` accepted the installed entry. The assembled
   `at-spi2-registryd --help` command exited successfully, and
   `at-spi-bus-launcher --launch-immediately` reached its session-bus connect
   before the isolated chroot, which has no session bus, rejected the
   connection. Direct store inspection found both daemons, both D-Bus
   activation files, the systemd user service, `libatspi.so.0`, and the
   accessibility default in the finished desktop system.

   The four system checksums and factory-tree assertions match row 4.
   Commit: `pkg: complete desktop autostart runtimes`.

21. `pkg/libs/graphics/nvidia-580.yaml`

   The `conf` output declares `/etc/OpenCL/vendors/nvidia.icd`, whose content
   names `libnvidia-opencl.so.1`. Outcome 5 applies. The Khronos ICD extension
   reference fixes this directory on Linux, so the manifest retains the path.
   The package already generated the file but neither public bundle selected
   `conf`; `full` and `runtime` now include it.

   The strict package command built twice with checksum
   `58b3a59dc14fcc68b8d755b4a153d7b5e489548e2b803f4a7f1cc85897d2885a`.
   A C smoke program linked the packaged `libOpenCL.so.1`, called
   `clGetPlatformIDs`, and returned the expected `-1001` without a GPU. In the
   rebuilt image, the same call loaded the Nvidia ICD far enough to attempt
   the Nvidia kernel module before returning `-1001`. The image placed the ICD
   at `/usr/share/factory/etc/OpenCL/vendors/nvidia.icd` and reproduced with
   checksum
   `e833ab46831dfa9cc16b17c784f147c88b23567b4b98b9a03c7bf9f5b085ef88`.
   Commit: `pkg: include nvidia OpenCL ICDs at runtime`.

22. `pkg/libs/graphics/nvidia-current.yaml`

   This branch declares the same `/etc/OpenCL/vendors/nvidia.icd` path and
   names the same `libnvidia-opencl.so.1` soname. Outcome 5 and the Khronos
   evidence above apply. Its `full` and `runtime` bundles now include `conf`.

   The strict package command built twice with checksum
   `1bf7490765156c555325cfd1efa3ff26466e3f20af023677085a88f057b97b06`.
   The same installed-library test called `clGetPlatformIDs`; the assembly
   call attempted to load the Nvidia module and returned the expected `-1001`
   on this non-Nvidia test host. The image placed the ICD in its factory tree
   and reproduced with checksum
   `43c20f709c15622e278ae0f1bf486be49f1ae578d377d6586d47498b91fc0a57`.
   Commit: `pkg: include nvidia OpenCL ICDs at runtime`.

26. `pkg/libs/system/attr.yaml`

   The old `conf` output declared `/etc/xattr.conf`. Libattr's
   `attr_copy_action()` reads this policy when file-copy programs decide which
   extended attributes to preserve. Outcome 2 applies. The generic patch with
   SHA-256
   `e3c468b1ba29ed573bdf8424c880884dce9c5db6b847c9094faa088db83ec1a6`
   selects the first complete file from `/etc/xattr.conf`,
   `/run/xattr.conf`, and `/usr/lib/xattr.conf`; an empty higher file masks
   lower policy. The packaged default now lives only at the vendor path.

   The strict package command built twice with checksum
   `2fba331aba23c967ea505c421dafcf8292abd130d4b69a37077e2377599e7d1f`.
   Its test linked against the installed libattr and proved the vendor file,
   a transient override, a lasting administrator override, an empty
   administrator mask, and the compiled no-file default. Because libattr
   caches the parsed list, the test used a fresh consumer process for each
   case.

   The five standalone base assemblies now select Attr's `conf` output; all
   descendants inherit it. All eleven affected systems built twice and
   matched: flat-minimal
   `1dd7c09bc51ff7f23fb904ff786c12d6d0a95eb21570b69f6aac58bca2a50d69`,
   flat-systemd
   `90fafdd1915f50aa96fdd994eff3cad62a336222757eca8be928913c4c8a5d87`,
   nex-minimal
   `b49d191e88a32fccac63373246acd7bac4efb271547e410fc562c041a0bcde5f`,
   nex-systemd
   `20375ce353f9bac9be3f107499fefa654f357f9d70944d2f661030353702d1b4`,
   installer
   `968fa77f837379bdf866108cee311a83b6af094818b059df7ef5008424546b8a`,
   flat-podman
   `88153a7977e75d17a9d7e944b14436be7b581f46a5e33623db6edfd639819a65`,
   Edgebox
   `4430d5b3ce4b91c1fc4ca71bb9a3b33d13dc72b89f5e9d67291dbaa4aacded83`,
   desktop-vwl
   `1eee5cb9d9ff7a2ba76a220702bd8ba320843af8b1ff88b10e72c5df358d09f4`,
   Nvidia 580
   `58e63fd12d7359b1e88466be454681e5fe26a02665838689ddd761403ed90372`,
   Nvidia current
   `b4adc76117103b37f1e8b40928eff8d35518dcbdc8e5ff1ea27631e067e4d017`,
   and desktop-dev
   `ebd5f9fc50220d3b0b31bedbb86c8940fc4cd5e3923441ca1a02b60107c4d937`.

   In checked-out flat and Nex-structured roots, the packaged Coreutils
   `cp --preserve=xattr` copied `user.keep`, skipped the vendor rule
   `user.Beagle.*`, and obeyed a replacement `/etc/xattr.conf`. Desktop and
   installer roots passed the same installed-consumer check. The Edgebox
   rootfs smoke passed every assertion, and the direct nex-systemd QEMU test
   printed `ASSERT-BOOT-PASS`. Commit: `pkg: layer extended-attribute copy
   policy`.

27. `pkg/libs/system/fuse3.yaml`

   The old `conf` output declared `/etc/fuse.conf`. Its installed contents
   are only comments that describe the `user_allow_other` and `mount_max`
   choices and leave the helper's built-in behavior unchanged, so outcome 4
   applies. The package now installs the template as
   `/usr/share/doc/fuse3/examples/fuse.conf`; `fusermount3` continues to read
   a real administrator file from `/etc/fuse.conf` when one exists.

   The strict package command built twice with checksum
   `9829fa1b95f94bc0a3185e52c6b618b5b1d5e9eb6a26b51edbdd0b5536c79047`.
   A checked-out desktop root ran `fusermount3 --version` and printed
   `3.17.4`; its help path also parsed successfully before returning its
   documented nonzero status without a mount point. A content assertion found
   no active line in the packaged template. Store inspection found the
   public documentation link and no `/etc/fuse.conf` or factory copy.

   All four affected images built twice and matched: desktop-vwl
   `6c2310500fbcaca12a87de05243fa16104e47a141c523478afa1f567c3a61923`,
   Nvidia 580
   `a716f757a68c58a725df0b37c164ad0eb5c7ea5d1d262a0fb21c9a01c62b829a`,
   Nvidia current
   `cb5a222913bf7a429d67bf609a05baca69af6f7ada2d51cd1265d94fb313b2e8`,
   and desktop-dev
   `c53305429c0200a5392834fb52230800c5d02af2a9c1b298b7edb7f2543035c3`.
   Commit: `pkg: move configuration samples to docs`.

28. `pkg/libs/text/vte.yaml`

   The `misc` output declares `/etc/profile.d/vte.csh` and
   `/etc/profile.d/vte.sh`. Outcome 5 applies. VTE does not read these files;
   the host's system profile sources them to add VTE terminal title, working
   directory, and shell prompt integration. The pinned Meson build installs
   both files to `vte_sysconfdir/profile.d`. Moving the package hooks alone
   would make a conventional profile that scans only `/etc/profile.d` miss
   them.

   The strict command built VTE twice with unchanged checksum
   `92157350d5c80cc7991d8166e6187e5e94f5219b4d971e7fc5345bc0ab7cbc27`.
   The packaged interactive Bash sourced `vte.sh` with a supported terminal
   and VTE version, created `__vte_osc7`, added OSC 133 markers to `PS1`, and
   printed `vte-shell-hook-pass`. The packaged `vte-urlencode-cwd` helper also
   encoded the current directory successfully. No assembly selects VTE
   directly, so this audit changed no finished system. Commit: `pkg: document
   system shell integration paths`.

29. `pkg/libs/tui/slang.yaml`

   The old `conf` output declared `/etc/slsh.rc`. Slsh loads this system
   startup file before the optional per-user file. Outcome 2 applies. The
   generic patch with SHA-256
   `9c95e4314ad6f2c7a284a3d11e431fa9173b8e55a6029feae6404ba4dd2a1a7a`
   retains `SLSH_CONF_DIR` and `SLSH_LIB_DIR` as explicit single-directory
   overrides; without one, Slsh selects the first `slsh.rc` in its configured
   administrator directory, `/run`, or `/usr/lib`. The packaged file now
   lives only at `/usr/lib/slsh.rc`, and an empty selected file masks lower
   startup files.

   The strict package command built twice with checksum
   `9dbe12cf16e3dbdf85decc1ba685c5f27dfd8afa7924f02cd10e64947d5cc1be`.
   Both builds ran the installed Slsh and proved vendor, transient,
   administrator, empty-mask, explicit-environment, and no-file behavior.
   Store inspection found the complete installed startup file below
   `/usr/lib` and no old `/etc/slsh.rc` output. No assembly selects Slsh's
   `conf` or `bin` output; Newt and NetworkManager use only its library bundle,
   whose source was unchanged. Commit: `pkg: layer slsh startup files`.

30. `pkg/net/firewall/iptables.yaml`

   The old `conf` output declared `/etc/ethertypes`, and both name and number
   readers opened only the compiled `XT_PATH_ETHERTYPES`. Outcome 2 applies.
   The generic patch with SHA-256
   `b5514bde8c6f49f3c377684f0628aca0344262177d20ac191e0e95cf4f22c187`
   selects `/etc/ethertypes`, `/run/ethertypes`, then
   `/usr/lib/ethertypes` as whole files. A non-ENOENT open error stops the
   search, and an empty selected file masks lower databases. The package now
   installs the complete upstream database only at `/usr/lib/ethertypes`.

   The strict package command built twice with checksum
   `f5382ebd5cd04f3e472ac57ee95124b1f672b4e84f90ced55c7d4171be882417`.
   Both builds ran the installed `ebtables-translate` with its packaged
   libraries and extensions, then proved vendor, transient, administrator,
   empty-mask, and restored-vendor behavior. Store inspection returned the
   complete upstream database with SHA-256
   `ed38f9d644befc87eb41a8649c310073240d9a8cd75b2f9c115b5d9d7e5d033c`
   and no `/etc/ethertypes` output. Docker is the only manifest that declares
   the full bundle, so its strict rebuild received the new vendor file and
   reproduced checksum
   `6f82e2a82020fb2a8701a3ca2818fc4a497ba5c619d3e26600926caa5e84ecdb`.
   Docker's published outputs need only Iptables binaries and libraries, so
   they do not export the sibling database to Edgebox. No current assembly
   selects Iptables' `conf` output or ships `ebtables-translate`; the package
   behavior test therefore remains the concrete consumer proof. Commit:
   `pkg: layer iptables ethertype data`.

31. `pkg/net/firewall/nftables.yaml`

   The old `conf` output declared `/etc/nftables/osf/pf.os`, and the OS
   fingerprint loader opened only that compiled path. Outcome 2 applies. The
   generic patch with SHA-256
   `1f77d1396bfe3b0bcfd1910d9e341cc42617480f4e04dba0f510f9615637a13f`
   selects `/etc/nftables/osf/pf.os`, then `/run/nftables/osf/pf.os`, then
   `/usr/lib/nftables/osf/pf.os` as whole files. A non-ENOENT open error stops
   the search, and an empty selected file masks lower databases. The package
   now installs the complete upstream database only below `/usr/lib`.

   The strict package command built twice with checksum
   `ed4cd32130e0dc86cc2aedc09d7a455d3646f312770e401f399c0f9153e701c0`.
   Both builds ran the installed `nft --debug mnl --check` in a private
   network namespace and proved vendor, transient, administrator, empty-mask,
   and restored-vendor behavior. Store inspection returned the complete
   upstream database with SHA-256
   `2e49e6bd24a07b7691937c5683cfdb15dbb1b7ce7c5a37865ad28d71ea2b6ed5`
   and no `/etc/nftables/osf/pf.os` output. Edgebox selects Nftables' full
   bundle directly. Its strict rebuild reproduced checksum
   `8c29d7b5685704b5bbba57d3243fb6df5ac39413029e1613a8e6f229933e89f5`;
   the finished-root smoke ran the installed `nft` against an `osf` rule in a
   private network namespace and checked that it opened and loaded the vendor
   file. Commit: `pkg: layer nftables fingerprint data`.

## Interfaces and Dependencies

This plan may change the 31 package manifests listed above, local patch files
beside those manifests, and assembly manifests or overlays that must own host
state. It does not add a Nex-only package configuration API. Programs keep
their normal command-line and environment interfaces.

The package build interface is `./nex build <manifest> --verbose --single
--check --update-checksum --force --compute-deps --record-profile
--generate-outputs`. The assembly build interface is `./nex build <assembly>
--single --check --update-checksum --verbose`. Both use
`/home/wegel/work/perso/zub/target/debug/zub` through `ZUB_BIN`.

Any new local source patch must be a tracked `file:` source with a lowercase
SHA-256, a complete patch header, explicit application order, and
`patch --batch --fuzz=0`. Online patches may remain URLs only when their host
should retain them for at least as long as the pinned target archive.
