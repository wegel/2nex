# Small Daily CLI Tools

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Package and assemble the small command-line tools from the OSTreefy parity
matrix that a daily desktop or development system should expose. The end state
is that the chosen tools build reproducibly, land in `desktop-vwl` or
`desktop-dev`, and pass command-level smoke checks from a built package output
or checked-out system tree.

## Progress

- [x] (2026-06-28 05:31Z) Created this sub-EP after 002b froze the parity
  matrix.
- [x] (2026-06-28 05:31Z) Confirmed the target list against
  `.agents/ostreefy-parity-matrix.md`.
- [x] (2026-06-28 05:45Z) Added and built `pkg/cli/json/jq.yaml`.
- [x] (2026-06-28 06:10Z) Added and built `pkg/cli/system/screen.yaml`.
- [x] (2026-06-28 05:57Z) Added `jq` to `asm/desktop-vwl/desktop-vwl.yaml`
  locally.
- [x] (2026-06-28 12:04Z) Committed the unrelated deployment-management dirty
  source-snapshot work as `1b92dc3`.
- [x] (2026-06-28 12:18Z) Added 002c runtime tools to `desktop-vwl`, added
  Python and Perl for script wrappers, built the assembly reproducibly, and
  smoked the added commands from a checked-out system tree.
- [x] (2026-06-28 12:23Z) Added 002c developer tools to `desktop-dev`, built
  the assembly reproducibly, and smoked the added commands from a checked-out
  system tree.
- [x] (2026-06-28 06:03Z) Updated the `jq` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 06:10Z) Updated the `screen` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 06:50Z) Fixed the manifest formatter so empty package
  `outputs` and `resolution` maps stay valid YAML maps.
- [x] (2026-06-28 07:05Z) Fixed chroot package build roots so explicit build
  dependencies are materialized with their runtime files before the build
  script starts.
- [x] (2026-06-28 07:10Z) Added and built `pkg/dev/tools/cloc.yaml`.
- [x] (2026-06-28 07:14Z) Updated the `cloc` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 07:30Z) Fixed output generation so `outputs: {}` can be
  replaced by generated output lists.
- [x] (2026-06-28 07:38Z) Added and built `pkg/cli/crypto/age.yaml`.
- [x] (2026-06-28 07:38Z) Updated the `age` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 07:50Z) Fixed Cargo vendoring for legacy Cargo.lock
  metadata checksum entries.
- [x] (2026-06-28 07:58Z) Added and built `pkg/dev/tools/tokei.yaml`.
- [x] (2026-06-28 07:58Z) Updated the `tokei` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 08:18Z) Added and built `pkg/cli/net/wget.yaml`.
- [x] (2026-06-28 08:18Z) Updated the `wget` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 08:31Z) Added and built `pkg/cli/system/btop.yaml`.
- [x] (2026-06-28 08:31Z) Updated the `btop` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 08:55Z) Fixed generated output metadata and compute-deps so
  Python script packages keep explicit interpreter needs.
- [x] (2026-06-28 08:58Z) Added and built `pkg/cli/system/dool.yaml`.
- [x] (2026-06-28 08:58Z) Updated the `dool` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 09:10Z) Added and built `pkg/cli/system/chezmoi.yaml`.
- [x] (2026-06-28 09:10Z) Updated the `chezmoi` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 09:25Z) Added and built `pkg/libs/data/libconfuse.yaml` as
  a `bmon` dependency.
- [x] (2026-06-28 09:35Z) Added and built `pkg/cli/system/bmon.yaml`.
- [x] (2026-06-28 09:35Z) Updated the `bmon` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 09:48Z) Added and built `pkg/libs/crypto/libmd.yaml` as
  an `openbsd-netcat` dependency.
- [x] (2026-06-28 09:55Z) Added and built `pkg/libs/system/libbsd.yaml` as
  an `openbsd-netcat` dependency.
- [x] (2026-06-28 10:06Z) Added and built
  `pkg/cli/net/openbsd-netcat.yaml`.
- [x] (2026-06-28 10:06Z) Updated the `openbsd-netcat` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 10:24Z) Added and built `pkg/dev/tools/taplo-cli.yaml`.
- [x] (2026-06-28 10:24Z) Updated the `taplo-cli` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 04:46Z) Added and built `pkg/dev/tools/ast-grep.yaml`.
- [x] (2026-06-28 04:46Z) Updated the `ast-grep` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 05:01Z) Added and built `pkg/cli/shells/fish.yaml`.
- [x] (2026-06-28 05:01Z) Updated the `fish` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 05:08Z) Added and built `pkg/cli/net/mosh.yaml`.
- [x] (2026-06-28 05:08Z) Updated the `mosh` row in
  `.agents/ostreefy-parity-matrix.md` to `needs-assembly`.
- [x] (2026-06-28 11:58Z) Skipped Nushell by human request after two
  reproducibility attempts failed.
- [x] (2026-06-28 11:58Z) Updated the `nushell` row in
  `.agents/ostreefy-parity-matrix.md` to `deferred-request`.
- [x] (2026-06-28 12:24Z) Completed the 002c package and assembly batch, with
  Nushell deferred by human request.
- [x] (2026-06-28 12:40Z) Promoted verified durable notes into
  `.agents/knowledge/package-manifests.md`,
  `.agents/knowledge/system-assemblies.md`,
  `.agents/knowledge/cli-testing.md`, and
  `.agents/knowledge/reproducibility.md`.

## Surprises & Discoveries

- Observation: The `./nex` wrapper can fail before a package build when
  `src/cli/vendor` is missing.
  Evidence: `./nex build pkg/cli/json/jq.yaml --verbose --single --check
  --update-checksum --force --compute-deps --record-profile
  --generate-outputs` failed while Cargo tried to read
  `src/cli/vendor`. Running the same build through
  `./src/cli/target/debug/nex` avoided the wrapper rebuild path.
- Observation: The jq release tarball builds without `bison`, `byacc`,
  `file`, `cmp`, or `diff` in the build root.
  Evidence: configure reported those tools missing, but the strict jq build
  completed and the second build produced the same output checksum.
- Observation: `desktop-vwl` source snapshots make assembly checksums sensitive
  to unrelated dirty files under `src/cli` and `scripts`.
  Evidence: the local assembly build that added jq packaged dev source
  tarballs from `pkg`, `asm`, `src/cli`, and `scripts`. This worktree still
  has unrelated uncommitted changes under `src/cli` and `scripts`, so the
  generated checksum `1055cc3119e65da2d5947067683e7a64495be56a397b135782418c580bd5b4fb`
  is not safe to commit as the jq assembly proof.
- Observation: GNU Screen 5.0.1 needs PAM in this repo's default build.
  Evidence: the first screen build failed at `checking for PAM support... no`.
  Adding `x86_64/pkg/libs/security/linux-pam/1.7.1/bundles/dev` made configure
  report `PAM support: yes`, and the strict build then passed reproducibility.
- Observation: `nex format` must render empty maps as maps, not as bare YAML
  keys.
  Evidence: manifests with `outputs: {}` or `resolution: {}` were reformatted
  into bare `outputs:` or `resolution:` keys, and `nex check` then rejected
  them because those sections parsed as null.
- Observation: Chroot package builds need runtime files for the build tools,
  not only the explicit tool outputs.
  Evidence: `cloc` first failed before the build script started because
  `/usr/bin/bash` had no loader in the build root. After only glibc was added,
  bash failed on `libncursesw.so.6`. The builder now uses the materializer in
  flat mode for chroot package build dependencies, so manifests can list build
  tools and the root also gets those tools' declared runtime files.
- Observation: `cloc`'s package script needs `sed` and `findutils` in addition
  to `bash`, `coreutils`, and `perl`.
  Evidence: after the builder root could start bash, the script failed first
  on missing `sed`, then on missing `find`.
- Observation: The output generator must be able to replace `outputs: {}`.
  Evidence: the first `age` build compiled and updated its profile, then
  `--generate-outputs` failed with `Unable to locate outputs section`.
  `manifest::update::tests::locates_flow_style_empty_outputs_block` now covers
  that case.
- Observation: `age` v1.3.x needs a newer Go toolchain than the repo currently
  packages.
  Evidence: upstream tags `v1.3.0` and `v1.3.1` declare `go 1.24.0` and
  `toolchain go1.25.5`, while `pkg/dev/lang/go.yaml` packages Go 1.23.8.
  `age` v1.2.1 declares `go 1.19` and built with the repo's Go package.
- Observation: Old Cargo.lock files store registry checksums under
  `[metadata]`, not inside each package entry.
  Evidence: `tokei` v9.1.1 initially made the Cargo vendor helper report
  `Found 0 packages in Cargo.lock`. After the helper read metadata keys such
  as `checksum aho-corasick 0.6.10
  (registry+https://github.com/rust-lang/crates.io-index)`, it found 129
  crates and produced a non-empty vendor tarball.
- Observation: `wget` 1.25.0 installs info docs and config but no man output
  with this manifest's configure flags.
  Evidence: the first strict build created `outputs/bin`, `outputs/conf`, and
  `outputs/info`, then failed bundle processing because the manifest still
  listed `man` in `bundles.dev`. Replacing `man` with `conf` and `info` let
  the strict build publish the dev bundle and pass reproducibility.
- Observation: `btop` v1.4.7 and v1.4.6 need C++ library support that the
  repo's GCC 13 does not provide.
  Evidence: v1.4.7 failed to compile on `std::ranges::to` with GCC 13.2.0,
  and a local source check found the same symbol in v1.4.6. v1.4.5 does not
  use that API and built with the repo compiler.
- Observation: Script packages can need manual runtime `needs` because ELF
  scanning cannot see interpreters.
  Evidence: `dool` is a Python script. Output generation and compute-deps now
  preserve an explicit `/usr/bin/python3` need on `/usr/bin/dool`, and the
  package smoke passed only after checking out Python and its runtime libraries
  into the temporary root.
- Observation: `chezmoi` v2.59.0 is the newest sampled release that fits the
  repo's Go 1.23.8 package.
  Evidence: recent `chezmoi` releases require Go 1.24 through 1.26. The
  v2.59.0 `go.mod` declares Go 1.23.4, and the strict Nex package build passed
  with `GOTOOLCHAIN=local`.
- Observation: `libconfuse` v3.3 builds with older Autotools expectations.
  Evidence: its generated configure script needed `sed` and `awk` in the build
  root. It also reported `--disable-static` and `--disable-nls` as unrecognized
  options, so the manifest uses `./configure --prefix=/usr` and includes the
  generated `static` and `locale` outputs.
- Observation: `bmon` smoke roots need the GCC runtime through a transitive
  library dependency.
  Evidence: `readelf -d` on `/usr/bin/bmon` listed ncurses, libconfuse, libnl,
  libm, and libc, but not `libgcc_s.so.1`. The first smoke root failed on
  missing `libgcc_s.so.1`; adding `gcc`'s `outputs/lib` made `bmon -V` pass.
- Observation: OpenBSD netcat uses Debian's Linux portability chain.
  Evidence: Debian `netcat-openbsd_1.238-1.dsc` declares `libbsd-dev` and
  `pkgconf` as build dependencies. Nex had no `libbsd` or `libmd` manifests,
  so this sub-EP started that chain with `libmd`.
- Observation: `libbsd` installs `libbsd.so` as a linker script that pulls in
  `libmd`.
  Evidence: the smoke program linked with `-lbsd`; `LD_LIBRARY_PATH` was needed
  during the host smoke so `ldd` resolved indirect `libmd.so.0` from the
  package checkout rather than from the host.
- Observation: Multi-source package scripts need absolute source paths after
  they change directory.
  Evidence: the first `openbsd-netcat` build failed when the script ran
  `tar -xf ${SOURCE_debian}` after `cd ${WORK_DIR}`. Using
  `/./${SOURCE_debian}` matched existing Go-vendor package scripts and fixed
  the source lookup.
- Observation: Taplo's default Rust TLS stack failed to link in this build
  root, but native TLS worked.
  Evidence: building Taplo 0.10.0 with default features failed with unresolved
  `ring_core_0_17_7_*` symbols from the `ring` crate. Building with
  `--no-default-features --features completions,lint,lsp,native-tls,toml-test`
  linked against packaged OpenSSL and passed reproducibility.
- Observation: `ast-grep` needs Linux headers and non-LTO tree-sitter C
  archives.
  Evidence: the first build failed while `tree-sitter` included
  `<linux/limits.h>` through glibc headers. After adding `linux-headers`, the
  build reached the final Rust link but failed with undefined `tree_sitter_*`
  and `ts_*` symbols even though the static archives contained those symbols.
  Removing `-flto=auto` from `CFLAGS`, `CXXFLAGS`, and `LDFLAGS` before Cargo
  builds the parser C archives made the strict reproducible build pass.
- Observation: fish 4.7.1 needs its vendored UTF-32 PCRE2 build in this repo.
  Evidence: fish 4.8.0 currently vendors a Codeberg git source that returned
  HTTP 503 during this work. Fish 4.7.1 vendors from GitHub, but the vendor
  helper copied the `rust-pcre2` workspace into both crate directories, so the
  build script replaces `vendor/pcre2-sys-0.2.9` with the nested subcrate.
  The repo pcre2 package disables PCRE2-32, so fish sets
  `-DFISH_USE_SYSTEM_PCRE2=OFF`. The vendored C archive also needed
  `-flto=auto` removed from C, C++, and link flags before the final Rust link
  could resolve `pcre2_*_32` symbols.
- Observation: mosh needs manual script runtime metadata.
  Evidence: the ELF scanner found native library needs for `mosh-client` and
  `mosh-server`, but `/usr/bin/mosh` is a Perl script. The manifest rewrites
  its shebang to `#!/usr/bin/perl` and records manual needs for
  `/usr/bin/perl` and `/usr/bin/ssh` so desktop assemblies pull in Perl and
  OpenSSH.
- Observation: Nushell 0.111.0 did not build reproducibly after basic Rust
  entropy controls.
  Evidence: The first attempt used Nushell's upstream thin-LTO release profile
  and produced output checksums
  `01cbbdd015a420e960fe96eea30ee8f663ef670e3e1519a7684c75d448f6b9d5` and
  `cc81bb152049a9c0c01e47cafa5fc6e8c0374923e303509ae0deef346c100219`.
  The second attempt disabled release LTO and produced
  `b0f2c3488a665322e4b30c8d192d6970b58559c613e9ae81de706e0125bc9246` and
  `d36a7491bcf58a4fe041949172627f9bb2f7ae9ff3f816f534bb0b00158cf3ad`.
  The final attempt also set `CARGO_BUILD_JOBS=1`, `CARGO_INCREMENTAL=0`,
  `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1`, and
  `SOURCE_DATE_EPOCH=1704067200`; it still produced
  `8bfeaf6916046cd37fb1e39cab08ecbbc848ef0052ab7acc434ef787780d02bc` and
  `e0e8214676aab6b34d5b3b4f295d6934dfdac8fcfcb430100b39de19c4efbe1b`.
  The human said to skip Nushell because they do not use it.
- Observation: `desktop-vwl` needed explicit Python and Perl runtime packages
  for script wrappers.
  Evidence: The first checked-out system smoke after adding `dool` and `mosh`
  failed because `/usr/bin/dool` could not find `python3` and `/usr/bin/mosh`
  could not execute without Perl. Adding `dev/lang/python3` and
  `dev/lang/perl` to `desktop-vwl` made the next reproducible assembly build
  and system smoke pass.
- Observation: `desktop-dev` keeps Shadow's `/usr/bin/sg` rather than
  ast-grep's alias.
  Evidence: The checked-out `desktop-dev` tree linked `/usr/bin/sg` to the
  Shadow package. The ast-grep package still provides `/usr/bin/ast-grep`, and
  the system smoke uses that command.

## Decision Log

- Decision: Treat shell and daily terminal tools in this sub-EP even when the
  main EP's initial target list did not spell out every row.
  Rationale: The frozen matrix assigns `fish`, `nushell`, and `dool` to 002c,
  and the human asked for program parity using Nex judgment rather than exact
  package-name parity.
  Date/Author: 2026-06-28 / Ralph

- Decision: Skip Nushell for the parity push.
  Rationale: The package is not used by the human, and three strict build
  attempts failed the reproducibility check after version selection, LTO
  changes, single-job Cargo, disabled incremental compilation, fixed codegen
  units, and fixed `SOURCE_DATE_EPOCH`.
  Date/Author: 2026-06-28 / Ralph

- Decision: Keep Shadow's `sg` in assembled systems and use `ast-grep` as the
  smoke command.
  Rationale: `sg` is a standard user/group command in the base system. Replacing
  it with ast-grep's short alias would be surprising and less compatible.
  Date/Author: 2026-06-28 / Ralph

## Outcomes & Retrospective

This sub-EP produced the package commits `7533670`, `cda083d`, `8873388`,
`9ed0703`, `9d3eed8`, `cf1a9da`, `876ff3b`, `ec65cce`, `2f0d72c`, `ead1c2d`,
`16be68a`, `4fe34ff`, `43d8217`, `9c01e7c`, `e774da1`, `bc9be81`, `4d46ec5`,
and `b6dc3ec`, plus supporting CLI fixes in `fbe5ec2`, `163600e`,
`95ebd63`, `db3e1b3`, `460bef8`, and `fae2b5c`. The final assembly commit is
`f30843e asm: include daily cli tools`.

The final `desktop-vwl` build passed reproducibility with checksum
`58cc1f613928f8bdb86983feaf75e061d3b58d34cc2c6526b308e34ade54be90`, and the
checked-out system smoke ran `fish`, `screen`, `btop`, `bmon`, `jq`, `age`,
`chezmoi`, `dool`, `wget`, `mosh`, and `nc`. The final `desktop-dev` build
passed reproducibility with checksum
`6bea1682e25038044197e9c5a95910fed5b3521cfb92f32d12fe65f8dd5288ec`, and the
checked-out system smoke ran `ast-grep`, `cloc`, `taplo`, and `tokei`.

Nushell remains deferred by human request. The temporary Nushell manifest was
removed, and `.agents/ostreefy-parity-matrix.md` records the row as
`deferred-request`.

`jq` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/json/jq.yaml
./src/cli/target/debug/nex build pkg/cli/json/jq.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-jq-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/json/jq/1.8.2/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/text/oniguruma/6.9.10/outputs/lib "$tmpdir"
LD_LIBRARY_PATH="$tmpdir/usr/lib" "$tmpdir/usr/bin/jq" -n '{ok: true, value: (1 + 2)}'
```

The smoke command printed `{ "ok": true, "value": 3 }`.

Local assembly smoke, not yet commit-ready:

```bash
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml
./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force
tmpdir=$(mktemp -d /tmp/nex-desktop-jq.XXXXXX)
zub --repo .nex/repo checkout --copy systems/desktop-vwl/0.0.1 "$tmpdir"
unshare --user --map-root-user --mount --fork --root="$tmpdir" /usr/bin/jq -n '{ok: true, value: (1 + 2)}'
```

The smoke command printed `{ "ok": true, "value": 3 }` inside the checked-out
system root. Do not use the generated checksum from that build for a commit
until the unrelated source-snapshot dirt is gone or the assembly is built in an
isolated clean worktree.

`screen` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/system/screen.yaml
./src/cli/target/debug/nex build pkg/cli/system/screen.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-screen-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/system/screen/5.0.1/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/security/linux-pam/1.7.1/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/ncurses/6.4-20230520/outputs/lib "$tmpdir"
LD_LIBRARY_PATH="$tmpdir/usr/lib" "$tmpdir/usr/bin/screen" --version
```

The smoke command printed `Screen version 5.0.1`.

CLI formatter and builder proof:

```bash
rustfmt --check --edition 2021 src/cli/src/manifest/format.rs src/cli/src/build/mod.rs src/cli/src/system/mod.rs
cargo test --manifest-path src/cli/Cargo.toml --bin nex -- --skip build::orchestration::tests::rebuilds_when_dependency_manifest_changes
cargo test --manifest-path src/cli/Cargo.toml --test blob_ref_tests -- --skip test_time_travel_history_search
```

The formatter test
`manifest::format::tests::formats_empty_output_and_resolution_maps_as_flow_maps`
ran in the CLI unit test suite. The build tests
`build::tests::classifies_build_dependency_materialize_requests` and
`build::tests::validates_chroot_build_root_launcher_files` also ran there.

`cloc` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/tools/cloc.yaml
./src/cli/target/debug/nex build pkg/dev/tools/cloc.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-cloc-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/dev/tools/cloc/2.08/files "$tmpdir"
"$tmpdir/usr/bin/cloc" --version
```

The smoke command printed `2.08`.

Output generator proof:

```bash
rustfmt --check --edition 2021 src/cli/src/manifest/update.rs
cargo test --manifest-path src/cli/Cargo.toml --bin nex -- --skip build::orchestration::tests::rebuilds_when_dependency_manifest_changes
cargo test --manifest-path src/cli/Cargo.toml --test blob_ref_tests -- --skip test_time_travel_history_search
```

The unit test
`manifest::update::tests::locates_flow_style_empty_outputs_block` ran in the
CLI suite.

`age` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/crypto/age.yaml
./src/cli/target/debug/nex build pkg/cli/crypto/age.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-age-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/crypto/age/1.2.1/files "$tmpdir"
"$tmpdir/usr/bin/age" --version
"$tmpdir/usr/bin/age-keygen" -version
```

The smoke commands printed `v1.2.1` and `(devel)`.

Cargo vendoring proof:

```bash
rustfmt --check --edition 2021 src/cli/src/cargo_vendor/mod.rs
cargo test --manifest-path src/cli/Cargo.toml --bin nex -- --skip build::orchestration::tests::rebuilds_when_dependency_manifest_changes
cargo test --manifest-path src/cli/Cargo.toml --test blob_ref_tests -- --skip test_time_travel_history_search
```

The unit test
`cargo_vendor::tests::parses_legacy_lock_metadata_checksums` ran in the CLI
suite.

`tokei` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/tools/tokei.yaml
./src/cli/target/debug/nex build pkg/dev/tools/tokei.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-tokei-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/dev/tools/tokei/9.1.1/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/toolchain/gcc/13.2.0/553fbd55598d795e833e38ffe4d3793a040d59f5c4c00ed950eff54d6695c78b/files "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/ce80d6a13e70d5b91f217da6af3d078d10c85d2a609e7dcd732b753e5ecc1cac/files "$tmpdir"
"$tmpdir/usr/bin/tokei" --version
```

The smoke command printed `tokei 9.1.1 compiled without serialization
formats.`

`wget` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/net/wget.yaml
./src/cli/target/debug/nex build pkg/cli/net/wget.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-wget-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/net/wget/1.25.0/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/libs/openssl3/3.3.1/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/zlib/1.3.1/outputs/lib "$tmpdir"
unshare --user --map-root-user --mount --root "$tmpdir" /usr/lib/ld-linux-x86-64.so.2 --library-path /usr/lib /usr/bin/wget --version
```

The smoke command printed `GNU Wget 1.25.0 built on linux-gnu.`

`btop` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/system/btop.yaml
./src/cli/target/debug/nex build pkg/cli/system/btop.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-btop-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/system/btop/1.4.5/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/toolchain/gcc/13.2.0/outputs/lib "$tmpdir"
unshare --user --map-root-user --mount --root "$tmpdir" /usr/lib/ld-linux-x86-64.so.2 --library-path /usr/lib /usr/bin/btop --version
```

The smoke command printed `btop version: 1.4.5`.

CLI script dependency proof:

```bash
rustfmt --check --edition 2021 src/cli/src/manifest/update.rs src/cli/src/commands/compute_deps.rs
cargo test --manifest-path src/cli/Cargo.toml --bin nex -- --skip build::orchestration::tests::rebuilds_when_dependency_manifest_changes
cargo test --manifest-path src/cli/Cargo.toml --test blob_ref_tests -- --skip test_time_travel_history_search
cargo build --manifest-path src/cli/Cargo.toml --bin nex
```

The unit tests
`manifest::update::tests::generated_outputs_preserve_existing_file_needs`,
`commands::compute_deps::tests::merge_needs_preserves_manual_script_dependencies`,
and `commands::compute_deps::tests::merge_needs_adds_discovered_libraries_without_duplicates`
ran in the CLI suite.

`dool` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/system/dool.yaml
./src/cli/target/debug/nex build pkg/cli/system/dool.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-dool-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/system/dool/1.3.8/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/lang/python3/3.12.2/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/toolchain/gcc/13.2.0/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/compression/bzip2/1.0.8/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/libffi/3.4.6/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/ncurses/6.4-20230520/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/libs/openssl3/3.3.1/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/compression/xz/5.4.6/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/zlib/1.3.1/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/expat/2.6.2/outputs/lib "$tmpdir"
unshare --user --map-root-user --mount --root "$tmpdir" /usr/lib/ld-linux-x86-64.so.2 --library-path /usr/lib /usr/bin/python3 /usr/bin/dool --version
```

The smoke command printed `Dool 1.3.8` and listed plugins from
`/usr/share/dool`.

`chezmoi` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/system/chezmoi.yaml
./src/cli/target/debug/nex build pkg/cli/system/chezmoi.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-chezmoi-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/system/chezmoi/2.59.0/bundles/dev "$tmpdir"
file "$tmpdir/usr/bin/chezmoi"
"$tmpdir/usr/bin/chezmoi" --version
```

The `file` command reported a statically linked ELF binary. The smoke command
printed `chezmoi version v2.59.0, commit v2.59.0, built at
2024-01-01T00:00:00Z, built by nex`.

`libconfuse` is packaged and built reproducibly as a `bmon` dependency.
Package proof:

```bash
./src/cli/target/debug/nex check pkg/libs/data/libconfuse.yaml
./src/cli/target/debug/nex build pkg/libs/data/libconfuse.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-libconfuse-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/libs/data/libconfuse/3.3/bundles/dev "$tmpdir"
gcc -I"$tmpdir/usr/include" -L"$tmpdir/usr/lib" -Wl,-rpath,"$tmpdir/usr/lib" -o "$tmpdir/confuse-smoke" -x c - -lconfuse
"$tmpdir/confuse-smoke"
ldd "$tmpdir/confuse-smoke" | rg 'libconfuse|libc'
```

The test program called `cfg_init`, read the default string through
`cfg_getstr`, and exited successfully. `ldd` showed
`libconfuse.so.2` loading from the package checkout.

`bmon` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/system/bmon.yaml
./src/cli/target/debug/nex build pkg/cli/system/bmon.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-bmon-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/system/bmon/4.0/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/data/libconfuse/3.3/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/net/libnl/3.11.0/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/ncurses/6.4-20230520/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/toolchain/gcc/13.2.0/outputs/lib "$tmpdir"
unshare --user --map-root-user --mount --root "$tmpdir" /usr/lib/ld-linux-x86-64.so.2 --library-path /usr/lib /usr/bin/bmon -V
```

The smoke command printed `bmon 4.0` and the upstream warranty text.

`libmd` is packaged and built reproducibly as an `openbsd-netcat` dependency.
Package proof:

```bash
./src/cli/target/debug/nex check pkg/libs/crypto/libmd.yaml
./src/cli/target/debug/nex build pkg/libs/crypto/libmd.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-libmd-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/libs/crypto/libmd/1.2.0/bundles/dev "$tmpdir"
gcc -I"$tmpdir/usr/include" -L"$tmpdir/usr/lib" -Wl,-rpath,"$tmpdir/usr/lib" -o "$tmpdir/libmd-smoke" -x c - -lmd
"$tmpdir/libmd-smoke"
ldd "$tmpdir/libmd-smoke" | rg 'libmd|libc'
```

The test program called `MD5Data("nex", 3, ...)` and printed
`1dd300a7572d1455f6d45db299ceceb2`. `ldd` showed `libmd.so.0` loading from
the package checkout.

`libbsd` is packaged and built reproducibly as an `openbsd-netcat` dependency.
Package proof:

```bash
./src/cli/target/debug/nex check pkg/libs/system/libbsd.yaml
./src/cli/target/debug/nex build pkg/libs/system/libbsd.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-libbsd-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/libs/system/libbsd/0.12.2/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/crypto/libmd/1.2.0/bundles/dev "$tmpdir"
gcc -I"$tmpdir/usr/include" -L"$tmpdir/usr/lib" -Wl,-rpath,"$tmpdir/usr/lib" -o "$tmpdir/libbsd-smoke" -x c - -lbsd
LD_LIBRARY_PATH="$tmpdir/usr/lib" "$tmpdir/libbsd-smoke"
LD_LIBRARY_PATH="$tmpdir/usr/lib" ldd "$tmpdir/libbsd-smoke" | rg 'libbsd|libmd|libc'
```

The test program called `strlcpy` from `bsd/string.h` and exited
successfully. `ldd` showed `libbsd.so.0` and `libmd.so.0` loading from the
package checkout.

`openbsd-netcat` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/net/openbsd-netcat.yaml
./src/cli/target/debug/nex build pkg/cli/net/openbsd-netcat.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-netcat-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/net/openbsd-netcat/1.238/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/libbsd/0.12.2/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/crypto/libmd/1.2.0/outputs/lib "$tmpdir"
unshare --user --map-root-user --mount --root "$tmpdir" /usr/lib/ld-linux-x86-64.so.2 --library-path /usr/lib /usr/bin/nc -h
```

The smoke command printed `OpenBSD netcat (Debian patchlevel 1.238-1)` and
usage text.

`taplo-cli` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/tools/taplo-cli.yaml
./src/cli/target/debug/nex build pkg/dev/tools/taplo-cli.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-taplo-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/dev/tools/taplo-cli/0.10.0/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/toolchain/gcc/13.2.0/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/libs/openssl3/3.3.1/outputs/lib "$tmpdir"
unshare --user --map-root-user --mount --root "$tmpdir" /usr/lib/ld-linux-x86-64.so.2 --library-path /usr/lib /usr/bin/taplo --version
```

The smoke command printed `taplo 0.10.0`.

`ast-grep` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/dev/tools/ast-grep.yaml
./src/cli/target/debug/nex build pkg/dev/tools/ast-grep.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-ast-grep-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/dev/tools/ast-grep/0.44.0/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/toolchain/gcc/13.2.0/outputs/lib "$tmpdir"
ln -s usr/lib "$tmpdir/lib64"
mkdir -p "$tmpdir/work"
printf 'const value = foo(1);\n' > "$tmpdir/work/sample.js"
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/ast-grep --version
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/sg --version
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/sg --pattern 'foo($A)' --lang javascript /work/sample.js
```

The smoke commands printed `ast-grep 0.44.0` for both binaries and matched
`foo(1)` in the JavaScript sample.

`fish` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/shells/fish.yaml
./src/cli/target/debug/nex build pkg/cli/shells/fish.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-fish-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/shells/fish/4.7.1/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/toolchain/gcc/13.2.0/outputs/lib "$tmpdir"
ln -s usr/lib "$tmpdir/lib64"
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/fish --version
unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/fish -c 'string match -r "f.*h" fish'
```

The smoke commands printed `fish, version 4.7.1` and matched `fish`.

`mosh` is packaged and built reproducibly. Package proof:

```bash
./src/cli/target/debug/nex check pkg/cli/net/mosh.yaml
./src/cli/target/debug/nex build pkg/cli/net/mosh.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
tmpdir=$(mktemp -d /tmp/nex-mosh-smoke.XXXXXX)
zub --repo .nex/repo checkout --copy x86_64/pkg/cli/net/mosh/1.4.0/bundles/dev "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/glibc/2.39/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/core/toolchain/gcc/13.2.0/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/libs/libabseil-cpp/20250127.0/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/libs/openssl3/3.3.1/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/ncurses/6.4-20230520/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/libs/protobuf/30.2/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/libs/system/zlib/1.3.1/outputs/lib "$tmpdir"
zub --repo .nex/repo checkout --copy --force x86_64/pkg/dev/lang/perl/5.38.2/bundles/dev "$tmpdir"
ln -s usr/lib "$tmpdir/lib64"
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/mosh --version
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/mosh-client --help | sed -n '1,4p'
LC_ALL=C LANG=C unshare --user --map-root-user --mount --root "$tmpdir" /usr/bin/mosh-server --help | sed -n '1,4p'
```

The smoke commands printed `mosh 1.4.0`, the `mosh-client` version header,
and `mosh-server` usage.

## Context and Orientation

Relevant durable notes:

- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/ostreefy-parity.md`
- `.agents/knowledge/agent-workflow.md`

Initial target rows from the matrix:

```text
age
ast-grep
bmon
btop
chezmoi
cloc
dool
fish
jq
mosh
nushell
openbsd-netcat
screen
taplo-cli
tokei
wget
```

Assemblies:

- Put daily runtime commands in `asm/desktop-vwl/desktop-vwl.yaml`.
- Put developer-only counters or source tools in `asm/desktop-dev.yaml` when
  they do not belong on the runtime desktop.

## Plan of Work

1. Inspect existing package manifests for CMake, Autotools, Cargo, Go, and
   simple source-build patterns.
2. Start with a small coherent batch. Prefer packages with few dependencies and
   predictable non-network smoke checks.
3. For each package, run:

   ```bash
   ./src/cli/target/debug/nex check <manifest>
   ./nex build <manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
   ```

4. Add each completed package to the right assembly and run the matching
   assembly check or build named by this sub-EP after the batch.
5. Smoke commands from built outputs or a checked-out assembly tree.
6. Commit each checked batch with an imperative scoped subject.

## Validation and Acceptance

This sub-EP is complete when every target row either:

- has a built package manifest, assembly ref, and smoke proof, or
- moves to a later sub-EP with a concrete reason recorded in this plan and the
  matrix.

Before each package commit, run `nex check`, the strict package build command,
and the package smoke check. Before an assembly commit, run `nex check` for the
changed assembly and the smallest checkout or build proof that the commands
exist in the system tree.

## Idempotence and Recovery

Package builds may update checksums, generated outputs, resolution data, and
profiles. Keep those generated fields with the package manifest commit that
required them. If a package build fails because an upstream source hash or build
dependency is wrong, update the sub-EP before trying the next package.

## Artifacts and Notes

Update these files during the sub-EP:

- Package manifests under `pkg/`
- `asm/desktop-vwl/desktop-vwl.yaml`
- `asm/desktop-dev.yaml`
- `.agents/ostreefy-parity-matrix.md`
- `.agents/SCRATCH_KNOWLEDGE.md`
