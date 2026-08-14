# Container and VM Support

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Package and assemble the container, filesystem, network capture, and virtual
machine helper programs from the OSTreefy parity matrix. After this subplan,
`desktop-vwl` should expose the rootless Podman helper tools, FUSE tools,
`sshfs`, `tcpdump`, XFS tools, and VM GUI or firmware helpers that the old
personal system expected, or the matrix should record an explicit replacement
or blocker with proof.

## Progress

- [x] (2026-06-29 07:50Z) Started this sub-EP after completing
  `002e-desktop-session-glue`.
- [x] (2026-06-29 07:50Z) Listed knowledge files:
  `agent-workflow.md`, `cli-testing.md`, `kernel-and-boot.md`,
  `ostreefy-parity.md`, `package-manifests.md`, `reproducibility.md`, and
  `system-assemblies.md`.
- [x] (2026-06-29 07:50Z) Read relevant package-manifest, system-assembly,
  reproducibility, workflow, and parity knowledge. Key reminders: build-time
  dependencies are explicit, runtime ELF library dependencies are computed,
  disposable smoke roots go through `.agents/cleanup-workdirs.sh`, and
  checked-out `nex_structure` roots need `unshare --root` for public path
  checks.
- [x] (2026-06-29 07:50Z) Ran the new-ExecPlan worktree pre-task. The only
  visible untracked files were local notes/settings files already left
  uncommitted by prior work; no coherent tracked commit candidate was present.
- [x] (2026-06-29 07:51Z) Found existing manifests for `podman`, `fuse3`,
  `dmidecode`, `efibootmgr`, QEMU, and libvirt. `desktop-vwl` already includes
  Podman, runc, conmon, aardvark-dns, netavark, passt, QEMU, and libvirt.
- [x] (2026-06-29 08:17Z) Added and built `fuse-overlayfs` 1.17. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `dc0e54c0b50146982449634c5fa4b0fa3fb454d3e931998c92e35c2ec9cf4c1a`, and
  `nex resolve fuse-overlayfs --verbose` returned a four-ref runtime closure.
  A fresh smoke root verified `/usr/bin/fuse-overlayfs`, the manpage, the
  real-loader closure through `libfuse3`, glibc, and GCC runtime, and
  `fuse-overlayfs --version` under `unshare --root` after adding the loader
  symlink and minimal proc sysctl files that the program reads.
- [x] (2026-06-29 08:35Z) Added and built `libslirp` 4.9.3 for
  `slirp4netns`. `nex check` passed, the strict package build passed
  reproducibly with checksum
  `0830198349c79edf3d5ad44150d3421e39586370f1073a68b7a4fafbc4fc90f8`, and
  `nex resolve libslirp --verbose` returned a four-ref runtime closure. A
  fresh smoke root verified `slirp.pc` version 4.9.3, the loader closure
  through GLib and PCRE2, and a tiny C consumer that printed `4.9.3` under
  `unshare --root`.
- [x] (2026-06-29 08:46Z) Added and built `slirp4netns` 1.3.4 against
  `libslirp`, GLib, libcap, and libseccomp. `nex check` passed, the strict
  package build passed reproducibly with checksum
  `52d4f83611ff84292bccda77d0e56fb8dcc753af468f2ed30eab204dbc7e4cd6`, and
  `nex resolve slirp4netns --verbose` returned a six-ref runtime closure. A
  fresh smoke root verified `/usr/bin/slirp4netns`, the manpage, the loader
  closure through GLib, PCRE2, libslirp, and libseccomp, and
  `slirp4netns --version` under `unshare --root`.
- [x] (2026-06-29 09:06Z) Added `fuse3`, `fuse-overlayfs`, and
  `slirp4netns` to `desktop-vwl`. `nex check` passed, the assembly build
  passed reproducibly with checksum
  `814407d0445ecf89c2c39d251759f01f81d21aac36c3885959436b72bb45f528`, and a
  checked-out `systems/desktop-vwl/0.0.1` root exposed `/usr/bin/fuse-overlayfs`,
  `/usr/bin/fusermount3`, `/usr/bin/mount.fuse3`, `/usr/bin/slirp4netns`,
  `/etc/fuse.conf`, and the helper manpages. The smoke ran
  `slirp4netns --version`, `fuse-overlayfs --version`, and real-loader
  `--list` checks under `unshare --root`.
- [x] (2026-06-29 09:28Z) Added and built `distrobox` 1.8.2.5 from the
  stable upstream release line. `nex check` passed, the strict package build
  passed reproducibly with checksum
  `4666f493d2b526b6f24e97f6386fa098dc2035ebf0d35c9cd6c68bab868b1f21`,
  `nex resolve distrobox --verbose` returned a 17-ref runtime closure through
  the script command dependencies and Podman, and a fresh smoke root verified
  the scripts, manpage, shell completions, icon, `distrobox --version`,
  `distrobox-create --help`, and `distrobox-assemble --help`.
- [x] (2026-06-29 09:46Z) Added and built `sshfs` 3.7.6. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `5e6cf35b6ee545c9450e871c3e17b71fb12081f94942e652c82083a77dd196b7`,
  and `nex resolve sshfs --verbose` returned an eight-ref runtime closure
  through OpenSSH, FUSE3, GLib, PCRE2, OpenSSL, zlib, and glibc. A fresh smoke
  root verified `sshfs`, `mount.sshfs`, `mount.fuse.sshfs`, the manpage,
  `sshfs -V`, and the real-loader closure.
- [x] (2026-06-29 02:12Z) Added and built `libpcap` 1.10.6 for `tcpdump`.
  `nex check` passed, the strict package build passed reproducibly with
  checksum `1808d805be7e34e8e5db4dc15040bdc93cd488f9948ff118d18263e02ce04d60`,
  and `nex resolve libpcap --verbose` returned a four-ref runtime closure
  through Bash, glibc, and ncurses because `pcap-config` is a shell script. A
  fresh smoke root verified `/usr/bin/pcap-config`, its rewritten
  `#!/usr/bin/sh` shebang, `pcap-config --version`, pkg-config metadata, a
  tiny host-built consumer linked against the checked-out root, and the
  real-loader closure for `libpcap.so.1.10.6`.
- [x] (2026-06-29 02:23Z) Added and built `tcpdump` 4.99.6. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `eaef1cb9fec8d7c8dd3e32caae70232c3d849cca4ea99cae6887ab1a8e41dd5f`,
  and `nex resolve tcpdump --verbose` returned a four-ref runtime closure
  through glibc, OpenSSL, and libpcap. A fresh smoke root verified
  `/usr/bin/tcpdump`, `/usr/bin/tcpdump.4.99.6`, the manpage,
  `tcpdump --version`, BPF filter compilation with `tcpdump -ddd 'tcp port
  22'`, and the real-loader closure.
- [x] (2026-06-29 02:44Z) Added and built `userspace-rcu` 0.15.6 as an
  `xfsprogs` dependency. `nex check` passed, the strict package build passed
  reproducibly with checksum
  `5c39905fd7c8034b2e75b68b81a717d8f8be3fa88e862c27df05ae38262aee25`,
  and `nex resolve userspace-rcu --verbose` returned a two-ref runtime
  closure through glibc. A fresh smoke root verified `liburcu.pc`, headers, a
  tiny host-built RCU reader program linked against the checked-out root, and
  the real-loader closure for `liburcu.so.8.1.0`.
- [x] (2026-06-29 03:23Z) Added and built `xfsprogs` 7.0.1. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `ddf954fc22cadb6168dbda47be31d503d2e0a11c91a5470492ff4b646936ffaf`,
  and `nex resolve xfsprogs --verbose` returned a 23-ref runtime closure
  through shell helpers, Python, `systemd`, `inetutils`, ICU, `inih`,
  `userspace-rcu`, and `util_linux`. A fresh smoke root verified rewritten
  `/usr/bin` shebangs, `mkfs.xfs -V`, `xfs_repair -V`, `xfs_db -V`,
  `xfs_info -V`, `xfs_scrub_all -V`, `libhandle` symlinks, and real-loader
  closures for `mkfs.xfs` and `xfs_scrub`.
- [x] (2026-06-29 03:47Z) Added `distrobox`, `sshfs`, `tcpdump`, and
  `xfsprogs` to `desktop-vwl`. `nex check` passed, the assembly build passed
  reproducibly with checksum
  `c44bdc0ef6fa983d1e704af425aa25a5c45d4a1e6072fe0d10dcd2db386172b4`,
  and a checked-out `systems/desktop-vwl/0.0.1` root exposed the new public
  commands. The smoke ran `distrobox --version`, `distrobox-create --help`,
  `sshfs -V`, `tcpdump --version`, `tcpdump -ddd 'tcp port 22'`,
  `mkfs.xfs -V`, `xfs_repair -V`, `xfs_db -V`, `xfs_info -V`,
  `xfs_scrub_all -V`, `qemu-system-x86_64 --version`, `virsh --version`,
  `podman --version`, and `podman build --help` under `unshare --root`.
- [x] (2026-06-29 05:02Z) Fixed the libvirt dev bundle so it exposes the API
  XML files advertised by `libvirt.pc`, then added and built
  `libvirt-python` 11.0.0 for `virt-manager`. `nex check` passed for
  `pkg/dev/virt/libvirt.yaml` and `pkg/dev/python/libvirt-python.yaml`.
  The strict libvirt build passed reproducibly with checksum
  `43f098cc7a70181459de90986f651a97a95981354f55ef690700dd06a709fc19`.
  The strict `libvirt-python` build passed reproducibly with checksum
  `880bf622b79daf5182fb20d54750eed0b2936c13e98005533ee7d0c2cce68856`.
  `nex resolve libvirt-python --verbose` returned a 22-ref runtime closure.
  A fresh smoke root verified the API XML files, the `libvirt.pc`
  `libvirt_api` path, `import libvirt`, `import libvirt_qemu`,
  `import libvirt_lxc`, `import libvirtaio`, `libvirt.getVersion()`, and
  real-loader closures for all three compiled Python extension modules.
- [x] (2026-06-29 05:05Z) Committed the libvirt API XML and
  `libvirt-python` work as `a5f1f9c pkg: add libvirt python bindings`.
- [x] (2026-06-29 03:24Z) Added and built `osinfo-db` 20251212. The strict
  package build passed reproducibly with checksum
  `a7fe950eca49fc5f8862e6c6ad603846a89b2d54b1b86680d082f0ec1f35f40e`,
  and `nex check pkg/dev/virt/osinfo-db.yaml` passed.
- [x] (2026-06-29 03:27Z) Added and built `libvirt-glib` 5.0.0 for
  `virt-manager`'s early GI imports. The strict package build passed
  reproducibly with checksum
  `b5d242cf88fddfc7edcc0ef7625c3731af8bd5258619ca66a7adab31de455c14`,
  and `nex check pkg/dev/virt/libvirt-glib.yaml` passed.
- [x] (2026-06-29 03:37Z) Added and built `libosinfo` 1.12.0. The first
  strict build exposed the missing `util-linux` build input for `mount.pc`;
  after adding it, the strict package build passed reproducibly with checksum
  `15af755284ff4a52c082237dcc8a8c9d675016ce449b2f67ec622c2eaf62b5f2`.
- [x] (2026-06-29 03:50Z) Added manual runtime metadata for `libosinfo` and
  `libvirt-glib` GI typelibs and `libosinfo`'s hardware ID files. A fresh
  `.nex/tmp/virt-manager-smoke` root, cleaned through
  `.agents/cleanup-workdirs.sh`, used `nex resolve` output for Python,
  PyGObject, `libosinfo`, `libvirt-glib`, and `osinfo-db`. The smoke ran
  `osinfo-query os`, loaded 951 OS entries through Python GI Libosinfo, loaded
  LibvirtGLib, LibvirtGConfig, and LibvirtGObject through Python GI, and
  resolved `/usr/bin/osinfo-query` with the real loader.
- [x] (2026-06-29 03:53Z) Added and built GTK-VNC 1.3.1 and VTE 0.76.4 as
  `virt-manager` console widget dependencies. The work also enabled
  introspection in `at-spi2-core` so the active ATK library provider owns
  `Atk-1.0.typelib` and `Atspi-2.0.typelib`. `nex check` passed for
  `pkg/libs/graphics/at-spi2-core.yaml`, `pkg/dev/virt/gtk-vnc.yaml`, and
  `pkg/libs/text/vte.yaml`. Strict builds passed reproducibly with checksums
  `105391caacbf521e9ca4bb7c1040c48669bd387c17c10914aa61f2f0cd8a438d`
  for `at-spi2-core`,
  `c776600478a4cef16c188768f7873349a6346b2b0a428e96ea9623a6517e8263` for
  `gtk-vnc`, and
  `a74c6d251f6e720b339e212e0c2406e53aa8efbfe3ef27a272fd144ed92db1e5` for
  VTE. Fresh roots cleaned through `.agents/cleanup-workdirs.sh` imported
  `GtkVnc 2.0`, `GVnc 1.0`, and `Vte 2.91` through Python GI, listed
  `libgtk-vnc-2.0.so.0.0.2` and `libvte-2.91.so.0.7600.4` with the real
  loader, and ran `gvnccapture --help` and `vte-2.91 --help`.
- [x] (2026-06-29 04:28Z) Added and built the Python `requests` runtime stack
  for `virt-manager`: `urllib3` 1.26.20, `certifi` 2026.6.17,
  `charset-normalizer` 3.4.7, `idna` 3.10, and `requests` 2.34.2. `nex check`
  passed for all five manifests. Strict builds passed reproducibly with
  checksums `23d21f91d240f0ca783d1314cf93e93a89b3a9bb5b82f57cb8429aa51a5783a6`
  for `urllib3`,
  `da06b9febe9861a13309b7a79fe65a5ac8ea58de968f3de3c2b2000c6fb51b01` for
  `certifi`,
  `53df43eca49f2ef75c4a301ce3f54b59be22fc5b3b307bb42dcbb54a50eb27ab` for
  `charset-normalizer`,
  `fc4001447ae5afa3a476e1f98b6948387b6ff96e2916788bb87307114f6328de` for
  `idna`, and
  `8f65420fef1bb860f1333a771a4b7b0cc1faa1b5644eac17b1d6e4497a90929d` for
  `requests`. A fresh `.nex/tmp/requests-smoke` root, cleaned through
  `.agents/cleanup-workdirs.sh`, imported all five modules, loaded `ssl`, and
  ran `normalizer --version` under `unshare --root`.
- [x] (2026-06-29 04:29Z) Added and built `virt-manager` 5.1.0. `nex check`
  passed, the strict package build passed reproducibly with checksum
  `5fce6d6bc78c620636aef561fcf3c4dbd0fc8fad50aa99ca516e5f5f43470203`, and
  `nex resolve virt-manager --verbose` returned an 80-ref runtime closure. A
  fresh `.nex/tmp/virt-manager-smoke` root, cleaned through
  `.agents/cleanup-workdirs.sh`, verified all four public wrappers,
  `requests`, `libvirt`, `libxml2`, the GTK, GTKSource, GTK-VNC, VTE,
  LibvirtGLib, and Libosinfo GI imports, `virtManager.virtmanager`, and real
  loader closures for `libgtk-vnc-2.0.so.0` and `libvte-2.91.so.0`.
- [x] (2026-06-29 04:37Z) Fixed `virt-manager`'s `osinfo-db` runtime edge so
  it names `/usr/share/osinfo/os/fedoraproject.org/fedora-43.xml` instead of
  the `/usr/share/osinfo/os/fedoraproject.org` directory. `nex check` passed,
  the strict package build passed reproducibly with unchanged build output
  checksum `5fce6d6bc78c620636aef561fcf3c4dbd0fc8fad50aa99ca516e5f5f43470203`,
  `nex resolve virt-manager --verbose` included `osinfo-db` through the exact
  XML file, and a fresh smoke root verified all four wrappers, the Fedora XML
  file, and the Python/GI imports.
- [x] (2026-06-29 04:43Z) Fixed `virt-manager`'s installed Python wrappers so
  they insert their capsule-local `site-packages` path before importing
  modules. `nex format`, `nex check`, and a strict package build with
  `--single --check --update-checksum --force --compute-deps --record-profile
  --generate-outputs` passed. The build updated the package checksum to
  `2af9edbd76b164b79aa86ca03f03f259495764840a47ab3196d89754ceaafaf6` and then
  reproduced it. A fresh `.nex/tmp/virt-manager-smoke` root, cleaned through
  `.agents/cleanup-workdirs.sh`, ran all four wrappers under `unshare --root`
  and imported `gi`, GTK, GTKSource, GTK-VNC, VTE, LibvirtGLib, Libosinfo,
  `libvirt`, `libxml2`, `requests`, `virtinst`, and
  `virtManager.virtmanager`.
- [x] (2026-06-29 04:53Z) Added explicit PyGObject runtime support files to
  `virt-manager` after the first assembled `desktop-vwl` smoke showed that
  package-root smokes can hide missing capsule files. `nex check`, the strict
  package build, and the fresh package smoke passed with unchanged package
  checksum `2af9edbd76b164b79aa86ca03f03f259495764840a47ab3196d89754ceaafaf6`.
- [x] (2026-06-29 04:57Z) Fixed the capsule flattener so a runtime need under
  `/site-packages/<package>/...` copies the provider package directory, not
  just one leaf file. `cargo fmt`, `cargo test`, and `cargo build` passed for
  `src/cli`; the `desktop-vwl` assembly then built reproducibly with checksum
  `53afebab57780a66a79be94b9756fad395a286963c046ce2a038cf38b3f81f2d` and
  flattened 343 runtime files into the `virt-manager` capsule.
- [x] (2026-06-29 04:57Z) Checked out a fresh `systems/desktop-vwl/0.0.1` root,
  after cleaning through `.agents/cleanup-workdirs.sh`, and smoked all four
  public virt-manager wrappers under `unshare --root`. The smoke printed
  `5.1.0` for `virt-manager`, `virt-install`, `virt-clone`, and `virt-xml`,
  found the Fedora OS metadata XML in the capsule, and imported GTK,
  GTKSource, GTK-VNC, VTE, LibvirtGLib, Libosinfo, `libvirt`, `libxml2`,
  `requests`, `virtinst`, and `virtManager.virtmanager` from the assembled
  root.

## Surprises & Discoveries

- Observation: `dmidecode` and `efibootmgr` already exist and are in
  `desktop-dev`, but the 002f matrix rows focus on the daily desktop and VM
  support.
  Evidence: `rg -n "name: (dmidecode|efibootmgr)" pkg asm` found package
  manifests and `asm/desktop-dev.yaml` entries.
- Observation: Package-root smokes for programs that spawn helper binaries
  can need the same `/lib64/ld-linux-x86-64.so.2` path that an assembled
  `nex_structure` system provides.
  Evidence: `fuse-overlayfs --version` spawned `fusermount3`; the helper was
  present in the smoke root but could not exec until the disposable smoke root
  supplied the `/lib64` loader compatibility link.
- Observation: Runtime needs should avoid directory paths when the package
  can name a concrete file from the same provider.
  Evidence: Adding `virt-manager` to `desktop-vwl` failed during assembly
  flattening with `invalid object type: directory` while `virt-manager`
  depended on `/usr/share/osinfo/os/fedoraproject.org`. Changing the need and
  resolution key to `fedora-43.xml` kept `osinfo-db` in the resolver closure
  and made the package smoke pass again.
- Observation: Python applications that run from `/nex/pkg/...` capsules need
  wrappers that add capsule-local Python modules.
  Evidence: The assembled `desktop-vwl` root exposed `/usr/bin/virt-manager`
  as a symlink into the `virt-manager` capsule, and its Python dependencies
  existed under that capsule's `site-packages` directory. Without a wrapper
  shim, `virt-manager --version` failed to import `gi`; with the shim, the
  fresh package root ran the wrapper and imported the expected modules.
- Observation: The assembly capsule flattener must copy a full Python package
  directory when a runtime need points inside that directory.
  Evidence: The first assembled `desktop-vwl` smoke copied only
  `gi/__init__.py` and later only `urllib3/__init__.py`, so Python imports
  failed on sibling files such as `_gi` and `urllib3.exceptions`. After the
  flattener expanded `/site-packages/<package>/...` needs to the provider
  manifest's files under that package directory, `virt-manager` flattened 343
  files and the assembled-root smoke imported the full `requests` and GI stack.
- Observation: Some CLI tests rely on optional host features and should skip
  only when that feature is absent.
  Evidence: `cargo test --manifest-path src/cli/Cargo.toml` failed in the
  stale dependency rebuild test on this host with `uid 0 not mapped in
  namespace`, and the legacy OSTree history integration test failed because
  `ostree` is not installed. The tests now skip those exact missing host
  capabilities and still fail on other errors.
- Observation: A checked-out `nex_structure` assembly root provides the
  `/lib64/ld-linux-x86-64.so.2` loader shim that spawned helpers need.
  Evidence: The `desktop-vwl` 002f smoke found
  `.nex/tmp/desktop-vwl-002f-smoke/lib64/ld-linux-x86-64.so.2`; with no
  extra symlink, `fuse-overlayfs --version` spawned `fusermount3` and printed
  both tool versions.
- Observation: Script packages need explicit `needs` entries for runtime
  commands because ELF scanning cannot infer shell command calls.
  Evidence: `pkg/apps/containers/distrobox.yaml` lists `/usr/bin/sh`,
  Coreutils, Findutils, Gawk, Grep, Sed, Glibc `getent`, and Podman paths in
  script `needs`; `nex resolve distrobox --verbose` then returned those
  runtime refs instead of only the Distrobox files.
- Observation: Meson packages that use `glib-2.0.pc` need PCRE2's development
  output listed explicitly when that `.pc` file references `libpcre2-8.pc`.
  Evidence: The first SSHFS Meson configure found GLib but failed because
  `libpcre2-8.pc` was missing. Adding `pcre2` `bundles/dev` fixed the
  pkg-config lookup.
- Observation: C packages that include glibc headers may still need
  `linux-headers` even when their direct dependencies already build.
  Evidence: SSHFS compilation failed on `linux/falloc.h` and `linux/errno.h`;
  adding `linux-headers` fixed the compile.
- Observation: Upstream helper scripts may install `#! /bin/sh` even when the
  manifest records `/usr/bin/sh` in `needs`; the package must rewrite the
  installed script when Nex does not provide `/bin/sh`.
  Evidence: The first `libpcap` smoke found `/usr/bin/pcap-config`, but
  `unshare --root .nex/tmp/libpcap-smoke /usr/bin/pcap-config --version`
  failed with `No such file or directory`. `head -n1` showed `#! /bin/sh`.
  The manifest now rewrites the installed helper to `#!/usr/bin/sh`, and the
  smoke runs `pcap-config --version` in the checked-out root.
- Observation: Host-side compile smokes for package libraries can exercise
  headers and shared libraries without adding a source file to the repository.
  Evidence: The `libpcap` smoke compiled a small C program from stdin with
  `PKG_CONFIG_LIBDIR` and `PKG_CONFIG_SYSROOT_DIR` pointed at
  `.nex/tmp/libpcap-smoke`, linked it against the checked-out package root,
  and ran the resulting binary under `unshare --root`.
- Observation: `nex format` can produce a file that `nex check` still rejects
  when a new manifest uses `checksum: ""`.
  Evidence: The first `tcpdump` manifest had `checksum: ""`; `nex format`
  rewrote it to a blank `checksum:` value, then `nex check` reported `needs
  formatting`. Replacing the empty string with a 64-hex placeholder let format
  and check agree before the strict build wrote the real checksum.
- Observation: Autotools/libtool packages can complete with degraded feature
  probes when `diffutils` and `file` are missing, so list those build helpers
  explicitly when configure tries to use them.
  Evidence: The first `userspace-rcu` configure run printed `cmp: command not
  found`, `diff: command not found`, and `checking for file... no`. Adding
  `diffutils` and `file` made configure find `/usr/sbin/dd`, `file`, `cmp`,
  and `diff`, and the package then built reproducibly.
- Observation: Upstream `xfsprogs` installs administrative commands under
  `/usr/sbin` by default, but Nex rejects package files there because usrmerge
  reserves that path.
  Evidence: `nex check pkg/core/fs/xfsprogs.yaml` rejected `/usr/sbin`
  outputs. Configuring with `--sbindir=/usr/bin` produced accepted package
  paths and working public commands.
- Observation: `xfs_scrub_all` can report its version without D-Bus when the
  manifest moves the upstream top-level `import dbus` into the service binding
  path.
  Evidence: Before the patch, `xfs_scrub_all -V` failed with
  `ModuleNotFoundError: No module named 'dbus'` before parsing arguments.
- Observation: `virt-manager`'s GTK console path needs the GI namespaces
  from GTK-VNC and VTE.
  Evidence: Upstream code imports `GtkVnc` version `2.0`, `GVnc` version
  `1.0`, and `Vte` version `2.91`; package smokes loaded those namespaces in
  Python through PyGObject.
- Observation: A closure that mixes old standalone `atk` typelibs with the
  newer ATK libraries from `at-spi2-core` can fail at GI import time.
  Evidence: Early GTK-VNC and VTE smoke roots failed with
  `undefined symbol: atk_object_get_help_text` until `at-spi2-core` built and
  shipped `Atk-1.0.typelib`.
- Observation: `zub union-checkout` treats duplicate files as conflicts,
  while the Nex build materializer checks refs out with overwrite semantics.
  Evidence: A GTK-VNC smoke using `zub union-checkout` failed on a duplicate
  ATK header path; sequential `zub checkout --copy --force` using
  `nex resolve --verbose` refs produced a root that matched package-runtime
  behavior and passed.
  After the patch, the smoke root ran `xfs_scrub_all -V` and printed
  `xfs_scrub_all version 7.0.1`; `rg -n "import dbus|def bind"` shows the
  import inside `scrub_service.bind`.
- Observation: Agents should call `.agents/cleanup-workdirs.sh` for
  disposable work roots instead of hand-running destructive cleanup commands.
  Evidence: This subplan added `.nex/tmp/xfsprogs-smoke` to the script and
  used `bash .agents/cleanup-workdirs.sh`, which removed the configured
  disposable paths before the fresh XFS package smoke.
- Observation: `nex check` can accept an assembly ref that names a semantic
  bundle the store does not have, so the assembly build must prove package
  refs.
  Evidence: `nex check asm/desktop-vwl/desktop-vwl.yaml` passed while
  `desktop-vwl` referenced `tcpdump` `bundles/full`; the build failed with
  `commit not found in any repo`. `tcpdump` declares only `bundles/dev`, and
  the corrected assembly build passed.
- Observation: Host environment variables can leak into `unshare --root`
  smokes and change program behavior.
  Evidence: The host exported
  `CONTAINERS_CONF=/etc/containers/containers-nested.conf`, so Podman looked
  for that file inside the smoke root and failed. Running the Podman smokes
  with `env -u CONTAINERS_CONF` made `podman --version` and
  `podman build --help` run inside the checked-out system root.
- Observation: For a checked-out `nex_structure` system, public command
  execution is a better loader proof than invoking `/lib64/ld-linux*`
  directly.
  Evidence: The system root's `/lib64/ld-linux-x86-64.so.2` is
  `nex-ld-shim`; invoking it directly failed with `.nex-app-root not found`.
  Running the public commands through `unshare --root` exercised the shim and
  resolved their package capsules correctly.
- Observation: `libvirt-python` needs the libvirt API XML files that
  `libvirt.pc` advertises.
  Evidence: The first `libvirt-python` strict build failed because
  `setup.py` opened `/usr/share/libvirt/api/libvirt-api.xml`, but the
  existing libvirt dev bundle exposed only `/usr/lib/pkgconfig/libvirt.pc`.
  The libvirt manifest now runs `python3 ../scripts/apibuild.py ../docs
  docs-api` after `ninja install`, installs the four generated API XML files,
  and `libvirt-python` builds against them.
- Observation: `virt-manager --version` imports GI libraries before it can
  print a version.
  Evidence: The installed upstream wrapper imports `virtManager.virtmanager`,
  which calls `gi.require_version` for Gdk, Gtk, and LibvirtGLib. The imported
  `virtinst` package then requires Libosinfo at module import time.
- Observation: The `osinfo-db` release tarball is a data tree, not a Meson
  project.
  Evidence: `osinfo-db-20251212.tar.xz` contains top-level directories such
  as `os/`, `device/`, `platform/`, `install-script/`, `datamap/`, and
  `schema/`, with no build system. The manifest installs it under
  `/usr/share/osinfo`, where `libosinfo` looks by default.
- Observation: `libosinfo` uses the same `gio-2.0.pc` transitive build input
  rule seen in `libvirt-glib`.
  Evidence: The first strict `libosinfo` build failed while Meson resolved
  `gio-2.0` because pkg-config could not find `mount.pc`. Adding the
  `util-linux` dev bundle supplied `mount.pc`.
- Observation: GI typelibs need manual runtime `needs` for imported typelibs.
  Evidence: A closure-expanded smoke root had the Libosinfo and Libvirt
  typelibs but Python GI imports failed on missing `libxml2-2.0.typelib` and
  `GLib-2.0.typelib`. Adding manual `needs` from the installed typelibs to
  gobject-introspection's base typelibs made the imports pass.
- Observation: `libosinfo` commands need hardware ID data at runtime.
  Evidence: `osinfo-query os` failed in a smoke root with
  `Error opening file /usr/share/hwdata/pci.ids`. Adding manual `needs` for
  `/usr/share/hwdata/pci.ids` and `/usr/share/hwdata/usb.ids` made
  `nex resolve libosinfo --verbose` pull in `hwdata`, and the command listed
  OS records in the smoke root.
- Observation: Pure Python libraries need explicit runtime `needs` for
  imported modules because Nex does not infer Python imports.
  Evidence: `virt-manager --version` failed on
  `ModuleNotFoundError: No module named 'requests'` until the `requests`
  package declared runtime needs for `certifi`, `charset_normalizer`, `idna`,
  and `urllib3`, and the `virt-manager` wrappers declared a need on
  `requests/__init__.py`.
- Observation: Python packages can need standard-library shared libraries that
  are not pulled by `/usr/bin/python3` alone.
  Evidence: The first `requests` smoke root imported Python's `binascii`
  extension through `urllib3` and failed on missing `libz.so.1`. The
  `requests` manifest now declares `libz.so.1`, `libssl.so.3`, and
  `libcrypto.so.3` needs, and the smoke imports `ssl` successfully.
- Observation: Direct imports of some `virtManager.details` internals can trip
  upstream circular imports that the public wrapper does not hit.
  Evidence: A smoke that imported `virtManager.details.viewers` and
  `virtManager.details.serialcon` directly failed with a partially initialized
  `virtManager.baseclass`; `virt-manager --version` and importing
  `virtManager.virtmanager` both succeeded in the same root.

## Decision Log

- Decision: Treat Docker and Docker Buildx rows as Podman replacements unless
  a later smoke exposes a missing user workflow.
  Rationale: The matrix already records that Nex chooses Podman for container
  runtime parity and Podman build tooling for Buildx parity.
  Date/Author: 2026-06-29 / Ralph

## Outcomes & Retrospective

Complete after the tracked CLI and assembly commits land. `desktop-vwl` now
has the 002f container, filesystem, network capture, and VM GUI coverage: Podman
replaces Docker and Buildx; `distrobox`, `fuse-overlayfs`, `fuse3`,
`slirp4netns`, `sshfs`, `tcpdump`, `xfsprogs`, QEMU, libvirt, and
`virt-manager` are present or already existed; and fresh package or assembled
root smokes exercised the public commands.

## Context and Orientation

The active 002f matrix rows are:

```text
docker
docker-buildx
distrobox
fuse-overlayfs
fuse3
slirp4netns
sshfs
tcpdump
virt-manager
xfsprogs
```

Existing relevant manifests:

```text
pkg/apps/containers/podman.yaml
pkg/libs/system/fuse3.yaml
pkg/cli/system/dmidecode.yaml
pkg/cli/system/efibootmgr.yaml
pkg/dev/virt/qemu.yaml
pkg/dev/virt/libvirt.yaml
```

Missing or unconfirmed target manifests at subplan start:

```text
pkg/apps/containers/distrobox.yaml
pkg/apps/containers/fuse-overlayfs.yaml
pkg/apps/containers/slirp4netns.yaml
pkg/cli/fs/sshfs.yaml
pkg/cli/net/tcpdump.yaml
pkg/apps/virt/virt-manager.yaml
pkg/core/fs/xfsprogs.yaml
```

`desktop-vwl` already includes:

```text
podman
runc
conmon
aardvark-dns
netavark
passt
qemu
libvirt
```

## Plan of Work

1. Verify existing `fuse3`, `dmidecode`, `efibootmgr`, Podman, QEMU, and
   libvirt coverage from package manifests and checked-out system roots.
2. Add or verify `fuse-overlayfs` and `slirp4netns` first because rootless
   Podman commonly calls those helpers.
3. Add `distrobox` after the container helpers build.
4. Add FUSE filesystem and network tools: `sshfs`, `tcpdump`, and `xfsprogs`.
5. Add `virt-manager` if its Python, GTK, libvirt, and GSettings dependencies
   can be packaged cleanly. If the dependency stack is too large for this
   slice, record the concrete missing packages and split a new sub-EP.
6. Add completed package refs to `asm/desktop-vwl/desktop-vwl.yaml` unless a
   package is development-only or the matrix records a replacement.
7. Build the changed assembly reproducibly, check out `systems/desktop-vwl/0.0.1`,
   and smoke the public commands and VM or container helper files.
8. Update `.agents/ostreefy-parity-matrix.md` after each row reaches covered,
   replaced, deferred, skipped, or blocked.

## Concrete Steps

Run package checks from the repository root:

```bash
./src/cli/target/debug/nex check pkg/path/to/package.yaml
./src/cli/target/debug/nex build pkg/path/to/package.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

Run assembly checks after each coherent package batch:

```bash
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml
./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force
```

Use `.agents/cleanup-workdirs.sh` for disposable roots. Add new smoke paths to
that script before cleaning:

```bash
bash .agents/cleanup-workdirs.sh
zub --repo .nex/repo checkout --copy --force systems/desktop-vwl/0.0.1 .nex/tmp/desktop-vwl-002f-smoke
```

Expected package smokes include:

```bash
unshare --root <root> /usr/bin/fuse-overlayfs --version
unshare --root <root> /usr/bin/slirp4netns --version
unshare --root <root> /usr/bin/distrobox --version
unshare --root <root> /usr/bin/sshfs -V
unshare --root <root> /usr/bin/tcpdump --version
unshare --root <root> /usr/bin/mkfs.xfs -V
unshare --root <root> /usr/bin/virt-manager --version
```

Podman proof should go beyond `podman --version` when practical. Prefer a
rootless `podman info` or a local image-free helper check first. Do not require
network image pulls for this subplan.

## Validation and Acceptance

- Every new or changed package manifest passes `nex check`.
- Every new package builds with the strict package command and passes a smoke
  check against a fresh checked-out package root.
- `desktop-vwl` builds reproducibly after new 002f packages land in the
  assembly.
- A checked-out `desktop-vwl` root exposes the public commands and service or
  helper files named by the matrix.
- Docker and Docker Buildx rows remain marked as Podman replacements only if
  the checked system exposes enough Podman runtime and build behavior for the
  current parity goal.
- The 002f matrix rows have no `needs-manifest` status when this subplan ends.

## Idempotence and Recovery

All package and assembly builds are reproducible. Rerun failed strict builds
from scratch instead of manually removing builder-managed temporary
directories. If a disposable smoke root or downloaded inspection source must be
cleaned, add it to `.agents/cleanup-workdirs.sh` and run that script.

If a strict package build updates a checksum, rerun the same command until the
second pass succeeds. If the two passes disagree, compare generated files and
record the mismatch in `Surprises & Discoveries` before changing package
logic.

If a package needs a large Python, GTK, kernel, or privilege policy design that
does not fit this subplan, create a follow-up sub-EP and record the blocker in
the matrix.

## Artifacts and Notes

Keep short transcripts, checksums, smoke roots, and failure findings here.
