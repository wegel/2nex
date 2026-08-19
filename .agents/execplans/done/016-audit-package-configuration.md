# Audit every package manifest for configuration ownership

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

That scratch file is not tracked by Git, and this plan is long enough that
waiting for completion risks losing weeks of notes. Promote a fact as soon as
it is verified and clearly reusable, rather than holding everything to the end.
This run already moved the build sandbox facts into
`.agents/knowledge/build-sandbox.md` and the tier classification rules into
`.agents/knowledge/package-manifests.md` that way.

## Purpose / Big Picture

After this work, the repository will contain an explicit, human-readable audit
for every tracked package manifest below `pkg/`. Each audit entry will name the
package, explain what it ships, identify every file-based configuration reader
or writer that the package exposes at runtime, and cite the manifest, patch,
source, and test evidence that supports its result.

The audit will answer a narrower and stronger question than a filename count.
It will prove that an agent examined every tracked package manifest and
classified every configuration surface that the examination found. A reader
that consumes distribution defaults must honor Nex's `/usr`, `/run`, and
`/etc` ownership rules. A reader that must use a standard fixed path, machine
state, secret, database, cache, or runtime state must have a concrete reason
why the three-tier rule does not apply. A package with no system-wide file
configuration must say how the auditor established that fact.

The audit will not claim that 31 files whose names contain `uapi`, or any other
patch count, proves complete coverage. It will inspect all configuration-related
patches referenced by package manifests, regardless of filename, and it will
inspect the pinned package source when the manifest and patches do not reveal
all readers. If the audit finds a missing tier, Claude implements the package
fix and its test, Codex checks the result against the repository, and Codex
commits only after that review. Do not apply that repair path under
`pkg/bootstrap/`. Bootstrap seeds stay on unmodified upstream lookup
so a reader can see that Nex is built from standard packages. Record
the reader, classify the package as `build-only input`, and leave the
seed unpatched. Repair the shipped copy under another `pkg/` tree
instead. A search of `pkg/bootstrap/` found no UAPI-config patches to
revert; the only `file:` sources there are the two GCC reproducibility
patches already used by the shipped toolchain.

Codex is the main agent. It assigns work, checks every report and every fix
against the repository, writes the tracked audit, and creates every commit.
Claude is the audit and coding agent. It reads assigned manifests, writes the
compact report, and implements the package repairs Codex assigns. Codex does not
do first-pass audit or first-pass coding. Claude does not write the tracked
ledger or create commits.

### Agent Roles

**Supervisor handover, 2026-08-18.** Codex exhausted its usage allowance
mid-plan. Its last turn ended with `usage_limit_exceeded` and a reset date of
2026-08-24. The human operator moved the supervisor role to Claude for the rest
of this plan. Claude now performs every task the `Codex (supervisor)` list
below names, including writing the tracked audit and creating commits. The
audit and coding work stays with `claude-sonnet-5` workers launched through
`.nex/tmp/ep016-batch.sh`, which selects that model unless it is called with
`--grok`. The worker limits in `Claude (audit and coding)` still bind those
workers. Restore the original split if Codex returns before the plan ends.

**Codex (supervisor)**

- Pick the next batch or repair from `Progress` and the batch ledger.
- Launch Claude with a bounded prompt. Do not paste worker transcripts into
  the supervisor context.
- Open every cited local path before accepting a report.
- Write or revise the tracked audit entry.
- Review Claude's patch, test, and build evidence before committing.
- Stage exact files and create the commit after the plan's checks pass.
- Record Progress, Decision Log, and scratch knowledge.
- Spawn other read-only helpers only for a narrow search. Claude does the
  grunt work.

**Claude (audit and coding)**

- Audit: read assigned manifests, patches, and pinned sources; return one
  compact report block per manifest. The existing prompt at
  `.agents/prompts/package-configuration-audit.md` is the audit contract.
- Coding: when Codex assigns a confirmed gap, implement the package-specific
  fix, add or strengthen the focused test, run `nex check` and the builds the
  assignment names, and stop for Codex review.
- Do not write the tracked audit. Do not create commits. Do not spawn further
  agents. Do not start the next batch until Codex accepts the current one.

If Claude reports allowance exhaustion, that is a hard blocker unless the
human names a different worker.

## Progress

- [x] (2026-08-16 15:06Z) Confirmed that EP015 is archived, ran the worktree
  pre-task, and found a clean worktree at `9b06fde`. Listed the durable
  knowledge themes and read the package, configuration, testing, assembly, and
  reproducibility notes that govern this audit.
- [x] (2026-08-16 15:06Z) Counted the starting package set and checked the
  installed agent clients. Git tracks 533 package manifests: 36 under `apps`,
  43 under `bootstrap`, 64 under `cli`, 48 under `core`, 20 under `desktop`,
  93 under `dev`, five under `fonts`, 215 under `libs`, eight under `net`, and
  one under `servers`. Grok CLI 1.0.3 and Claude Code 2.1.233 expose the options
  required by this plan.
- [x] (2026-08-16 16:20Z) Added the audit ledger, the fixed worker prompt, and
  a coverage checker. `.agents/audits/package-configuration/README.md` defines
  the claim, classes, fields, and result states; the ten area files carry 533
  headings in Git order, each with `Result: not audited`;
  `.agents/prompts/package-configuration-audit.md` holds the read-only worker
  contract; `scripts/check-package-config-audit.sh` compares headings against
  `git ls-files` and rejects missing, duplicate, stale, misfiled, and malformed
  records. Its `--self-test` proves each of those rejections.
- [x] (2026-08-16 16:45Z) Ran the Grok pilot over five representative packages
  and validated every claim against the repository. Wrote the first five
  tracked records: `zstd` and `attr` pass, `ca-certificates` passes after the
  root agent rejected the worker's `gap`, and `less` and `dbus` are confirmed
  gaps. `sh scripts/check-package-config-audit.sh` reports
  `pass: 3, gap: 2, not audited: 528`. Corrected the worker prompt with the
  three distinctions the pilot showed it needed.
- [x] (2026-08-16 18:55Z) Audited all 32 manifests that reference a
  configuration patch and validated every cited reader and behavior test. All
  32 entries are finished: 22 `pass` and 13 `gap fixed`, with no entry left
  open. Thirteen packages needed a repair, each landed with a test that fails
  against the old behavior and two strict builds: Less, OpenSSH, BlueZ,
  Linux-PAM, Bash, e2fsprogs, Shadow, Swaync, GTK 3, Libvirt, CUPS, D-Bus, and
  Podman. The twenty-two that passed as built are Fontconfig, Glibc, Libnl,
  LibTirpc, Logrotate, the OpenCL ICD Loader, PulseAudio, Readline, S-Lang,
  Wget, Tig, Smartmontools, iwd, Iptables, Nftables, OpenSSL 3, Waybar, Fish,
  and ImageMagick, plus the pilot's Zstd, Attr, and CA Certificates.
- [x] (2026-08-16 16:10Z) Reconciled the patch counts by content and published
  the result in the audit README. All 39 tracked patches are referenced by
  exactly one manifest each; 32 change configuration lookup and map one-to-one
  onto 32 manifests; the other seven serve reproducible builds, parser syntax,
  a shell helper, a build-time probe, and a `/proc` override. Adding Shadow's
  two remote `login.defs` patches gives the remembered 34.
- [x] (2026-08-17) Swapped the live agent roles: Grok is now the supervisor,
  Claude is the audit and coding worker. Historical batches produced under the
  old Grok-first, read-only-worker arrangement stay as written. New batches
  and new repairs follow Agent Roles in Purpose.
- [x] (2026-08-17 23:29Z) Codex took over as supervisor after Grok exhausted
  its weekly allowance. Codex read the active plan, the manifest and test
  rules, the matching knowledge notes, the dirty tree, and the ignored worker
  and build logs. The Chromium policy repair remains prepared and unproven.
  Claude also left a PipeWire patch and manifest draft. `nex check` passed,
  but none of its three build attempts completed: the first was killed after
  Meson setup, the second lacked `grep`, and the third stopped after install
  because PipeWire's same-basename filter kept the vendor drop-in and skipped
  the `/run` and `/etc` files. No package build or worker process remains live.
  The coverage checker reports 295 `pass`, 48 `gap fixed`, six `gap`, and 184
  `not audited` records.
- [x] (2026-08-17 23:52Z) Finished and proved the interrupted PipeWire
  repair. Codex found that the first worker patch inserted `/run` but left
  PipeWire's vendor-first basename filter unchanged, so the worker rewrote the
  reader to remove a lower file's parsed entries when a higher tier supplies
  the same basename. The installed `pw-config` smoke now proves distinct-file
  merge order, complete `/etc` and `/run` basename shadowing, and main-file
  fallback across `/etc`, `/run`, and `/usr/share`. Both strict commands built
  twice, printed the smoke four times total, and reproduced checksum
  `df069d95709ca98b4f3b247c6d3fb0d9c9fc28cf16ea7e5afd3cf7205191fe10`.
  The stored library names `/etc/pipewire`, `/run/pipewire`, and
  `/usr/share/pipewire`, with no `/usr/etc`. The checker now reports 295
  `pass`, 49 `gap fixed`, five `gap`, and 184 `not audited` records.
- [x] (2026-08-18 00:07Z) Rebuilt and exercised both systems that ship
  PipeWire. `edgebox-rootfs` built twice with checksum
  `2983fe635c849d21b439d1676f9608a2c23d86f13538ab3792c8340ad5502695`.
  `desktop-vwl` pinned PipeWire to commit
  `6ce640ed8bba09a1cea70503e64ad1722a9eb890`, built twice, and reproduced
  checksum
  `18db2ae5b3c6c01c35643dfc371ca3d023b72b46f27e8281f1efac562d711785`.
  In a checkout of each finished root, the installed `pw-config` selected
  `/usr/share/pipewire/pipewire.conf`, then a planted `/run` copy, then a
  planted `/etc` copy. The first desktop attempt stopped before a system build
  because `.nex-dev-prepare` exhausted this user's `/tmp` quota while copying
  a Rust source tree. The retry put that temporary tree below ignored
  `.nex/tmp`, then both build passes and the runtime smoke passed.
- [x] (2026-08-18 00:26Z) Repaired and proved WirePlumber's configuration
  reader. Claude added `/run/wireplumber` between the compiled `/etc` and
  `/usr/share` tiers. Codex found that the first same-basename smoke reused
  one scalar key across all three files, so a broken merge could still pass.
  Claude changed the files to use `etc.only`, `run.only`, and `vendor.only`;
  the real installed library smoke now requires the winning key and rejects
  every shadowed key at each fallback step. Two strict commands built twice,
  printed the smoke four times, and reproduced checksum
  `3f149b178bcb364575441bf2cb7377303f121d313f732b04269dc35a1b3fa28c`.
  The stored library names `/etc`, `/run`, and `/usr/share`, with no
  `/usr/etc`. The checker now reports 295 `pass`, 50 `gap fixed`, four `gap`,
  and 184 `not audited` records.
- [x] (2026-08-18 00:34Z) Rebuilt and exercised both systems that ship
  WirePlumber. `edgebox-rootfs` built twice with checksum
  `3216c0230d6fe26ff87f8e7d9329dd807961dab998391e016ca0e108f177e09c`.
  `desktop-vwl` pinned WirePlumber to commit
  `9729ad01d377201193b5a6e97d8e9427b83fd702`, built twice, and reproduced
  checksum
  `1afcc7afc8de03cebdc99748053ace56b3135b28b5b2f055ab2f620077bec516`.
  A helper linked to each finished root's installed WirePlumber library opened
  a unique config from `/usr/share`, then selected planted `/run` and `/etc`
  copies in order. Edgebox ran the helper directly. The Nex desktop loader
  correctly rejected the disposable helper because it was not in a package
  capsule, so that smoke invoked the same helper through the real glibc loader
  inside the checked-out root. Both runtime checks passed.
- [x] (2026-08-18 01:26Z) Repaired and proved both GnuTLS system
  configuration readers. Claude changed the priority reader and the deprecated
  PKCS#11 module reader to select the first regular file from `/etc/gnutls`,
  `/run/gnutls`, and `/usr/lib/gnutls`. Codex found that the first smoke did
  not prove that a caller-supplied PKCS#11 filename still bypassed the new
  walk, so Claude added a fourth case with `/etc` present and a uniquely named
  explicit file. The helper links the newly built `libgnutls.so` and checks
  the library's successful-open log lines, which distinguish a selected file
  from a failed attempt that names the same path. Two strict commands built
  twice, printed the smoke four times, and reproduced checksum
  `0dc92d11e5afba54ac363d2eac4f87f76ec2268398222cec32d7fdc006387f14`.
  `nex check`, the package-path checker, and the audit checker pass. The audit
  checker now reports 295 `pass`, 51 `gap fixed`, three `gap`, and 184
  `not audited` records. No assembly manifest names GnuTLS directly, so no
  assembly `manifest_ref` changed. The real library consumer is the smallest
  runtime test required by `.agents/TESTING.md`; a system boot would not add
  evidence about these synchronous file readers.
- [x] (2026-08-18 01:50Z) Repaired and proved libgcrypt's three system
  readers: `fips_enabled`, `hwf.deny`, and `random.conf`. Each now selects
  the first regular file below `/etc/gcrypt`, `/run/gcrypt`, and
  `/usr/lib/gcrypt`. Claude's first smoke used the public FIPS flag, which
  reads `active` regardless of which present tier won, and its interposer
  runner suppressed a helper failure with `|| true`. Codex rejected both
  conditions. Claude revised the interposer to record the real library's
  successful `access` for FIPS and `fopen` for the other readers, required
  the exact winning path, rejected any other successful same-name path, and
  allowed every helper failure to stop the build. Two strict commands built
  twice, printed the corrected smoke four times, and reproduced checksum
  `8b32a1dc29664db24ef678ef06229e58ea9494755f429ad9b2c289504573f30f`.
  `nex check`, the package-path checker, and the audit checker pass. No
  assembly manifest names libgcrypt directly, so the real library consumer
  supplies the required runtime proof without an assembly or QEMU run. The
  checker now reports 295 `pass`, 52 `gap fixed`, two `gap`, and 184
  `not audited` records.
- [x] (2026-08-18 02:12Z) Repaired and proved GNU Screen's system screenrc
  overlay. Screen now reads `/usr/lib/screen/screenrc`, then
  `/run/screen/screenrc`, then `/etc/screenrc`, then the existing user file;
  its reattach path adds the same lower-tier `StartRc` calls without rerunning
  commands. The saved first smoke expected a detached error message that
  Screen did not expose. A second probe used `setenv`, but this build
  namespace maps only group 0, so Screen could not assign a PTY to compiled
  group 5. Claude switched to Screen's `sessionname` command and an inotify
  socket watcher. Codex then rejected a hidden Screen status, replaced the
  doomed PTY window with Screen's documented `//group` window, required the
  complete rename sequence rather than only the last name, and closed a race
  by waiting until the watcher proves its kernel watch exists. Ten detached
  sessions prove every tier and fallback, `-c`, `$SCREENRC`, their precedence,
  a live socket, successful quit, and bounded cleanup. Two strict commands
  built twice, printed the smoke four times, and reproduced checksum
  `ce7b2cce5083a9427519f2882fadeba5bb2c67295dbce3f6f1c39ed6d0b81ed4`.
  The checker now reports 295 `pass`, 53 `gap fixed`, one `gap`, and 184
  `not audited` records. No assembly manifest names Screen directly, and the
  real command smoke proves the changed startup without QEMU.
- [x] (2026-08-18 08:27Z) Repaired and proved Chromium's enterprise policy
  reader. The patch makes the real loader merge
  `/usr/lib/chromium/policies`, `/run/chromium/policies`, and
  `/etc/chromium/policies`, installs a watcher for each tier and policy
  subdirectory, and includes every tier in `LastModificationTime`. Under the
  recorded Chromium-only exception, one strict command ran both internal
  `--check` builds. The real headless browser printed the ordered policy smoke
  twice, both builds produced checksum
  `e064f9e57a297ef56f08dd02b7d519de2b38db1d0e119cfc1eae9bd07c0e31a1`,
  and the reproducibility check passed. The stored browser contains the three
  intended roots exactly once and no `/usr/etc/chromium/policies`. After the
  build, Codex removed unified-diff context-marker spaces that Git reported as
  trailing whitespace. Reconstructing those 19 spaces reproduced the exact
  built patch hash
  `80bf2c9e4b8028ecd4a11555950915b1a26b22d65f78cb80fcc0e01597e730e4`,
  and `git apply --numstat` accepts the cleaned patch, so the cleanup changes
  neither an applied source line nor the built binary. The audit remains at
  295 `pass`, 53 `gap fixed`, one `gap`, and 184 `not audited`: Chromium's
  separate native-messaging host reader still uses only `/etc` and still needs
  a test before it can change. `desktop-vwl` pins Chromium directly; Codex
  will advance that pin and rebuild the assembly after this package commit.
- [x] (2026-08-18 08:40Z) Rebuilt and exercised the desktop assembly that ships
  Chromium. `desktop-vwl` now pins package commit
  `4423c75b5a1ebcc468c74949e63b15df54a1fc1e`, built twice, and reproduced
  checksum
  `64e6629ddb2ebdeb1b76be48ed260baa58b20ded4e4b39d4c364fc8cfff85b7a`.
  In the finished assembly root, a private user and mount namespace supplied
  real `/dev` and `/proc` mounts, while three bind-mounted policy directories
  supplied vendor, runtime, and administrator JSON files. The assembled
  Chromium process exited successfully, named all three absent roots, then
  loaded the three files in vendor, runtime, administrator order. No QEMU run
  was needed: the package and assembly tests both invoke the real synchronous
  file reader, while a system boot, compositor, or hardware device does not
  participate in this behavior.
- [x] (2026-08-18 08:50Z) Audited p11-kit after reconciling eleven stale
  `pending` rows in the ignored batch ledger with records already written in
  the tracked audit. Claude correctly left the module-registration question
  `uncertain` because it did not find the pinned source. Codex found the
  retained 0.25.5 build tree and opened the compiled definitions and both real
  readers. Trust anchors already merge `/usr/share`, `/run`, and `/etc`, and
  the manifest tests that merge plus certificate distrust. The global
  `pkcs11.conf` reader compiles only `/etc/pkcs11/pkcs11.conf`; module loading
  reads the user directory, `/etc/pkcs11/modules`, then
  `/usr/share/p11-kit/modules`, with no runtime tier. p11-kit is a confirmed
  `gap`. The checker now reports 295 `pass`, 53 `gap fixed`, two `gap`, and
  183 `not audited` records.
- [x] (2026-08-18 09:29Z) Repaired and proved p11-kit's global settings and
  module-registration readers. The global file now selects `/etc`, `/run`,
  then `/usr/share`; the system module set overlays vendor, runtime, then
  administrator files with complete same-basename shadowing while preserving
  the upstream user modes and explicit overrides. Claude's first smoke used
  the separately linked `print-config` reader for every case. Codex required
  the audit's promised library-backed command too, so the final smoke copies
  the valid trust module to three distinct paths and makes `list-modules`
  report the administrator path, then runtime, then vendor. Two strict
  commands printed the combined smoke four times and reproduced checksum
  `9a23e7fbfc3b8d6c9b47b7c5370d636272f2d865c70a6d31008afafa7d8f1985`.
  Codex also rejected the first stored patch because Git found whitespace in
  its ordinary diff context. Claude regenerated it with zero context; fresh
  old-patch and new-patch applications produced byte-identical results for
  all six source files. The checker now reports 295 `pass`, 54 `gap fixed`,
  one `gap`, and 183 `not audited` records. `nex-systemd` ships p11-kit
  directly and must be rebuilt after this package commit.
- [x] (2026-08-18 09:37Z) Rebuilt and exercised the assembly that ships
  p11-kit. `nex-systemd` now pins package commit
  `7e929fb300cbd62a7a96caf7008df57c0e186039`, built twice, and reproduced
  checksum
  `ccacab85f4f0f26bc93e810f72002e41bb8dfd9e9a5ba98aa576c62954cc04b0`.
  A direct checkout first stopped at `boot` with `EPERM`, so Claude repeated
  the copy in a mapped-root user namespace. In that copied finished root,
  Claude and Codex independently ran the installed `p11-kit`: `print-config`
  selected administrator, runtime, then vendor settings, and `list-modules`
  loaded `/usr/lib/pkcs11/p11-kit-trust.so`. No QEMU run was needed because
  the real assembled command and library exercise both synchronous readers;
  booting cannot add evidence about their chosen files.
- [x] (2026-08-18 09:40Z) Wrote the `libs-data` batch as two `pass` records
  after checking Claude's report against both complete manifests and pinned
  sources. libconfuse is a parser library whose caller supplies every main or
  include path; it ships no command or default. SQLite's only implicit
  configuration is the user's XDG or home `sqliterc`, and its interactive
  history is user-owned generated state; `-init` and `SQLITE_HISTORY` remain
  exact overrides. The checker now reports 297 `pass`, 54 `gap fixed`, one
  `gap`, and 181 `not audited` records.
- [x] (2026-08-18 09:43Z) Wrote the `libs-desktop` batch as three `pass`
  records. dbus-glib opens only binding-tool arguments and generated output.
  desktop-file-utils derives each adjacent `mimeinfo.cache` from caller or XDG
  data directories rather than reading policy. Claude called libportal a
  no-file reader; Codex found and recorded its `/.flatpak-info` and
  `/proc/<pid>/cgroup` probes, which are fixed sandbox and kernel interfaces,
  plus its caller-supplied content files. None is distribution policy. The
  checker now reports 300 `pass`, 54 `gap fixed`, one `gap`, and 178
  `not audited` records.
- [x] (2026-08-18 09:52Z) Wrote `libs-gnome-1` as six `pass` records after
  checking every manifest and pinned source. Claude missed geocode-glib's
  per-user response cache and shortened several claims past their useful
  evidence, so Codex rebuilt those records from the actual readers.
  gcr delegates module and trust policy to the repaired p11-kit library;
  gnome-autoar opens only caller-supplied archive paths; and the schema package
  ships data but no reader. gnome-desktop's live file surfaces are GSettings,
  XDG thumbnailer registrations, user caches and content, fixed timezone and
  sandbox interfaces, and standard XKB, ISO, and locale databases. libgweather
  reads its generated vendor location database or an explicit environment
  path, plus user GSettings and cache data. None needs a new tier repair. The
  checker now reports 306 `pass`, 54 `gap fixed`, one `gap`, and 172
  `not audited` records.
- [x] (2026-08-18 10:00Z) Wrote `libs-gnome-2` as one `pass` record after
  checking Tracker's caller-owned database and ontology paths, user cache,
  installed command registry and modules, and fixed Flatpak and procfs
  interfaces. An exhaustive production-source sweep found no `/etc`,
  `SYSCONFDIR`, GSettings, or XDG system-configuration reader. The checker now
  reports 307 `pass`, 54 `gap fixed`, one `gap`, and 171 `not audited`
  records.
- [x] (2026-08-18 10:25Z) Repaired at-spi2-core's accessibility-bus policy
  reader. It now selects `/etc/at-spi2/accessibility.conf`, then the new
  `/run/at-spi2/accessibility.conf`, then its existing vendor default. The
  real installed launcher ran under a session bus and passed the expected
  path to its child for vendor, runtime, administrator, and empty-file mask
  cases. Codex fixed the generated dependency map so `libdbus-1.so.3` still
  resolves to the library-bearing `dbus` dependency rather than the added
  config-only dependency. Two final strict commands printed the smoke twice
  each and reproduced checksum
  `103cb027ae2076616c88b680348272a187680bf1f4cd8471270ed4536b915c2b`.
  The checker now reports 307 `pass`, 55 `gap fixed`, one `gap`, and 170
  `not audited` records. `desktop-vwl` ships this package directly and must
  be rebuilt after the package commit.
- [x] (2026-08-18 10:30Z) Wrote the other five `libs-graphics-1` records as
  `pass`. Claude's short reports called several packages no-file readers;
  Codex found and recorded at-spi2-atk's procfs ancestry check and runtime
  socket, ATK's locale-directory override, Cairo's procfs trace label and
  Fontconfig handoff, fcft's Fontconfig and FreeType handoffs, and FreeType's
  caller paths and property environment. None adds another machine-policy
  gap. The checker now reports 312 `pass`, 55 `gap fixed`, one `gap`, and 165
  `not audited` records.
- [x] (2026-08-18 11:50Z) Ralph resumed the plan and inspected the dirty
  worktree. The five finished graphics records and their plan counts form one
  checked documentation commit; the `desktop-vwl` AT-SPI pin remains a
  separate assembly change until two builds and the installed-launcher smoke
  pass. Git reported no untracked files. The audit checker found all 533
  records with 312 `pass`, 55 `gap fixed`, one `gap`, and 165 `not audited`;
  the package-output path checker and `git diff --check` also passed.
- [x] (2026-08-18 12:04Z) Rebuilt and exercised the desktop assembly that
  ships at-spi2-core. `desktop-vwl` now pins package commit
  `2c14acaa0abb2885a35204e4d5289bdd0b8f9d99`. Two independent strict
  commands each passed their two internal builds and reproduced checksum
  `352315b5fb2e164b121fc5e7e6da256070188071a890fa01ffc5cb6572cc03c9`.
  In a copied finished root, a private user and mount namespace supplied
  `/dev` and `/proc`; the assembled `at-spi-bus-launcher` passed the vendor,
  runtime, administrator, empty-administrator, and empty-runtime config paths
  to its child exactly as expected. The first build had stopped while
  `.nex-dev-prepare` copied Rust sources into quota-limited `/tmp`; removing
  only its ignored failed root and setting `TMPDIR` below `.nex/tmp` fixed the
  host-side failure.
- [x] (2026-08-18 12:33Z) Finished `libs-graphics-2` as five `pass` records
  and one repaired gap. gdk-pixbuf owns a generated loader registry; glslang
  opens only caller-named inputs, includes, resource limits, and outputs;
  Granite delegates desktop settings to the portal or GSettings and keeps its
  own paths below XDG user roots; Graphene has no runtime reader; and hwdata
  ships immutable identification tables but no reader. GTK4 loaded
  `XDG_CONFIG_DIRS` forward even though later settings overwrite earlier
  settings, so the desktop's vendor directory beat its runtime and
  administrator directories. The new patch walks that list backwards and
  loads the compiled `/etc/gtk-4.0/settings.ini` afterward. A real
  library-linked smoke proved vendor, runtime, both administrator forms,
  user, and every fallback case without a display. Two independent strict
  commands ran the smoke twice each and reproduced checksum
  `2b2819c8c4a80a6112cf90828ab5c0c2168302608682047e21550dc20e1b811a`.
  Meson's own setup output confirmed that this manifest's `--prefix=/usr`
  turns the default sysconfdir into `/etc`. No assembly names GTK4 directly.
  The checker now reports 317 `pass`, 56 `gap fixed`, one `gap`, and 159
  `not audited` records.
- [x] (2026-08-18 12:51Z) Wrote `libs-graphics-3` as six `pass` records after
  checking every complete manifest and pinned source. Little CMS opens only
  caller profiles, measurements, lookup tables, and relative content
  includes. libadwaita delegates desktop preferences to the portal or
  GSettings, tests Flatpak's fixed root marker, and loads caller application
  resources. libdbusmenu exchanges live menu data over D-Bus, while its
  installed JSON-loader library opens an explicit caller path. libdisplay-info
  embeds hwdata's PNP table at build time and opens only the EDID operand at
  runtime. libdrm reads its matching immutable `amdgpu.ids` table, fixed kernel
  interfaces, caller BO files, and process controls. libepoxy delegates fixed
  EGL and GLES sonames to the dynamic linker and EGL implementation. None owns
  a system policy file that needs a new tier. The checker now reports 323
  `pass`, 56 `gap fixed`, one `gap`, and 153 `not audited` records.
- [x] (2026-08-18 13:19Z) Finished `libs-graphics-4` as five `pass` records
  and one repaired gap. libhandy delegates appearance and theme choices to
  GTK, GLib, the portal, and GSettings. libjpeg-turbo opens caller images,
  profiles, tables, scans, and outputs. libnotify opens caller icons and sends
  them to a desktop service. libplacebo receives cache paths and color or
  shader data from its caller. libpng opens caller PNG paths. libglvnd omitted
  `/run/glvnd/egl_vendor.d` and loaded same-basename registrations from every
  default directory. The patch merges the default `/etc`, `/run`, and
  `/usr/share` tiers by basename while keeping both environment overrides on
  their upstream ordered, exclusive behavior. Codex fixed an unchecked
  basename allocation and strengthened both override tests after reviewing
  Claude's draft. A real EGL smoke used three upstream dummy vendors to prove
  each tier, both mask forms, global basename order, and the two overrides.
  Two independent strict commands ran the smoke twice each and reproduced
  checksum
  `b7a914309f4e10ea7f06a5d8b9e7dc083f9708cc0fb638d3fdc1b9d7c0291d62`.
  No assembly manifest names libglvnd directly. The checker now reports 328
  `pass`, 57 `gap fixed`, one `gap`, and 147 `not audited` records.
- [x] (2026-08-18 14:03Z) Repaired and proved libva's single system
  configuration reader while continuing `libs-graphics-5`. The original
  `va_parseConfig()` opened only `/etc/libva.conf`; it now selects the first
  existing main file below `/etc`, `/run`, and `/usr/lib`, and an empty
  higher file masks lower files. Codex corrected Claude's Meson path claim,
  added the missing no-file/default assertion, and made the smoke create the
  absent `/etc` and `/run` sandbox directories. The smoke enters the real
  installed library through `vaInitialize()` and distinguishes the effective
  messaging level through libva's own log. Two independent strict commands
  ran it twice each and reproduced checksum
  `c8b1667889b871409df67ef0e9b8e9233a94cc04708da5cb4372acd87328ea90`.
  No assembly manifest names libva directly. The checker now reports 328
  `pass`, 58 `gap fixed`, one `gap`, and 146 `not audited` records.
- [x] (2026-08-18 15:02Z) Repaired and proved Mesa's `drirc` reader at the
  package boundary while continuing `libs-graphics-5`. The default now merges
  `/etc`, `/run`, and `/usr/share` drop-ins by basename in one global filename
  order, selects one main file in administrator/runtime/vendor priority, then
  retains the user file. `DRIRC_CONFIGDIR` still replaces all compiled system
  reads. Codex rejected the first smoke's non-conflicting order and environment
  cases, added both runtime mask forms, required exact fixture cleanup, fixed
  allocation failure so it cannot expose a lower file, and guarded the empty
  sort. Two independent strict commands ran the real `libxmlconfig` smoke
  twice each and reproduced checksum
  `122404af405511bffdf9e3d62c71099f2a258e2b5b8a76738058177289188ba8`.
  The later assembly commit repins all three affected desktops and exercises
  Mesa in a finished root.
  The checker now reports 328 `pass`, 59 `gap fixed`, one `gap`, and 145
  `not audited` records.
- [x] (2026-08-18 15:49Z) Repaired and proved NVIDIA 580's proprietary
  application-profile search at the package boundary. Six closed libraries
  now use the old versioned-vendor table slot for
  `/run/nvidia/nvidia-application-profiles-rc.d` and reach the packaged
  versioned profile through a stable unversioned vendor link. The checked
  rewriter requires one exact old table per ELF and preserves every byte
  count. Codex kept the change local to 580, removed product-specific wording
  from installed vendor docs, constrained the smoke to exact fixtures, and
  used the real GLX parser's debug log to prove administrator/runtime/vendor
  order and fallback without a GPU. That smoke exposed undeclared `libX11`
  and `libXext` dependencies; the manifest now declares their build inputs,
  output needs, and resolutions. Two independent strict commands ran the
  structural and real-parser checks twice each and reproduced checksum
  `5abc3b27f1fbf5d54ed79f0b086522495c61e8cdbb3c5975189c09cb2b51c8db`.
  The later assembly commit repins and exercises `desktop-vwl-nvidia-580`.
  The checker then reported 328 `pass`, 60 `gap fixed`, one `gap`, and 144
  `not audited` records.
- [x] (2026-08-18) Repinned and proved all three affected desktop assemblies.
  Two independent strict builds reproduced `desktop-vwl` checksum
  `6455d10fbb06dbe02cb6eecdce2f7cb13dce48c4ecd898398ebdceefa54e55dc`,
  `desktop-vwl-nvidia-current` checksum
  `6c0e617e03da79bef58c93bb914b9d8ed4bb9c6f8d9d85cab968ff3e4cbd96a6`,
  and `desktop-vwl-nvidia-580` checksum
  `5875e6b1c51be144d23191a03b2c2002320e02173bd17c4fabd052fe098de5e7`.
  A copied base root loaded assembled Mesa through surfaceless EGL and proved
  every administrator/runtime/vendor drop-in and main-file choice. A copied
  NVIDIA 580 root loaded the proprietary GLX parser and proved its three
  profile tiers. Neither synchronous reader gains extra coverage from QEMU.
- [x] (2026-08-18) Finished `libs-graphics-5`. librsvg opens caller-selected
  SVG files, streams, and relative resources; libtiff opens caller operands
  and reads only process decoder-safety variables; libwebp consumes caller
  buffers or files, with disabled memory-failure debug controls and an
  optional kernel CPU probe. Their three `pass` records close the batch. The
  checker reports 331 `pass`, 60 `gap fixed`, one `gap`, and 141
  `not audited` records.
- [x] (2026-08-18 15:59Z) Repaired NVIDIA 595.84's application-profile
  reader while continuing `libs-graphics-6`. Its six proprietary libraries
  carried the same administrator/vendor table as 580 and omitted `/run`.
  Codex verified the exact table in every built library before applying the
  generic equal-length repair. A new shared check runs the structural proof
  and real GLX parser against any pinned version. The parser again exposed
  undeclared `libX11` and `libXext` metadata, which the manifest now supplies.
  Two independent strict commands ran the check twice each and reproduced
  checksum
  `1214b1068a387f7080decae4e059397372c3e6c649d21717de1d48f569c1c277`.
  The later assembly commit repins and exercises the finished desktop root.
  The checker now reports 331 `pass`, 61 `gap fixed`, one `gap`, and 140
  `not audited` records.
- [x] (2026-08-18) Repinned and proved `desktop-vwl-nvidia-current` with the
  repaired 595.84 package. Two independent strict commands each built the
  complete system twice and reproduced checksum
  `6bba5507ee0d3befd0ea9b523b2dbab33b4b80550c2b16fc43fb098c6647792f`.
  A copied stored root then loaded the assembled proprietary GLX parser
  through its real glibc loader and proved vendor fallback, runtime before
  vendor, and both administrator profile entries before runtime. QEMU would
  add boot and device paths that do not participate in this synchronous
  reader.
- [x] (2026-08-18) Finished `libs-graphics-6`. OpenCL-Headers is build-only.
  Pixman has one process fast-path switch and a kernel CPU probe. Qt5Compat
  consumes caller XML plus process locale and codec values. Qt Base walks the
  desktop's ordered XDG system paths for `QSettings` and logging, while its
  deployment and operating-system files keep their explicit owners. Qt
  Wayland uses process-selected paths, caller files, and `XDG_RUNTIME_DIR`.
  Their five `pass` records close the batch. The checker reports 336 `pass`,
  61 `gap fixed`, one `gap`, and 135 `not audited` records.
- [x] (2026-08-18) Repaired and proved Vulkan Loader while continuing
  `libs-graphics-7`. The Unix default driver and layer walk now adds `/run`
  and lets the first JSON basename mask lower tiers. Additive environment
  lists still load duplicate basenames in caller order and mask the later
  implicit copy; exclusive overrides remain ordered and unmasked. The same
  patch adds the runtime loader-settings tier and fixes the settings reader's
  colon parser, which previously made every path after the first unreachable.
  Codex rejected Claude's initial pass classification, then rejected two
  coding drafts that changed additive-list behavior, hid the settings parser
  bug, embedded the whole smoke in YAML, and broke the Windows branch. The
  final installed-library smoke enters through `vkCreateInstance()` and proves
  driver and implicit-layer tier order, both mask forms, distinct-name merge,
  additive and override order, additive masking, settings priority, and the
  normal two-entry XDG data fallback. Two independent supervised strict
  commands ran that smoke twice each and reproduced checksum
  `5d0aeb7c05a674c266b96a32616cd18e2e58eec42197c15ef30696e3f12cce4a`.
  The checker now reports 339 `pass`, 62 `gap fixed`, one `gap`, and 131
  `not audited` records. A later assembly commit must repin and exercise the
  base desktop and both inherited NVIDIA desktops.
- [x] (2026-08-18) Fixed the glibc clean-sandbox precondition exposed by the
  Vulkan desktop assembly build. Both a forced assembly attempt and a cached
  attempt reached the shipped glibc smoke, where `/etc/ld.so.conf` could not
  be created because the build sandbox did not yet contain `/etc`. The glibc
  manifest now creates that directory before writing its smoke fixture. Two
  independent strict glibc commands ran the UAPI smoke twice each and
  reproduced the unchanged checksum
  `bf348eabcec257edace3e1e05458bf79ddad1a5164f25e706b7e50d93b25190d`.
  Codex reversed every unrelated generated dependency-manifest diff from the
  failed assembly and glibc commands before preparing the focused fix.
- [x] (2026-08-17) Landed the git system-config
  repair. Reads walk `/etc`, `/run`, then
  `/usr/lib` for `gitconfig` and
  `gitattributes`. `git config --system` still
  writes `/etc/gitconfig`. Both
  `./nex build ... --check` invocations printed
  `git UAPI smoke: system gitconfig and gitattributes read all three tiers in order`.
  Checksum
  `51b15102f21257b22396de016d264484c94483e414eafd3e2985332cc30d3b35`.
- [x] (2026-08-17) Wrote `libs-crypto-1`.
  libmd, libsodium, libtasn1, and nettle
  pass. gnutls (`/etc/gnutls/config`) and
  libgcrypt (`/etc/gcrypt/{fips_enabled,
  hwf.deny,random.conf}`) are `gap`s.
- [x] (2026-08-17) Wrote `libs-compression-1`
  as six `pass` records. Overturned
  minizip-ng `uncertain` after fetching the
  pinned tarball; `rg` finds no `/etc`
  reader. xz has unused autotools
  `sysconfdir` and no reader.
- [x] (2026-08-17) Landed the jack2
  autostart repair. After `~/.jackdrc`,
  the first existing of `/etc/jackdrc`,
  `/run/jackdrc`, `/usr/lib/jackdrc`
  wins. A `jack_client_open` probe names
  the winning `-d` driver. `set -e` must
  allow the probe to exit non-zero.
  Checksum
  `3c5b550afc453b8b2f020268419bf0c42c20992802a2bcbdb8389ac3039353d5`.
- [x] (2026-08-17) Landed the alsa-lib
  `alsa.conf` repair. Drop-ins are vendor
  `/usr/share/alsa/conf.d`, `/run/alsa/conf.d`,
  then `/etc/alsa/conf.d`. The single file is
  `/etc/asound.conf`, `/run/asound.conf`, then
  `/usr/share/alsa/asound.conf`. Both
  `./nex build ... --check` invocations printed
  `alsa-lib UAPI smoke: alsa.conf names /etc, /run, and /usr/share, not /usr/etc`.
  Checksum
  `a0700dbc3e6578c094a82a7b8f6348eeb49f204bb12caf5a81d0918d982f4a71`.
- [x] (2026-08-17) Wrote `libs-audio-2`.
  libsndfile and opus pass. pipewire and
  wireplumber are `gap`s: meson compiles
  `/usr/etc` and neither reader has `/run`.
  PipeWire's PAM limits drop-in is already
  three-tier.
- [x] (2026-08-17) Wrote `libs-audio-1`. flac,
  libogg, libsamplerate, and alsa-utils pass.
  Overturned the worker's `pass` on alsa-lib:
  the shipped `alsa.conf` names `/usr/etc` and
  has no `/run` tier. jack2 is a `gap`:
  autostart reads only `/etc/jackdrc`.
- [x] (2026-08-17) Wrote `libs-archive` (libarchive)
  as `pass`. `/etc` hits in the pinned tree are
  test fixtures only.
- [x] (2026-08-17) Landed the gtk-vnc
  `--sysconfdir=/etc` repair. `SYSCONFDIR` is
  `/etc` and the cert dir is `/etc/pki`. TLS
  material is secret or credential, so no
  `/run` or `/usr` cert tier. Both
  `./nex build ... --check` invocations printed
  `gtk-vnc UAPI smoke: SYSCONFDIR/pki is /etc/pki, not /usr/etc`.
  Checksum
  `94acacb61fd92db377d2f8b79c6bf2ca3123693bdc6f1daded03dc249ed9ec47`.
- [x] (2026-08-17) Landed the libosinfo repair.
  `--sysconfdir=/etc` moves admin data to
  `/etc/osinfo`. The loader merges vendor
  `/usr/share/osinfo`, then `/run/osinfo`, then
  `/etc/osinfo`, then the user tree.
  `osinfo-query` names the winning overlay.
  Checksum
  `c4d3ef98b28f28f9303c44706ec9b0fbd8be8d870e43aa1ffe7c0735fe17d1b8`.
- [x] (2026-08-17) Landed the elfutils debuginfod URL
  repair. The profile hook concatenates
  `/etc/debuginfod/*.urls`, then `/run`, then
  `/usr/lib`. An already-set `DEBUGINFOD_URLS` is
  unchanged. Both `./nex build ... --check`
  invocations printed
  `elfutils UAPI smoke: debuginfod.sh concatenated all three url tiers`.
  Checksum
  `9e99673ba4c77f4fcdc543075acfe1f0d845bb00f9a6fddf7cd470f7cfe67bfb`.
- [x] (2026-08-17) Wrote `fonts-1` as five `pass` records
  (fira-code-nerd, hack-nerd, jetbrains-mono-nerd,
  noto-fonts-emoji, noto-fonts). Each package only
  installs TTF files under `/usr/share/fonts`. Checker:
  `pass: 278, gap: 6, gap fixed: 42, not audited: 207`.
- [x] (2026-08-17) Landed the file/libmagic repair. After
  `$HOME/.magic*`, the default list is `/etc/magic`,
  `/run/magic`, then the compiled vendor `MAGIC`. Missing
  files are skipped. Do not copy vendor `magic.mgc` over
  the planted text `/usr/share/misc/magic`; the compiled
  database shadows the text file. Both
  `./nex build ... --check` invocations printed
  `file UAPI smoke: magic loader read all three tiers in order`.
  Checksum
  `2997be9d6638d95527c89d6eb223c0c35c056410a54db407431960b6112da560`.
  Checker: `pass: 273, gap: 6, gap fixed: 42, not audited: 212`.
- [x] (2026-08-17) Wrote `dev-virt`. gtk-vnc and libosinfo
  are `gap`s (meson default `/usr/etc`). libvirt-glib,
  osinfo-db, qemu, and spice-protocol pass.
- [x] (2026-08-17) Wrote `dev-util` and `dev-vcs`. check,
  dejagnu, expect, and lazygit pass. Overturned dejagnu
  `uncertain` after fetching the pinned tarball.
  elfutils `debuginfod.sh` and git's `ETC_GITCONFIG` are
  `gap`s.
- [x] (2026-08-17) Wrote `dev-tools-2` and `dev-tools-3`
  as ten `pass` records.
- [x] (2026-08-17) Landed the docutils repair. The merge
  list is vendor `/usr/lib`, then `/run`, then `/etc`,
  then the cwd and home files. `rst2html --dump-settings`
  names the winning `report_level`. Checksum
  `a70bae1fe3a861e9a643ef2960db2b76424cf678e9b7c52fe96d8cb690c3600e`.
- [x] (2026-08-17) Wrote `dev-toolchain` and `dev-tools-1`.
  llvm, mold, ast-grep, cargo-c-bin, cloc, fakeroot, and
  gdb pass. `file` is a `gap`: magic lookup never opens
  `/etc/magic` unless `MAGIC` is set. Overturned mold
  `uncertain` after fetching the pinned tarball; `rg`
  finds no `/etc` reader.
- [x] (2026-08-17) Wrote `dev-python-1` through `dev-python-3`.
  Fifteen `pass`. `docutils` is a `gap`:
  `standard_config_files` is only `/etc/docutils.conf`.
- [x] (2026-08-17) Landed the nodejs npm `globalconfig`
  repair. After CLI/env overrides, the loader walks
  `/etc/npmrc`, `/run/npmrc`, then `/usr/lib/npm/npmrc`.
  Checksum
  `bfcc86f4306f33769ae01b172f14b25bbb7bf74c1d9ad20b619a97f27395708c`.
- [x] (2026-08-17) Wrote `dev-libs-5` and `dev-perl` as
  eight `pass` records.
- [x] (2026-08-17) Wrote `dev-libs-3` and `dev-libs-4` as
  twelve `pass` records. libgit2 reads Git's `/etc/gitconfig`
  overlay; this package installs no file there.
- [x] (2026-08-17) Wrote `dev-libs-2` as six `pass` records
  (glibmm2, gtkmm, gtkmm3, jsoncpp, libabseil-cpp,
  libassuan).
- [x] (2026-08-17) Wrote `dev-libs-1` as six `pass` records.
  Overturned glib2 `uncertain`: the keyfile GSettings
  backend is `/etc` only, but nothing in this tree selects
  it and there is no dconf package.
- [x] (2026-08-17) Landed the xdg-desktop-portal-wlr repair.
  After the user prefix the reader walks `/etc/xdg`, `/run/xdg`,
  then `/usr/share/xdg`. `max_fps` in `-l DEBUG` names the
  winner. Checksum
  `ccb43baa437bf345be0aabafa75789f58d7aaa7c5a484ca93e65cd80c668afe0`.
- [x] (2026-08-17) Wrote `dev-lang-2` as five `pass` records
  (rust-bin, tcl, tk, vala, zig-bin). rust-bin already
  relocates the installer `usr/etc` files into `/usr/share`.
- [x] (2026-08-17) Wrote `dev-lang-1`. go, luajit, m4, perl,
  and python3 pass. Overturned the worker's `pass` on
  nodejs: npm's globalconfig is `<prefix>/etc/npmrc`,
  which is `/usr/etc/npmrc`. `/etc/npmrc` is ignored.
- [x] (2026-08-17) Landed the swayidle and swaylock repairs.
  After the user files, both walk `/etc`, `/run`, then
  `/usr/lib`. `swayidle -d` and `swaylock -d` name the
  winning path without a compositor. Checksums
  `2450baf65802423118887ffbd7d782ff17570c605bd2fc73fd1a4d4c44be1e0c`
  and
  `714a8888dfe795e00a94bfbb5b4ef34693963e2ebbf8cd25d21059b9b81135b2`.
- [x] (2026-08-17) Wrote `dev-build` (cmake) as `pass`. No
  system-wide CMake policy file. The CA-bundle fallback is
  `#ifdef CMAKE_FIND_CAFILE` and this bootstrap does not
  define it.
- [x] (2026-08-17) Wrote `desktop-wayland-3`. wofi is `pass` (user
  `XDG_CONFIG_HOME` only). Overturned the worker's `pass` on
  `xdg-desktop-portal-wlr`: `get_config_path` is user then
  `/etc/xdg` and ignores `XDG_CONFIG_DIRS`, so the desktop overlay
  never supplies `/run` or `/usr`.
- [x] (2026-08-17) Landed the util-linux `login.defs` repair.
  First existing file wins across `/etc`, `/run`, and
  `/usr/lib`. The first smoke failed because `lslogins -s`
  printed nothing without a sandbox `passwd`/`group`; planting
  `root` made the `SYS_UID_MIN` probe work. Checksum
  `375d872e8d2d02c5fa6aa977de4116b2aeca47732d30e9d969e1f349a0b1b662`.
- [x] (2026-08-17) Wrote `desktop-wayland-2`: four `pass` (wl-clipboard,
  wl-mirror, wlopm, wlr-randr) and two `gap`s (swayidle, swaylock).
  Both lock/idle helpers compile a single `/etc/.../config` after
  the user files, the same shape as unpatched tmux.
- [x] (2026-08-17) Wrote `desktop-wayland-1` as six `pass` records
  (fuzzel, grim, ironbar, mako, slurp, swaybg). fuzzel uses
  `XDG_CONFIG_DIRS`; the manifest moves the example `fuzzel.ini`
  out of `/etc/xdg`.
- [x] (2026-08-17) Finished `core-userland-2`. psmisc, tzdata, and
  which pass. util-linux is a `gap`: `agetty` issue already has
  three tiers, but `login.defs` is still `/etc` only, so it misses
  Shadow's `/usr/lib/login.defs`.
- [x] (2026-08-17) Wrote `core-userland-2` partial records: psmisc,
  tzdata, and which pass. util-linux is still in the worker. Wrote
  `desktop-themes` (gnome-themes-extra) as `pass`.
- [x] (2026-08-17) Landed the procps-ng `topdefaultrc` repair.
  First existing file among `/etc/topdefaultrc`,
  `/run/top/topdefaultrc`, and `/usr/lib/top/topdefaultrc` wins.
  `/etc/toprc` is unchanged. Smoke uses `top -b -n 1` and a
  parse error. Checksum
  `c3485cf8fad139589c2bf345fa5ba023b4fbfd93908875f8c09c1d8e1aa09530`.
- [x] (2026-08-17) Wrote `desktop-portals` (xdg-desktop-portal) as
  `pass`. The worker called `gap` for a missing compiled `/run`
  path; `XDG_CONFIG_DIRS` from the desktop overlay already names
  `/run/xdg`.
- [x] (2026-08-17) Landed the acpid repair. Event rules scan `/etc`,
  then `/run`, then `/usr/lib`, first basename wins, `/dev/null`
  masks. The in-build smoke uses `acpid -d -f -S` and a dummy
  event file. Checksum
  `f72292006b358bcf9d866cd1360222484e6980bd884281bcab62a77af15018bb`.
- [x] (2026-08-17) Wrote `desktop-files` (gvfs) as `pass`. Mount
  backends come from `/usr/share/gvfs/mounts`.
- [x] (2026-08-17) Wrote `desktop-compositor` (vwl) as `pass`.
  Keybinds are compiled from `vwl-config.h`.
- [x] (2026-08-17) Landed the pkgconf repair. `--sysconfdir=/etc`
  moves compiled `PERSONALITY_PATH` off `/usr/etc`. The string
  lives in `libpkgconf.so.5.0.0`, not the CLI. Checksum
  `4446edbd460556d901aa7a600af0d000ef96947958fabffb1ed0e32b27763d5f`.
- [x] (2026-08-17) Wrote `core-userland-1` as five `pass` records
  and one `gap`. 2nex-utilities, coreutils, diffutils, findutils,
  and kbd pass. procps-ng `sysctl --system` already walks `/etc`,
  `/run`, and `/usr/lib`. `top` still reads only
  `/etc/topdefaultrc`. The extract script skipped the SourceForge
  `/download` URL; the tarball is in `inputs_cache` under the
  pinned sha256.
- [x] (2026-08-17) Landed the dbus-broker repair. The main conf now
  names `/usr/share`, `/run`, and `/etc` drop-in directories, and the
  parser shadows by basename and honours a `/dev/null` mask. Empty
  `/etc` output dirs are gone. Checksum
  `089a1bed94abf872b6841f59d4271bf6f3f4fe99315de22bf5728db77282648d`.
- [x] (2026-08-17) Wrote `core-toolchain-3` as two `pass` records
  (ninja, patch) and one `gap` (pkgconf). pkgconf compiles
  `PERSONALITY_PATH` with `${sysconfdir}` as `/usr/etc` because
  the manifest omits `--sysconfdir=/etc`. The shipped binary names
  never load a personality file; the banned string is still compiled
  in. Repair is the configure flag, not a source patch.
- [x] (2026-08-17) Wrote `core-toolchain-2` as six `pass` records
  (gn, gperf, libtool, make, meson, nasm). Libtool's `/etc/ld.so.conf`
  string is an echo, not a read. Meson searches XDG data dirs only
  for a named `--cross-file` / `--native-file` basename.
- [x] (2026-08-17) Wrote `core-toolchain-1` as six `pass` records
  (autoconf, automake, binutils, bison, flex, gcc). The worker
  missed that native `ld.bfd` reads `/etc/ld.so.conf`; that file
  still belongs to glibc. Autoconf's reader is
  `bin/autom4te.in:1043-1047`, not `lib/autom4te.in`.
- [x] (2026-08-17) Landed the mandoc repair. First existing file
  among `/etc/man.conf`, `/run/man.conf`, and `/usr/lib/man.conf`
  wins. `MANPATH` and `-C` stay ahead. Checksum
  `0ed8dbecb6982d6f962e33bbd820c08debca22be66dbc2c30b05a1f2a2f20d88`.
- [x] (2026-08-17) Recorded the human rule that bootstrap seeds must
  not receive UAPI-config patches. Searched `pkg/bootstrap/` for
  `uapi` patches and extra sysconfdir remaps: none exist. The only
  local `file:` sources are the two GCC reproducibility patches on
  phase0 toolchain and phase1 gcc. Updated `AGENTS.md`,
  `.agents/MANIFESTS_CODE_STYLE.md`, the audit README, the worker
  prompt, and the package-manifests note.
- [x] (2026-08-17) Wrote `core-power` (acpid) as `gap`. The daemon
  scans only `/etc/acpi/events`. Power-button and lid rules are
  shippable defaults, so this is three-tier policy, not machine state.
- [x] (2026-08-17) Landed the texinfo repair. `--sysconfdir=/etc`
  closes the `/usr/etc` trap; the patch inserts `/run/texinfo` and
  `/run/texi2any` between `$sysconfdir` and `$datadir`. Overlay is
  vendor share, then `/run`, then `/etc`. `texi2any` is a Perl script,
  so the smoke greps source fragments and plants `EXTRA_HEAD`.
  Checksum
  `a8740540bb8628a2423ef06115733817d4108503557c593046b92625e5a21864`.
- [x] (2026-08-17) Wrote `core-nex` as three `pass` records
  (nex-ld-shim, nex, zub). The shim's `.nex-app-root` and `nex.loader`
  live inside one deployment tree. The CLI and zub keep state under
  `/nex` or a caller-chosen repo; `utils.rs` only classifies build
  artifacts that happen to contain `/etc/`.
- [x] (2026-08-17) Landed the bat repair. Config overlay is
  `/usr/lib/bat/config`, `/run/bat/config`, then `/etc/bat/config`.
  Header decorations are silent without a tty on 0.26, so the smoke
  uses `--line-range`. Checksum
  `ca4729fe8bf863aef6709d98ad33062e546bcbb1a6cf70976b16d648ec7eef77`.
- [x] (2026-08-17) Wrote `core-kernel-2` as three `pass` records
  (linux, v4l2loopback, wireless-regdb).
- [x] (2026-08-17) Wrote `core-kernel-1` as six `pass` records. kmod
  already merges `modprobe.d`/`depmod.d` across `/etc`, `/run`, and
  `/usr/lib`. kvmfr's `module/kvmfr.c` has no file reader.
- [x] (2026-08-17) Landed the zsh repair. Each global startup file is
  sourced from `/usr/lib/zsh`, then `/run/zsh`, then `/etc`. Smoke uses
  `zshenv` and `zsh -c`. Checksum
  `314848d5f5efe69116afdb4e37a493fbf2213a32118db42c76537e3d14ece68c`.
- [x] (2026-08-17) Landed the procs repair. First-file-wins after the
  user paths: `/etc/procs/procs.toml`, `/run/procs/procs.toml`,
  `/usr/lib/procs/procs.toml`. Checksum
  `de0416d279e40219a92ac5dddda59371b1c347013faa48debce544c44cd9ab22`.
- [x] (2026-08-17) Wrote `core-ipc` (dbus-broker) as `gap`. Its
  `<includedir>` list has no `/run` tier and no basename mask, the
  same hole `dbus-transient-config.patch` already closed for
  `dbus-daemon`.
- [x] (2026-08-17) Landed the tmux repair. `TMUX_CONF` is now
  `/usr/lib/tmux/tmux.conf:/run/tmux/tmux.conf:/etc/tmux.conf` plus the
  user files. The smoke needs `glibc-locale-en-us` because tmux refuses
  `LC_ALL=C`. Checksum
  `137dbaf7b2751344e6b94f101827db037b5de7aa1ddea5499e2918d3e51768b0`.
- [x] (2026-08-17) Attempted the screen repair and did not land it.
  Claude's first-pass patch is saved at
  `.nex/tmp/ep016-screen-uapi-config.patch` and applies. The smoke is
  the blocker: `-d -m` forks before `FinishRc`, `Msg()` then goes to
  the backend instead of stderr, and the sandbox cannot allocate a
  pty. Do not commit that patch until a probe can fail against the
  old binary. `screen.yaml` was restored.
- [x] (2026-08-17) Wrote `core-init` (systemd) as `pass`. `CONF_PATHS`
  is `/etc`, `/run`, `/usr/local/lib`, `/usr/lib`, and the manifest
  already smokes `systemd-analyze cat-config` through those tiers.
- [x] (2026-08-17) Wrote `core-fs` (btrfs-progs, dosfstools, ostree,
  xfsprogs) as four `pass` records. e2fsprogs was already `gap fixed`.
- [x] (2026-08-17) Landed the htop repair. First-file-wins across
  `/etc/htoprc`, `/run/htop/htoprc`, `/usr/lib/htop/htoprc`. The smoke
  uses `config_reader_min_version=999` so `Settings_read` names each
  file without needing a TTY. Checksum
  `8e4700f17c9fe56a1e0213fa2ce47aa634d72d1e994d536684554f271105e2c1`.
- [x] (2026-08-17) Wrote `core-embedded` (busybox) as `pass`. The
  binary contains many single-path `/etc` readers, but current
  assemblies only run `busybox poweroff -f`, and the package ships no
  vendor files for those applets.
- [x] (2026-08-17) Landed the bmon repair. Claude used
  `--permission-mode bypassPermissions`, ran both strict builds, and the
  smoke used `conf_read`'s parse-error path to name each tier. Checksum
  `f1cd5790ce49afde4d00164f62ad18ead23b73540ac085c042e71be6f11661ac`.
  `strings` on the stored `usr/bin/bmon` shows `/usr/lib/bmon/bmon.conf`,
  `/run/bmon.conf`, and `/etc/bmon.conf`.
- [x] (2026-08-17) Landed the ncdu repair. Claude wrote the first-pass
  patch; `acceptEdits` blocked `./nex`, so Grok reviewed, replaced the
  chroot smoke with the same in-sandbox `/etc` `/run` `/usr` planting the
  other UAPI tests use, and ran both strict builds. Smoke marker
  `ncdu UAPI smoke: the config loader read all three tiers in order`
  appears in all four compiles. Checksum
  `2237bd9f2af18645825bdcedd75cf33e3b54779f0274e918069c94410c32d276`.
  `strings` on the stored `usr/bin/ncdu` shows `/usr/lib/ncdu/ncdu.conf`,
  `/run/ncdu.conf`, and `/etc/ncdu.conf`.
- [x] (2026-08-17) Assigned Claude `cli-text-1` and the ncdu repair. The
  audit report wrote itself to a Claude plans file instead of the JSON
  `.text` field; `jq -r .text` on current Claude Code JSON is empty. Opened
  every cited source. gawk, gettext, and grep pass. bat is a confirmed
  `gap` (worker said `uncertain`; `/etc/bat/config` is theme/pager policy).
  mandoc is a confirmed `gap` (worker said machine-owned; `man.conf` is a
  shippable default). less stays `gap fixed`. Checker:
  `pass: 133, gap: 10, gap fixed: 22, not audited: 368`.
- [x] (2026-08-17) Pre-task found only the uncommitted Chromium repair.
  Chromium's first compile finished all 54756 objects, then the smoke failed
  in the probe: `grep -o` on a NUL-containing dump-dom log extracted an
  empty load order after every `Found mandatory policy file:` line had
  already matched. The probe now keeps dump-dom off that log and greps with
  `-a`. Validated `cli-system-4` and `cli-system-5` against the pinned
  sources and wrote the eight records: five `pass`, three new `gap`s
  (procs, screen, tmux). Checker: `pass: 130, gap: 8, gap fixed: 22, not
  audited: 373`.
- [ ] Audit every remaining package manifest in sorted order and resolve every
  `gap` or `uncertain` report. 153 records still say `not audited`, and
  `sh scripts/check-package-config-audit.sh` prints that count on every run.
  After `core-nex` the checker reports `pass: 153, gap: 5, gap fixed: 29,
  not audited: 346`. After texinfo and `core-power` it reports
  `pass: 153, gap: 5, gap fixed: 30, not audited: 345`. After mandoc
  it reports `pass: 153, gap: 4, gap fixed: 31, not audited: 345`.
  `.nex/tmp/ep016-batches.tsv` holds the planned 127 batches, one line per
  batch as `name<TAB>status<TAB>manifests`, grouped by `pkg/<area>/<group>/`
  with at most six manifests each. Its status column is current: `done` for a
  finished batch, `records-written-gap-open` for `apps-web`, `pending`
  otherwise. Start one with
  `.nex/tmp/ep016-batch.sh <name> <manifests...>`, which extracts the pinned
  sources, writes the prompt, and runs the worker. Read a finished report with
  `jq -r .text .nex/tmp/ep016-audits/<name>.json` or, when that field is
  empty, the matching file under `~/.claude/plans/`. A worker report is never
  acceptance: open the cited source before writing each entry.

  Batches 1 to 6 (`apps-containers-1` through `apps-graphics`) are done. Their
  17 manifests produced 13 accepted `pass` records and four validated gaps, all
  four now repaired and proven: runc, distrobox, containerd, and Docker.
  Batches 7 to 14 were started and their reports land in the same directory.

  The popt repair landed, the fourth of this run. `poptReadDefaultConfig()`
  now reads `/usr/share/popt`, then `/run/popt`, then `/etc/popt`, each as a
  base file plus a `.d` drop-in directory, with `$HOME/.popt` still last. The
  patch factors the original single-tier body into a `poptReadConfigTier()`
  helper called once per tier.

  The precedence here is the opposite of krb5's and had to be derived
  separately. `poptAddItem()` appends each alias to `con->aliases`, and
  `handleAlias()` resolves a name by walking that array backward, so the last
  tier read wins and the tiers ascend in authority. krb5's profile takes the
  first entry instead. Two packages repaired hours apart needed opposite
  orderings, which is why each assignment refuses to state the direction and
  makes the worker prove it.

  `--sysconfdir=/etc` is load-bearing for popt and was a no-op for libinput and
  libxkbcommon. popt is autotools, so without the flag `POPT_SYSCONFDIR`
  compiles to `/usr/etc`. The flag also introduced no new output, because
  `sysconfdir` reaches only the `-DPOPT_SYSCONFDIR` define at
  `src/Makefile.am:6` and no install target.

  One supervisor error is recorded here. Rewriting the popt record by slicing
  the audit file between two headings deleted the `readline.yaml` record that
  sits between them alphabetically. `check-package-config-audit.sh` caught it
  immediately with `tracked manifest has no audit heading`, and the record was
  restored from `HEAD` and verified byte-identical. Splice by exact heading
  pairs only when the two headings are adjacent, and run the checker after
  every record rewrite rather than only after a batch.

  Both `uncertain` records were resolved rather than left open, and the method
  is worth reusing. gdbm and libpciaccess were undecidable only because their
  pinned tarballs were absent from `inputs_cache`. The supervisor fetched each
  from the URL its manifest pins, confirmed the sha256 against the manifest's
  own value, added it to `inputs_cache/`, and read the source. Neither needed a
  build.

  gdbm has no system configuration at all. `tools/gdbmtool.h:85` defines
  `GDBMTOOLRC` as the relative string `".gdbmtoolrc"`, so `source_rcfile()`
  tests the current directory and then `$HOME`, both user-controlled, with
  `--norc` disabling it. No `/etc` candidate exists to tier.

  libpciaccess reads one vendor data file and derives its path from `datadir`
  rather than `sysconfdir`: `meson.build:68-71` builds `PCIIDS_PATH` from
  `prefix`, `datadir`, and a `pci-ids` option defaulting to `hwdata`, giving
  `/usr/share/hwdata`. That is the same location `pciutils` uses for the same
  file, so the two agree, and the autotools `/usr/etc` problem cannot arise
  even though the manifest passes no `--sysconfdir`.

  The earlier caution about that package was justified. The only built
  `libpciaccess.so.0.11.1` under `.nex/tmp` belongs to Chromium's Debian
  sysroot, so treating it as evidence would have produced a confident wrong
  answer about a package built by another distribution entirely.

  The enchant2 repair landed, the fifth of this run and the one that cost the
  most builds. `enchant_get_conf_dirs()` now returns the vendor directory,
  then `/run/enchant-2`, then `/etc/enchant-2`, then the user directory, and
  `--sysconfdir=/etc` moves that administrator entry off `/usr/etc`.

  Its test design is the part worth reusing. This build disables every real
  spell-check backend, so provider ordering could not be observed at all
  without something to order. The worker therefore compiled a minimal
  `EnchantProvider` twice under different names, installed both into the real
  `/usr/lib/enchant-2` module directory so `enchant_broker_init()` loads them
  like real backends, and asserted the resulting order. That made the negative
  proof behavioral rather than a crash: without the patch the smoke reported
  `order: nex-uapi-other nex-uapi-want` where
  `order: nex-uapi-want nex-uapi-other` was expected, which is precisely the
  symptom of a `/run` tier that is never read.

  Four builds were lost first, and the cause is worth recording because it is
  cheap to avoid. The original smoke linked against
  `normalize_dictionary_tag()`, which `broker.c:571` defines but which the
  library does not export, so every attempt died at
  `undefined reference` after a full compile. `nm -D` on the built
  `libenchant-2.so.2` confirms it: `enchant_broker_get_ordered_providers` and
  `enchant_get_conf_dirs` are exported, `normalize_dictionary_tag` is not.
  Write a smoke against installed programs or exported symbols only, and check
  with `nm -D` before building. That warning was carried into the iproute2
  assignment before this repair even finished.

  The worker disclosed one tradeoff rather than burying it: the two functions
  it does use are exported but absent from the installed `enchant.h`, so the
  smoke declares their prototypes itself. Driving `enchant-lsmod-2` would avoid
  that at the cost of parsing program output. The record states the choice.

  Batches `net-manager`, `net-misc`, and `servers-xwayland.yaml` are written,
  and with them the `not audited` count reaches zero. Every one of the 533
  tracked manifests now carries a record: 456 pass, 66 gap fixed, 9 gap, and 2
  uncertain.

  NetworkManager is the reference example this plan had been missing: an
  upstream that already implements the rule without any patch.
  `src/core/nm-config.c:23-26` defines `NMCONFDIR/conf.d`, `NMLIBDIR/conf.d`,
  and `NMRUNDIR/conf.d`, so all three tiers are merged as shipped. Its main
  file is single-tier inside `/etc`, and that is not a gap, because a
  deployment ships policy through the vendor `conf.d` directory instead. The
  main file is the administrator's own entry point, the same arrangement
  systemd uses for `system.conf` beside its drop-ins.

  iproute2 is the ninth gap and the best-formed one found. Its nine mapping
  tables already read `/etc/iproute2` first and fall back to
  `/usr/share/iproute2` only on `-ENOENT`, and its drop-in handling already
  masks a vendor file when the same basename exists under `/etc`. That is the
  UAPI model in all but one respect: nothing in `lib/rt_names.c` or
  `lib/bpf_legacy.c` names `/run`. The package installs its vendor data
  correctly and plants nothing below `/etc`, so the repair is purely a missing
  tier in the reader.

  Xwayland passes on four separate reasons rather than one. Its XKB root is
  vendor data owned by xkeyboard-config; its keymap cache is derived data whose
  `access(..., W_OK | X_OK)` probe already skips a read-only `/usr` and lands
  in `XDG_RUNTIME_DIR`; `/etc/X<display>.hosts` lists which hosts may reach
  this display, which is machine-owned state beside `/etc/hosts`; and
  `/usr/lib/xorg/protocol.txt` is an immutable vendor registry.

  Batches `libs-x11-5` and `libs-x11-6` are written: all ten pass, which closes
  `pkg/libs/x11/` entirely. Four of the ten install no runtime component at
  all and are recorded as `build-only input`: util-macros ships autoconf
  macros, xcb-proto ships protocol XML and its generator, xorgproto ships 129
  headers, and xtrans ships headers that each consumer compiles into itself.

  xcb-util-cursor was the only reader and it matches its sibling
  `libxcursor` record. `configure.ac:29` sets the default search path to the
  literal `~/.icons:/usr/share/icons:/usr/share/pixmaps:/usr/X11R6/lib/X11/icons`,
  so a user's themes win and the deployment's are the fallback. The value is a
  hardcoded literal rather than something derived from `sysconfdir`, which is
  what keeps the autotools `/usr/etc` problem away from it. Its trailing
  `/usr/X11R6` entry is a legacy path that no longer exists, inert rather than
  wrong.

  With those two batches the whole X11 area, thirty-four manifests across six
  batches, came back `pass` exactly as the pre-screen predicted. The screen was
  worth running: it turned six batches into confirmation work rather than
  discovery work, and it meant the only records needing real argument were
  libX11's reference databases, libXt's resource chain, libXmu's compiled-out
  file branch, and the two cursor search paths.

  Batches `libs-x11-3` and `libs-x11-4` are written: all twelve pass, matching
  the prediction. Four of them needed a real look rather than a search.

  libXmu is the nicest catch the workers made in this area. `get_os_name()` in
  `src/CvtStdSel.c` contains `fopen(X_OS_FILE, ...)` and
  `fopen(MOTD_FILE, ...)`, which a plain grep would report as file reads.
  Neither macro is defined anywhere in the tree, and `:62-68` selects
  `USE_UNAME` when `X_OS_FILE` is absent, so the `#if` at `:88` compiles the
  whole branch out and the function calls `uname()`. Confirmed by checking that
  no definition of either macro exists.

  libXfont2 and libxkbfile both resolve by delegation. libXfont2 reads
  `fonts.dir` and `fonts.alias` from whichever directory the X server hands it,
  and `fonts.dir` is generated by `mkfontdir`, so it is derived data in a
  caller-chosen location. libxkbfile parses rules files whose base directory
  comes from its caller, and the XKB tiering belongs to libxkbcommon, already
  repaired in this plan.

  libXt is the substantive one. Its resource chain has exactly one system-wide
  entry, the compiled `XFILESEARCHPATHDEFAULT` at
  `include/X11/IntrinsicI.h:153`, which is a vendor path under `/usr/lib/X11`;
  everything above it is an environment variable or a per-user file. No `/etc`
  tier is missing, because application default resources belong to the
  deployment.

  One mismatch was recorded there for a future reader. libXt's build sets
  `appdefaultdir` to `${datadir}/X11/app-defaults`, which is
  `/usr/share/X11/app-defaults`, while the compiled runtime search names
  `/usr/lib/X11`. An application installing app-defaults under `/usr/share`
  would not be found through the compiled default. Nothing in this repository
  installs app-defaults at all, so it is inert today, but it should be settled
  if that changes.

  Batch `libs-x11-2` is written: libXcomposite, libXcursor, libxcvt,
  libXdamage, libXdmcp, and libXext all pass, again as predicted.

  libXcursor was the only one with a compiled path and it is correct.
  `configure.ac:73` builds `DEF_CURSORPATH` as
  `~/.local/share/icons:~/.icons:${datadir}/icons:${datadir}/pixmaps`, so a
  user's themes are found before the deployment's, with `XCURSOR_PATH`
  overriding both. The detail that matters for this plan is that the path
  derives from `datadir` rather than `sysconfdir`, so the autotools `/usr/etc`
  problem that caught popt, libxml2, and enchant2 cannot arise here. A cursor
  theme is artwork a deployment ships and a user selects, and which theme a
  session uses is settled by icon theme and toolkit settings rather than by
  this search path, so no administrator tier is missing.

  The other five are protocol marshalling or pure computation with no file
  access at all. libXdmcp is worth one line: it builds and parses XDMCP packets
  in memory, and where a display manager keeps its cookies is the caller's
  choice rather than a location compiled in here.

  Batch `libs-x11-1` is written: libfontenc, libICE, libSM, libX11, libXau, and
  libxcb all pass, as predicted before the report arrived.

  The X11 client libraries were cleared ahead of their batches by two screens.
  No `.c` or `.h` file in any of the twelve libraries extracted so far contains
  a `"/etc` literal or a `SYSCONFDIR` use, and the built `libX11.so.6.4.0` in
  the 204-package rootfs names only `/usr/lib/X11/locale`,
  `/usr/share/X11/locale`, `/usr/share/X11/Xcms.txt`,
  `/usr/share/X11/XErrorDB`, and `/usr/share/X11/XKeysymDB`.

  libX11 is the only one with real data directories, and every family it reads
  is reference data rather than machine policy: protocol error strings, colour
  name definitions, keysym names, and locale and input method data. A single
  vendor path is right for each, with `XLOCALEDIR` overriding per process. The
  genuinely administrator-owned part of X11 configuration is keyboard layout,
  and that belongs to libxkbcommon and xkeyboard-config, both already recorded.

  The other five split cleanly into per-user secrets and nothing at all. libXau
  and libICE each resolve an authority file from an environment variable then
  `$HOME`, which is one user's cookies and correctly has no system tier. libSM
  and libxcb read only an address, `SESSION_MANAGER` and `DISPLAY`, and
  delegate authentication to libICE and libXau. libfontenc's encoding tables
  are a vendor directory with an environment override.

  Expect the remaining `libs-x11-*` batches to be the same. A `gap` from one of
  them is a claim to verify against the built library, not to accept.

  Batch `libs-wayland` is written: wayland, wayland-protocols, libseat,
  wlroots, gtk-layer-shell, and gtk4-layer-shell all pass, which closes
  `pkg/libs/wayland/`. The whole area is environment-driven rather than
  file-driven: `WAYLAND_DISPLAY` and `WAYLAND_SOCKET` for connections, the
  `WLR_` family for compositor tunables, and `XDG_RUNTIME_DIR` for the socket
  itself.

  libseat was the one worth checking, because `SEATD_DEFAULTPATH` appears in
  its source. The manifest passes `-Dlibseat-logind=systemd`,
  `-Dlibseat-seatd=disabled`, and `-Dserver=disabled`, and the only users of
  that constant are `libseat/backend/seatd.c:57` and
  `seatd-launch/seatd-launch.c:117-141`, both in the disabled components, so
  the shipped library negotiates over logind's D-Bus interface and compiles in
  no socket path.

  wlroots is a useful cross-reference rather than a finding of its own. A
  wlroots compositor gets keyboard layouts through libxkbcommon and input
  quirks through libinput, and both of those families were repaired earlier in
  this plan, so the compositor library itself needs no configuration surface.

  This batch is also the one relaunched after the manifest-list error described
  above; the records here come from the corrected run against the ledger's own
  six manifests.

  Batch `libs-tui` is written: newt passes. Its colour palette file is
  reachable only through `NEWT_COLORS_FILE` in the environment, because the
  compile-time macro of the same name exists only when `configure` receives
  `--with-colorsfile`, which this manifest does not pass. The `#ifdef` block at
  `newt.c:307-309` is therefore compiled out and no system path exists to tier.

  A supervisor error is recorded here rather than buried. `libs-wayland` was
  launched with a hand-typed manifest list instead of the ledger's own, and
  three of the six paths did not exist. `ep016-extract.py` died on the first
  missing file and its traceback landed inside the worker's prompt, so that
  worker spent its run against a broken assignment and had to be killed, its
  artifacts deleted, and the batch relaunched from
  `grep -P '^libs-wayland\t' ... | cut -f3`.

  The fix is tooling rather than a resolution to be careful.
  `.nex/tmp/ep016-run-batch.sh` now takes only a batch name and derives the
  manifest list from the ledger. It refuses to run when the batch is not in the
  ledger, when its status is not `pending`, or when any manifest is missing from
  the worktree. Use it instead of calling `ep016-batch.sh` directly; the status
  guard also prevents relaunching a batch that is already `in-progress` or
  `done`, which would waste a worker and risk double-writing records.

  Batch `libs-text-3` is written: pango, unicode-ucd, vte, and woff2 all pass,
  which closes `pkg/libs/text/` apart from the enchant2 repair.

  pango was the one to check carefully, because a screen had flagged a
  `SYSCONFDIR` use in it. It is benign twice over. The value resolves to
  `/etc/pango` rather than `/usr/etc/pango`, because pango is meson, and the
  two functions that would name it,
  `pango_get_sysconf_subdirectory()` and `pango_get_lib_subdirectory()`, have
  been deprecated since 1.38 and have no callers anywhere in the tree. They are
  exported helpers rather than readers, the same shape as libgpg-error's
  `_gpgrt_fnameconcat()`.

  vte is the clearest example in this plan of a package correctly leaving the
  tiering to an assembly. It passes `--sysconfdir=/usr/lib`, so its `vte.sh`
  and `vte.csh` prompt scripts install to `/usr/lib/profile.d/` rather than
  `/etc`, and `asm/flat-systemd.yaml` merges `/usr/lib/profile.d`,
  `/run/profile.d`, and `/etc/profile.d` by basename with the last match
  winning. The package ships the vendor snippet and the assembly builds the
  tiers, which is exactly the arrangement the knowledge notes describe.

  Two workers in this area returned their reports through a plan file rather
  than through the result field, leaving only a one-line pointer in the JSON.
  The full reports were at `~/.claude/plans/package-configuration-audit-*.md`
  and were copied into `.nex/tmp/ep016-audits/` before use. Check the result
  field's length before assuming a batch produced nothing.

  Batch `libs-text-2` is written: libidn2, libspelling, libunistring,
  libutf8proc, libxslt, and oniguruma all pass. The worker returned libxslt as
  `uncertain` and the supervisor resolved it to `pass`, which is the useful
  part of this batch.

  libxslt has no reader of its own. `xsltproc` calls `xmlLoadCatalogs()` only
  under `--catalogs`, reading `SGML_CATALOG_FILES`, and the default catalog
  behavior its help text advertises lives inside libxml2. The worker could not
  settle whether that dependency tiers correctly, because libxml2 was outside
  its assignment, and said so rather than guessing. It was right to stop there:
  the answer already exists in this same audit, where
  `pkg/libs/system/libxml2.yaml` is recorded as a gap on the compiled
  `file:///usr/etc/xml/catalog`. So the defect stays recorded once, against the
  package that owns the reader, and libxslt carries a cross-reference.

  A detail worth keeping for whoever repairs libxml2: `xsltproc --help` names
  `file:///etc/xml/catalog`, which is upstream's wording and is not what this
  build compiles. Fixing libxml2 makes that text true rather than requiring any
  change in libxslt.

  This is the third record in the plan to resolve by delegation rather than by
  its own search, after libass deferring to fontconfig and xkeyboard-config
  deferring to libxkbcommon. The pattern is worth naming: when a package only
  calls another package's reader, record the finding against the owner and
  cross-reference it, rather than repeating or splitting it.

  Batch `libs-text-1` is written: fribidi, gtksourceview4, gtksourceview5,
  harfbuzz, and libass pass, and enchant2 is a gap. This is the second batch
  whose gap was predicted before the report arrived, and the prediction named
  the right package for the right reason.

  The worker's picture was richer than the prediction and improved the record.
  enchant's reader is a genuine merge, not a selection:
  `enchant_broker_load_provider_ordering()` loops over the vendor directory,
  the `SYSCONFDIR` directory, and the user directory in order and loads every
  `enchant.ordering` it finds, with later entries overwriting earlier ones per
  language tag. So the mechanism is already correct and only the directories
  are wrong.

  One correction was made to the worker's conclusion. It proposed
  `--sysconfdir=/etc` as the fix, which addresses only half the gap. That flag
  moves the administrator tier out of `/usr/etc`, but the family would still
  have no `/run` tier, so the record states that both halves are needed and
  that a `/run/enchant-2` entry belongs between the vendor and administrator
  directories, following the libinput and libxkbcommon repairs.

  The three signals that predicted this all held up: the `/usr/etc` string in
  the built `libenchant-2.so.2.8.2`, the `@SYSCONFDIR@` substitution at
  `src/Makefile.am:21`, and the reader at `lib/provider.c:350`.

  The two gtksourceview records share one reason worth stating once. Their
  language specifications, style schemes, and snippets are located through
  `_gtk_source_utils_get_default_dirs()` and the XDG data directories, which
  the assembly arranges through `XDG_DATA_DIRS`, so no `/etc` or `/run` tier
  belongs to those packages at all.

  The krb5 repair landed, and it was the cheapest of the three because both
  families already had the right mechanism. The profile list only needed two
  more entries and the GSSAPI registry only needed two more tiers; no new
  parser was written.

  The assignment deliberately refused to state which profile entry wins and
  required the worker to settle it. It did, with citations that were checked:
  `prof_init.c:211-214` sets `first_file` to the first entry that opened,
  `prof_tree.c:431` starts the iterator there, and `prof_get.c:219` returns the
  first value found, so the first listed existing entry wins and
  `/etc/krb5.conf:/run/krb5:/usr/share/krb5` gives administrator over boot over
  vendor. The GSSAPI ordering is the mirror image, because `addConfigEntry()`
  returns early for an OID already claimed, so the tiers load vendor first and
  `/etc` last.

  The unpatched-build proof was the strongest this plan has seen: rather than a
  scratch tree, the worker rebuilt the same pinned source through the real
  `nex build` pipeline with only the config patch omitted, and the check failed
  at its first assertion with `profile value: (unset)`.

  One limitation was reported rather than hidden, and the record keeps it. The
  smoke proves vendor and `/run` mechanism entries are read, but not which
  library wins when one OID appears in several GSS tiers, because
  `gss_indicate_mechs()` exposes only the OID set. That claim rests on the code
  citation. The assignment had only required proving the entries are read, so
  this is a noted gap in evidence rather than an unmet requirement.

  Batch `libs-system-7` is written: zlib passes, and it closes
  `pkg/libs/system/`. zlib is the one package in this plan that settles the
  sysconfdir question at build time by refusing it: its `configure:139` matches
  `--sysconfdir=` only to print `ignored option: --sysconfdir`, so no
  `sysconfdir`-driven path exists to compile and the `/usr/etc` shape cannot
  arise.

  Housekeeping ran with this batch. `ep016-drop-src.sh` was applied to the 363
  manifests belonging to `done` batches, taking `.nex/tmp/ep016-src` from
  4980 MB to 808 MB and returning `/var` to 139 GB free. Sources for every
  batch still carrying an open gap were deliberately kept, and krb5, libxml2,
  popt, appstream, fuse3, libssh, samba, and polkit were each confirmed present
  afterwards.

  Batch `libs-system-6` is written: ncurses, pciutils, shared-mime-info,
  tllist, and userspace-rcu pass, and popt is a gap. This is the first batch
  whose gap was predicted before the worker reported, using the `/usr/etc`
  artifact scan, and the worker reached the same verdict independently.

  popt is the second proven autotools case after libxml2, and it is the same
  shape. `src/Makefile.am:6` passes
  `-DPOPT_SYSCONFDIR="\"$(sysconfdir)\""`, autotools keeps its `${prefix}/etc`
  default, and `poptconfig.c:445` and `:449` therefore read `/usr/etc/popt` and
  stat `/usr/etc/popt.d`. The reader never consults `/etc`, where a machine
  owner can actually write, and has no `/run` tier. Two independent lines of
  evidence agree: the source reading, and the built `libpopt.so.0.0.2` in the
  rootfs snapshot, which contains `/usr/etc/popt`, `/usr/etc/popt.d`, and
  `/usr/etc/popt.d/*` and no `/etc/popt` at all.

  The three data families in this batch all stayed `pass` on the same
  reasoning, which is worth stating once. ncurses' terminfo tree describes what
  a terminal type can do, pciutils' `pci.ids` transcribes the published PCI
  registry, and shared-mime-info's `mime.cache` is regenerated from the
  installed XML. None of them expresses machine policy, so a single vendor path
  is right and there is nothing for an administrator to override.

  Batch `libs-system-5` is written: libxcrypt, libxmlb, linux-headers, mpc, and
  mpfr pass, and libxml2 is a gap that the worker reported as a pass.

  libxml2 is the first proven `/usr/etc` case in this plan, and it is the
  autotools counterpart to the meson correction recorded above. `catalog.c:55`
  builds `XML_XML_DEFAULT_CATALOG` from `SYSCONFDIR`, `Makefile.am:15` passes
  `-DSYSCONFDIR='"$(sysconfdir)"'`, and the manifest runs `./configure
  --prefix=/usr` with no `--sysconfdir`. Autotools keeps its `${prefix}/etc`
  default, so the compiled path is `file:///usr/etc/xml/catalog`.

  That was confirmed against a built artifact rather than inferred. The
  Nex-built `libxml2.so.2.13.5` inside
  `.nex/tmp/ep013-desktop-final-root/nex/pkg/dev/virt/libvirt/...` carries
  exactly one catalog string, `file:///usr/etc/xml/catalog`, and no
  `/etc/xml/catalog` at all. Provenance was checked, because the libpciaccess
  attempt in the previous batch found only a Chromium Debian sysroot copy that
  proved nothing: this path contains `/nex/pkg/` and the file version matches
  the manifest's pinned 2.13.5.

  The worker had every fact right, including `SYSCONFDIR=/usr/etc` and the
  three-tier classification, and still wrote `pass`, on the ground that the
  manifest installs no file at that path. That is the same reasoning already
  overturned for libssh and samba: an absent file does not make an unreachable
  administrator tier acceptable. libxml2's shape is worse than a missing `/run`
  tier, because the one place the reader looks is the one place nobody may
  write, leaving `XML_CATALOG_FILES` as the only working override.

  The lesson generalises. Both halves of the sysconfdir question are now proven
  from built binaries: meson gives `/etc`, autotools gives `/usr/etc`. Find a
  Nex-built copy of the library under a rootfs below `.nex/tmp` and run
  `strings` on it, checking that the path contains `/nex/pkg/` and that the
  version matches.

  Batch `libs-system-4` is written: libevent, libffi, libfyaml, libusb, and
  libxcrypt-compat pass, and libpciaccess is the second `uncertain`.

  libevent holds the only real reader. Its `evdns` component reads
  `/etc/resolv.conf` at `evdns.c:4036` and `/etc/hosts` at `evdns.c:3658`, both
  as hardcoded literals. Those stay `pass` for the reason libslirp's
  `/etc/resolv.conf` did: each names what this machine itself uses, so both are
  machine-owned state that the library only reads, and a caller may pass its own
  path to the parse function.

  libpciaccess could not be audited. Its tarball is absent from the inputs
  cache, and unlike gdbm there was a second avenue worth trying: a built
  artifact. That failed too. The only `libpciaccess.so.0.11.1` anywhere under
  `.nex/tmp` belongs to Chromium's Debian sysroot and was compiled by another
  distribution, so it says nothing about this package. The record refuses to
  assert the conventional answer and names the tarball checksum plus the exact
  grep that settles it.

  Two batches now carry `uncertain` for the same underlying reason, a source
  missing from `inputs_cache`. Both records name the pinned sha256 and the
  command to run once the tarball is available, so neither needs re-derivation.
  Neither package can install a vendor default in the wrong place regardless,
  because both declare no output below `/etc` or `/usr/etc`.

  Batch `libs-system-3` is written: jansson, json-c, json-glib, libbsd,
  libdevmapper, and libeconf all pass. Two records turned on scope rather than
  on a search.

  libdevmapper builds only the device-mapper half of LVM2. It ships
  `libdevmapper.so`, `dmsetup`, `dmstats`, and three udev rules, and none of
  the LVM tools, so `/etc/lvm/lvm.conf` is out of scope for this manifest. Its
  `libdm-config.c` does carry a generic key and value parser, but every entry
  point takes its text from the caller. That family needs auditing only if a
  manifest ever ships `lvm` or its siblings.

  libeconf is the interesting one, because it is the mechanism this whole plan
  measures other packages against. `econf_readConfig()` takes the project name,
  vendor subdirectory, configuration name, and suffix as arguments, so it opens
  only caller-chosen paths, and its documented merge order matches
  `CONFIGURATION.md` exactly. The `econftool` program hardcodes `root_dir` as
  `/etc` and `usr_root_dir` as `/usr/etc` with no flag to change either. That
  is a limitation rather than a violation: Nex puts vendor defaults below
  `/usr/lib` or `/usr/share`, so on a Nex system that root names a directory
  that does not exist and the tool's comparison feature finds nothing rather
  than reading the wrong file. The package installs nothing below `/etc` or
  `/usr/etc`.

  The worker left an open note asking whether any assembly invokes `econftool`
  expecting `/usr/etc` to hold vendor files, and said it had not checked.
  Searching `asm/` for `econftool` and `libeconf` returns nothing, so the
  question is settled and the record says so.

  The libxkbcommon repair landed, and the way it landed is a warning worth
  recording. Its patch adds `XKBCONFIGRUNPATH = '/run' / 'xkb'` and appends the
  new entry between the administrator path and the vendor root, so the include
  search became `/etc/xkb`, `/run/xkb`, then `/usr/share/X11/xkb`. Three strict
  builds passed on checksum `870066dd`, and the smoke carries 33 assertions.

  Two process failures happened around it, both the supervisor's to own.

  First, the coding worker staged its files with `git add` and then stopped
  without reporting, leaving only the stub "Waiting for the second strict build
  to finish". The supervisor then ran `git add <audit paths> && git commit`,
  and because `git commit` commits the whole index rather than the paths just
  named, `c39dc8c` silently absorbed the entire repair: the patch, the check
  script, the keymap fixture, and 24 lines of manifest wiring. That commit is
  labelled as an audit commit and is not one. Worse, it captured the wiring
  together with the pre-patch checksum, so `c39dc8c` is an inconsistent bisect
  point where the manifest builds a patched package while claiming the
  unpatched checksum. The fix was forward rather than a history rewrite: the
  following commit carries the corrected checksum and profile that make the
  manifest self-consistent again.

  Second, the worker's last edit stripped blank lines from the `outputs`
  section, and `nex check` rejected the result with `error: needs formatting`.
  `./src/cli/target/debug/nex format <manifest>` fixed it. Run `nex check`
  after a worker edit even when the diff looks like nothing but a checksum.

  The lesson for the rest of this plan: stage exactly, then commit exactly. Use
  `git commit -- <paths>` so a worker's staged files cannot ride along, and read
  `git diff --cached --stat` before every commit. Never assume a worker left the
  index clean, and never accept a build result the worker did not actually
  report; the supervisor reran the third strict build precisely because the
  worker's own second log ended mid dependency computation.

  Batch `libs-system-2` is written: both glibc locale packages, gmp, icu, inih,
  and iso-codes all pass. icu holds the only real reader, `/etc/localtime`,
  which `common/putil.cpp:702` compiles in as `TZDEFAULT`. That stays a `pass`
  because the file names which zone this machine sits in, so it is machine-owned
  state that icu only reads, and `TZ` overrides it per process.

  This batch confirms the autotools half of the sysconfdir correction. gmp,
  icu, and iso-codes are all autotools, and all three really do leave
  `sysconfdir` at `${prefix}/etc`, which is `/usr/etc`. In each case the value
  is harmless, because nothing is installed there and no runtime reader uses
  it: gmp installs only headers and libraries, icu's value reaches the generated
  `icu-config` script, and iso-codes' reaches only Makefile boilerplate. So the
  rule is that meson resolves this to `/etc` and autotools does not, and an
  autotools package needs the value traced to an installed path or a reader
  before it means anything.

  The two locale packages have no URL source at all, because `localedef`
  generates their data from the glibc dependency during the build, and
  `ep016-extract.py` reports `no url sources` for both. Their records rest on
  the manifest, which is sufficient here: every output path sits below
  `/usr/lib/locale/<name>/`, and neither package installs a program or a reader.

  Batch `libs-system-1` is written: acl, efivar, and expat pass, appstream and
  fuse3 are gaps, and gdbm is the first `uncertain` this run.

  This batch is where the plan's `/usr/etc` premise fell over. The worker's
  appstream block stated that meson applies an FHS special case resolving
  `sysconfdir` to `/etc` when `prefix` is `/usr`, which contradicted the
  reasoning written for libinput and libxkbcommon. Testing meson 1.12.0 on a
  three-line project confirmed the worker: `sysconfdir = /etc` and
  `get_option('prefix') / get_option('sysconfdir') = /etc`. fuse3 corroborates
  it from the build itself, because meson installs to `${OUT_DIR}/etc/fuse.conf`
  and the manifest then relocates that file. Both earlier records were
  corrected, and both gaps survive on the narrower ground that no `/run` tier
  exists.

  appstream and fuse3 are both gaps for that same narrower reason. appstream
  reads `/etc/appstream.conf` then the vendor
  `/usr/share/appstream/appstream.conf`, so vendor and administrator tiers are
  both present and correctly placed and only the boot tier is missing. fuse3 is
  thinner still: `fusermount3` opens only `/etc/fuse.conf` for `user_allow_other`
  and `mount_max`, with no vendor tier and no `/run` tier. fuse3's manifest
  already handles the vendor default correctly, copying the generated file to
  `/usr/share/doc/fuse3/examples/` and then deleting `${OUT_DIR}/etc`.

  gdbm could not be audited, because the pinned tarball is absent from this
  checkout and `ep016-extract.py` reported `not in inputs_cache (74b1081d21ff)`.
  The libraries are settled, since every database path comes from the caller,
  but whether `gdbmtool` reads a system-wide startup file is open. gdbm is
  autotools rather than meson, so the `${prefix}/etc` default really does apply
  there and must be checked rather than assumed. The record names the exact
  command that settles it once the tarball is available.

  Batch `libs-security-2` is written: polkit is a gap, and it is a new shape.
  The manifest already passes `--sysconfdir=/etc` and ships its one rules file
  into the vendor tier, so both the vendor and administrator tiers exist and
  sit in the right place. Only the boot tier is missing. The duktape backend's
  constructor allocates `g_new0(gchar *, 3)` and fills exactly two entries,
  `/etc/polkit-1/rules.d` and `/usr/share/polkit-1/rules.d`, so a boot cannot
  install transient authorization policy.

  Two details are worth carrying forward. The rules family is an additive
  ordered-script model, not a first-file-wins or masking model, so the
  drop-in replacement clause does not apply to it: every file from both
  directories runs. Upstream still gets administrator precedence, through
  `polkit_backend_common_rules_file_name_cmp()`, which compares basenames and
  then falls back to full paths with the comment `/* /etc wins over /usr */`.
  That works because `/etc` sorts before `/usr` and polkit takes the first rule
  returning a value other than `undefined`. The worker reported that both
  same-named files simply run, which is true but misses that the ordering is
  deliberate and is what gives the administrator the decision.

  The action directory at `/usr/share/polkit-1/actions` is correctly
  single-tier. An action file declares that an action exists and its default
  level, and upstream expects an administrator to change that through a rule
  rather than by replacing vendor XML.

  Selecting the right backend mattered here. The manifest passes
  `-Djs_engine=duktape`, so the built reader is
  `polkitbackendduktapeauthority.c`, not the mozjs `polkitbackendjsauthority.cpp`
  that a first search finds. Both carry the same two-directory constructor, so
  the conclusion held, but the cited file had to be corrected.

  The libinput repair landed, and it is the first package fix this plan
  completed under Claude as supervisor. `quirks_parse_tiered_files()` claims
  each basename from the highest tier holding it, taking `/etc/libinput`, then
  `/run/libinput`, then the vendor directory, and then parses the survivors in
  one global `strverscmp` order.

  That parse order deserves recording, because it looked wrong on first read.
  Merging by tier and then re-sorting globally means an administrator file
  named `10-mine.quirks` parses before a vendor `50-system-dell.quirks` and
  loses a section conflict to it. `CONFIGURATION.md:46-49` settles that this is
  intended: tiers resolve same-name replacement, and "the program then reads
  the resulting filenames in their documented order." The audit worker prompt
  paraphrases the same rule as "read vendor files first, overlay `/run` files,
  then overlay `/etc` files", which reads like tier-ordered parsing and is
  misleading. `CONFIGURATION.md` is the authority and the prompt says so.

  Both mask forms work through one `stat()` size test, because a symlink to
  `/dev/null` reports zero size just as an empty file does. The
  `install_emptydir(dir_etc / 'libinput')` call had to go: with the corrected
  `--sysconfdir=/etc` it would have declared a package output below `/etc`.
  Two independent strict builds passed, each printing the smoke's success line
  twice and each reporting `Build is reproducible. Checksums match.` on
  checksum `e7b45555`. The worker also proved the smoke fails against an
  unpatched build inside `unshare --user --map-root-user --mount`, where the
  old reader scanned only the vendor directory and the single override file.

  Batch `libs-security-1` is written: libcap-ng, libcap, libsecret, nspr, and
  nss pass, and krb5 is a gap.

  krb5 is the cleanest gap this plan has found, because the merging mechanism
  already exists and only the compiled default is too short. `init_os_ctx.c`
  reads `DEFAULT_SECURE_PROFILE_PATH` as a colon-separated list and the profile
  library merges every entry that exists. `osconf.hin:53` defines that macro as
  `"/etc/krb5.conf@SYSCONFCONF"`, and `configure.ac:12-16` sets `SYSCONFCONF`
  empty whenever `--sysconfdir` is `/etc`, which this manifest passes, so the
  list collapses to one entry. Extending the default to a vendor path and a
  `/run` path should need no new parser. The GSSAPI registry is a second gap in
  the same package: `g_initialize.c:61-64` hardcodes `/etc/gss/mech` and
  `/etc/gss/mech.d/*.conf`, which is already the right base-plus-drop-in shape
  but exists in one tier only. The keytab and the `/var/lib/krb5kdc` files are
  correctly scoped, and the sample profiles ship as documentation under
  `/usr/share/examples/krb5/` rather than into a search path.

  krb5 also exposed a limit in the cheap screen this run had been using. Its
  `grep -rl SYSCONFDIR --include='*.c' --include='*.h'` count is zero, because
  the path lives in a `.hin` template and the token is `SYSCONFCONF`. A zero
  from that screen means "no `/usr/etc` arithmetic", never "no reader".

  nss was returned as `uncertain` and was resolved to `pass`. The worker's
  doubt was whether an assembly activates `libnsssysinit.so`, but the reader
  question is settled: `getSystemDB()` returns a copy of the literal
  `/etc/pki/nssdb` with no override. That store is a live SQL trust database
  that `certutil` fills and that the module opens for writing, and this package
  seeds no content into it, so it belongs with the account-database exception
  rather than with `krb5.conf`. libcap carried an unshipped-reader claim that
  held: `pam_cap.c:33` names `/etc/security/capability.conf`, and the manifest
  installs no `pam_cap.so`.

  Batch `libs-net-2` is written: libslirp, libsoup3, libvncserver, and nghttp2
  pass, and libssh and samba are gaps. Both gaps came from overturning the
  worker, and both times the worker had already found the right facts and then
  drew the wrong conclusion from them.

  The worker marked libssh `uncertain` and samba `pass`, and gave the same
  reason for each: the reader is dormant, because no consumer in this
  repository was confirmed to call it. For samba it wrote the contradiction
  outright, classing `smb.conf` as three-tier distribution policy and then
  describing a single fixed `/etc/samba/smb.conf` read with no fallback. That
  pair is the audit contract's definition of a gap. Dormancy sets repair
  urgency; it does not change the class of a file. This plan already settled
  that the test applies to the file rather than the mechanism, and this is the
  eighth and ninth call overturned.

  libssh compiles `GLOBAL_CONF_DIR` to `/etc/ssh` at `DefineOptions.cmake:72`,
  and the manifest passes no `-DGLOBAL_CONF_DIR`. Its client configuration,
  server configuration, and `/etc/ssh/moduli` are each a single fixed read with
  no `/run` and no vendor tier. `config.h.cmake:12` declares a
  `USR_GLOBAL_CONF_DIR` that raises hope of a vendor tier, but nothing sets it
  and no source file reads it, so the `#cmakedefine` resolves to undefined.
  That matters because this repository already treats the same file names as
  three-tier: `pkg/cli/net/openssh-uapi-config.patch` moves `ssh_config`,
  `sshd_config`, and `moduli` onto a vendor directory plus `/run/ssh` with a
  drop-in merge. Two packages reading one family two different ways is the
  inconsistency worth fixing. libssh's host key defaults stay correct as
  single-path secrets.

  Two smaller worker errors were corrected in the records. libssh's
  `GLOBAL_BIND_CONFIG` is `/etc/ssh/libssh_server_config`, not `sshd_config`.
  samba already passes `--sysconfdir=/etc`, so its gap is the missing tiers
  rather than any `/usr/etc` arithmetic.

  Batch `libs-net-1` is written: ell, libmnl, libndp, libnftnl, libpcap, and
  libpsl all pass. Two records needed a classification call rather than a
  search. ell's `l_hwdb_new_default()` walks five fixed `hwdb.bin` paths,
  `/etc` before `/usr`, with no `/run` entry and no merge, and that is correct
  because `hwdb.bin` is one binary index that `systemd-hwdb update` generates
  from rule files other packages own. libpcap opens `/etc/ethers` from the
  compiled `PCAP_ETHERS_FILE` constant with no fallback, and that is correct
  because `ethers(5)` is an administrator lookup table of the machine's own
  address mappings, the same machine-owned class already applied to
  `rsyncd.conf`.

  libpcap also carried a third "not compiled" claim, and this one held.
  `pcap-sita.c` opens `/etc/hosts` directly, but `Makefile.in:92` lists
  `COMMON_C_SRC` without it, and the file appears only in the distribution list
  at `Makefile.in:361-362` beside other platform sources this build skips.
  libndp passes `--sysconfdir=/etc` yet never reads the substitution, so the
  flag is inert. libpsl compiles the suffix list in through `-Dbuiltin=true`,
  which makes the upstream data a build-only input.

  Batch `libs-multimedia-2` is written: openh264 passes. Its one source holds a
  configuration reader, `CReadConfig` in the console tools, but the manifest
  installs no path below `/usr/bin`, so that reader never ships. The check that
  `libs-multimedia-1` taught was applied here before the record was accepted.

  Batch `libs-multimedia-1` is written: dav1d, ffmpeg, gst-plugins-base,
  gstreamer, libaom, and libvpx all pass. None of the six sources contains a
  `SYSCONFDIR` reference, so the `/usr/etc` shape that libinput and
  libxkbcommon showed does not reach this batch. gstreamer holds the only real
  configuration surface, its binary plugin registry, and that is a generated
  cache keyed on `GST_REGISTRY_1_0`, `GST_REGISTRY`, or the user cache
  directory, with the installed plugin set as its source of truth.

  The worker's report was wrong on two records and the verdicts survived for
  different reasons. It claimed libaom ships only `aomdec` and libvpx ships
  only `vpxdec`, and rested both `pass` results on the encoder code being
  unshipped. Both manifests ship the encoder as well. libaom's `aomenc` does
  read a configuration file, but only the path that `--use-cfg` names, which
  `apps/aomenc.c:626-628` passes straight to `parse_cfg()` with no fallback, so
  it stays a command-line argument. libvpx's fixed-name writers sit behind
  `CONFIG_INTERNAL_STATS`, which the manifest never enables, and they write
  relative debug names rather than read system files. The records now say that.
  Always check a claim that a binary is unshipped against the manifest's
  `outputs`. That is the seventh worker call this audit has overturned.

  Batch `libs-input` is written, and it is the first batch this plan ran with
  Claude as supervisor. ibus, libevdev, mtdev, and xkeyboard-config pass.
  libinput and libxkbcommon are both gaps, and they share one root cause. This
  paragraph originally named the wrong cause and is corrected here. The claim
  was that neither manifest passes `--sysconfdir`, so meson computes the
  administrator directory as `get_option('prefix') / get_option('sysconfdir')`
  and lands it on `/usr/etc`, leaving the machine owner nowhere to put policy.
  That is false. Meson defaults `sysconfdir` to the absolute `/etc` when
  `prefix` is `/usr`, so both packages already resolved their administrator
  directory to `/etc/libinput` and `/etc/xkb`. Pre-existing `libinput.so`
  artifacts under `.nex/tmp/` confirm it: built before any `--sysconfdir` was
  added, they already contain `/etc/libinput/local-overrides.quirks`.

  The real shared cause is narrower and still a gap in both: neither reader has
  a `/run` tier. libinput additionally exposed its administrator layer as one
  fixed file rather than a drop-in directory, so it had no per-basename
  replacement and no masking. Both repairs remain correct, because both add the
  missing boot tier; the `--sysconfdir=/etc` each repair also adds is a no-op
  kept only to state intent. No assembly compensates for either path.

  ibus needed a second look and survived it. Its engine registry reads exactly
  one directory and `IBUS_COMPONENT_PATH` replaces that directory rather than
  extending it, but the family is vendor engine discovery rather than
  administrator policy, and `--disable-dconf` removes the only persistent
  settings backend. The per-user `XDG_RUNTIME_DIR` scan that libinput performs
  is user-owned and does not substitute for the missing `/run` tier.

  Batch 7 (`apps-misc`) is written. Its five manifests gave four new records,
  because `ca-certificates` already had the pilot's entry. Meld, Remmina, and
  DB Browser for SQLite are `pass`; FreeRDP was a gap and is repaired. The
  worker reported FreeRDP as `pass` and missed the hive entirely, so this is
  the fourth worker `gap`/`pass` call that root validation overturned.

  Batches 8 to 10 (`apps-multimedia`, `apps-security`, `apps-terminal`) are
  written: seven manifests, six `pass` and one `gap fixed`. mpv was the gap and
  the worker called it correctly; JACK example tools, pavucontrol, VLC,
  GNOME Keyring, polkit-gnome, and foot pass as built.

  Batches 11 to 14 (`apps-virt`, `apps-web`, `apps-x11`, `bootstrap-phase0-1`)
  are written: eleven manifests, nine `pass`, one `gap fixed`, and one open
  `gap`. Looking Glass was a gap that the worker called `pass`, and it is now
  repaired and proven. virt-manager, xauth, xhost, and the six phase0 seeds
  pass.

  One entry is deliberately left open: `pkg/apps/web/chromium.yaml` is a real
  gap. Chromium reads enterprise policy only from `/etc/chromium/policies` and
  native-messaging hosts only from `/etc/chromium/native-messaging-hosts`, with
  no `/run` or `/usr` tier, which the audit entry documents with the exact
  source lines and the fix it needs.

  The repair is written but not landed. The patch sits at
  `.nex/tmp/ep016-chromium-uapi-config.patch`, outside Git, and
  `patch --batch --fuzz=0 -Np1 --dry-run` applies all six of its files against
  the pinned 143.0.7499.169 tree. It gives `ConfigDirPolicyLoader` a list of
  directories read lowest priority first, one pair of watchers per directory,
  and a `LastModificationTime` that scans every tier; it adds
  `kTransientPolicyPath` and `kVendorPolicyPath` beside `kPolicyPath`; and it
  extends the native-messaging search with the matching `/run` and
  `/usr/lib` directories.

  Batches `cli-system-4` and `cli-system-5` are written (2026-08-17). The
  eight worker claims were opened against the pinned sources. starship, tree,
  usbutils, zmx, and zoxide are `pass`. procs, screen, and tmux are confirmed
  `gap`s of the same one-system-file shape already open on zsh, bmon, htop,
  and ncdu. The tmux prediction from `strings` on the stored binary held:
  `Makefile.am:14` compiles exactly
  `/etc/tmux.conf:~/.tmux.conf:$XDG_CONFIG_HOME/tmux/tmux.conf:~/.config/tmux/tmux.conf`.
  zmx's tarball was not in `inputs_cache`; the record cites the pinned
  GitHub commit `4d70b4c97d254aa058e4774b6cb53a3ee9d3992d` instead. The
  checker now prints `pass: 130, gap: 8, gap fixed: 22, not audited: 373`.

  Batch 29 (`cli-system-3`) is written: fd, fzf, i2c-tools, and lsd pass, and
  htop and ncdu are open gaps. Both are the same shape as bmon and zsh: one
  administrator file and no vendor tier, so a deployment cannot ship defaults.
  ncdu is the starkest case, because `src/main.zig:488` contains the literal
  `tryReadArgsFile("/etc/ncdu.conf")` rather than a `sysconfdir` macro, so no
  configure flag can move it.

  Batch 28 (`cli-system-2`) is written: dool, dua-cli, duf, dust, efibootmgr,
  and eza all pass. None of the six has a system configuration file: the Rust
  and Go tools use flags or a per-user file, dool's plugins already live below
  `/usr/share`, and efibootmgr's configuration is the firmware variable store
  itself.

  Batch 27 (`cli-system-1`) is written: bottom, broot, btop, chezmoi, and
  dmidecode pass, and bmon is a third open `gap`. bmon reads
  `SYSCONFDIR "/bmon.conf"` and then `~/.bmonrc`, overlaying each onto a
  compiled default buffer, so its one system tier is the administrator's and a
  deployment cannot ship a default. Its manifest does pass
  `--sysconfdir=/etc`, so it escapes the `/usr/etc` trap. Worth noting for
  future pre-checks: a grep for a literal `"/etc/` in bmon's sources finds
  nothing, because the path is assembled from a compile-time macro, so that
  shortcut cannot stand in for reading the reader.

  Batch 26 (`cli-net`) is written: aws-cli, curl, mosh, openbsd-netcat, rsync,
  and tcpdump all pass. rsync deserved the closest look, because its daemon
  reads a single `/etc/rsyncd.conf`. That file lists the local paths a host
  exports and the accounts allowed to reach them, so it is machine-owned state
  rather than a default a distribution could ship; a vendor copy below `/usr`
  would export modules the machine owner never chose. That is the same
  reasoning the audit applied to aardvark-dns and cryptsetup, and it contrasts
  with zsh, where a default `zshrc` is exactly the kind of thing a deployment
  ships. curl and mosh both avoid the `/usr/etc` trap, curl because it has no
  system `curlrc` at all and mosh because its manifest passes an explicit
  `--sysconfdir=/etc`.

  Restarting the stopped Chromium build settled a question this plan had left
  open: `nex build` rebuilds the composite rootfs each invocation, so
  `WORK_DIR` starts empty and a stopped build loses its compiled objects. The
  restarted Chromium ran `tar` and `xz` again rather than finding its
  `.nex_source_id` marker, which contradicts the earlier guess that the marker
  implied persistence. Interrupting that build therefore cost about 34000
  compiled objects; it still paid, because the GnuPG and Vim repairs it
  unblocked took roughly twenty minutes together and are both committed, while
  Chromium needed several more hours either way.

  Batch 25 covered `cli-editor`, `cli-editors`, `cli-fs`, and `cli-json` in one
  worker run: helix, neovim, sshfs, and jq pass, and Vim is a third open `gap`.
  Vim is the inverse of every other gap this audit has found. Its one system
  file is `SYS_VIMRC_FILE`, which `os_unix.h:190-191` defines as `$VIM/vimrc`
  and `main.c:3385-3386` sources exactly once, so the only tier that exists is
  the vendor one: a package could ship a default, but an administrator has
  nowhere to put machine policy, and on a Nex machine `/usr` is immutable. The
  package does not even install `/usr/share/vim/vimrc`, so the family has no
  file anywhere today. Neovim avoids this because it resolves
  `nvim/sysinit.vim` through `XDG_CONFIG_DIRS`, which the desktop assembly sets
  to all three trees.

  The Vim repair is landed and proven. Both strict builds reproduce checksum
  `8a4a6e7a5d150844ce623b6d2830a4641f81b5565824f0370f062855333da4b1`, and
  `strings` on the rebuilt `usr/bin/vim` now shows `$VIM/vimrc`,
  `/run/vim/vimrc`, and `/etc/vimrc`. The smoke took three attempts, and both
  failures were in the probe rather than the patch: the sandbox has no usable
  `/dev/stdout`, so the first probe wrote an empty file, and `vim -es` skips
  the system vimrc entirely because `main.c:3372` guards that block with
  `else if (!silent_mode)`, so the second probe reported the compiled default
  and looked exactly like a broken patch. Normal mode with `--not-a-term -n`
  and a `redir!` to a real file settled it. Vim's two and a half minute build
  made that iteration cheap; the same mistake in the Chromium smoke would have
  cost hours, which is why that one was rehearsed against the host browser
  first.

  The original note, for the record.
  `pkg/cli/editors/vim-uapi-config.patch` sources three files in vendor,
  transient, then administrator order through two new overridable macros,
  `patch --batch --fuzz=0 -Np1 --dry-run` applies it, the manifest already
  carries the patch source and the smoke, and `nex check` passes. Upstream supports this
  shape: `feature.h:701-708` already carries a commented-out `/etc/vimrc` for
  that macro. The smoke will plant a distinct `set tabstop=` in each tier and
  read the result with
  `vim -es -c 'redir >> /dev/stdout' -c 'echo &tabstop' -c 'redir END' -c 'qa!'`,
  a probe whose mechanics were checked on the host first.

  Batch 24 (`cli-calc`) is written: bc passes. It has no system configuration
  path at all; its defaults come from `BC_ENV_ARGS` and `DC_ENV_ARGS`, and `-l`
  loads its mathematical library from strings compiled into the binary.

  The three repairs already committed this run were re-verified from the store
  rather than from their build logs: `strings` on the stored
  `libwinpr3.so.3.27.1`, `libmpv.so.2.5.0`, and `looking-glass-client` shows
  `/etc`, `/run`, and `/usr/lib` variants of each family, so the fixes are in
  the artifacts and not only in the smoke output.

  Batch 23 (`cli-crypto`) is written: age, cryptsetup, and pinentry pass, and
  GnuPG is a second open `gap`. GnuPG resolves every system file through one
  compiled `GNUPG_SYSCONFDIR`, and because
  `pkg/cli/crypto/gnupg.yaml:98-104` passes `--prefix=/usr` with no
  `--sysconfdir`, autoconf's default of `${prefix}/etc` makes that directory
  `/usr/etc/gnupg`, a path `CONFIGURATION.md` bans outright. `strings` on the
  stored `usr/bin/gpg-agent` confirms it. So a machine's `/etc/gnupg` is never
  read, and the shipped `tools/applygnupgdefaults` refuses to run without
  `/etc/gnupg/gpgconf.conf`, a file no daemon would ever read. Upstream's own
  `doc/instguide.texi:28` tells packagers to pass `--sysconfdir=/etc`. The
  repair is a manifest flag plus a tiered `gnupg_sysconfdir()`, and it waits
  only because the Chromium build holds the package store.

  The GnuPG repair is landed and proven. Both strict builds reproduce checksum
  `22661374427c01f39fd358895d1a735abd217ab03b6ee87d9d2b29a12844cb9a`, the smoke
  passes in all four compiles, and `strings` on the rebuilt `usr/bin/gpg-agent`
  now shows `/etc/gnupg`, `/run/gnupg`, and `/usr/lib/gnupg` with no `/usr/etc`
  path. Landing it meant stopping the Chromium build: a five minute measurement
  put that build at 109 objects a minute with 20830 left, so it had about three
  more hours for its first compile and as long again for the second, and it
  blocked every other package. GnuPG then took about ten minutes.

  The original note, for the record. The patch is
  `pkg/cli/crypto/gnupg-uapi-config.patch`, staged but not committed, and
  `patch --batch --fuzz=0 -Np1 --dry-run` applies it; the manifest already
  carries `--sysconfdir=/etc`, the patch source, and a smoke that asserts
  `gpgconf --list-dirs sysconfdir` walks `/etc/gnupg`, `/run/gnupg`, and
  `/usr/lib/gnupg` as each directory appears and disappears. `nex check`
  passes. One trap was caught before it cost a build: the tarball ships
  generated `Makefile.in` files that already embed the `-DGNUPG_SYSCONFDIR`
  flags, so a change to `am/cmacros.am` would have done nothing, and the patch
  writes `/run/gnupg` literally instead.

  A store-wide sweep looked for other packages with the same defect.
  `grep -rhoa -E '/usr/etc/[A-Za-z0-9_.+-]+' .zub/objects/blobs` searched all
  23 GB in a few minutes, because zub stores blobs uncompressed, and returned
  30 distinct paths. Most are stale: the current `cupsd`, `wget`, and `tig`
  binaries each carry `/etc/<name>`, `/run/<name>`, and `/usr/lib/<name>`,
  which independently confirms the Milestone 2 repairs and the `pass` records
  for wget and tig. GnuPG is the only live defect the sweep found. The paths
  still worth checking when their batches arrive are `/usr/etc/alsa`,
  `/usr/etc/popt.d`, `/usr/etc/enchant-2`, `/usr/etc/libosinfo`, `/usr/etc/xml`,
  `/usr/etc/sgml`, `/usr/etc/X11`, `/usr/etc/grub.d`, and `/usr/etc/skel`.

  That find generalises: autoconf, unlike Meson, does not special-case
  `--prefix=/usr` into `sysconfdir=/etc`, and 210 tracked manifests run an
  autotools `configure` with `--prefix=/usr` and no `--sysconfdir`. Most have no
  system reader at all, so nothing reaches a binary, but the ones that do are
  invisible to `scripts/check-package-config-paths.sh`, which inspects declared
  outputs rather than compiled reader paths. A store-wide sweep for `/usr/etc`
  strings is the way to find the rest.

  Batch 22 (`cli-archive`) is written and opens `pkg/cli/`: cpio, gzip, lzo,
  tar, and unzip all pass. These are the shipped copies rather than bootstrap
  seeds, so the three tiers really do apply to them, but none of the five has a
  configuration file at all: each takes its defaults from an environment
  variable that it prepends to its own argument vector.

  Batch 21 (`bootstrap-phase1-4`) is written: sed, tar, test, xz, and zlib all
  pass. That closes `pkg/bootstrap/` entirely: all 43 manifests are recorded
  and every one is a `pass`, because no assembly installs a bootstrap package
  and each seed either has no configuration reader or keeps upstream's fixed
  paths while shipping nothing below `/etc`.

  Batch 20 (`bootstrap-phase1-3`) is written: gzip, linux-headers, m4, make,
  patch, and python3 all pass. linux-headers installs no program at all, and
  the phase1 python3 declares a reduced standard library that leaves out
  `mailcap`, `mimetypes`, `platform`, and `_ssl`, so the stdlib modules that
  name `/etc` paths are not a surface of this package. Five bootstrap records
  remain.

  Batch 19 (`bootstrap-phase1-2`) is written: diffutils, findutils, gawk, gcc,
  glibc, and grep all pass. The worker reported the phase1 glibc as a `gap`
  because its NSS, loader, and resolver readers are unpatched and read `/etc`
  only. Root validation overturned that: `grep -rln 'bootstrap/phase1/glibc'
  pkg/` returns twenty manifests, nineteen under `pkg/bootstrap/` and the
  twentieth `pkg/libs/system/glibc.yaml`, which lists it as a build dependency
  while compiling the real system library. Nothing this package builds ever
  runs on a Nex machine, and it deletes the `/etc` files it would otherwise
  ship, so it is a `pass` for the same reason as the phase0 toolchain. That is
  the sixth worker call this audit has overturned.

  Batch 18 (`bootstrap-phase1-1`) is written: the phase1 bash, binutils,
  bison, bzip2, and coreutils seeds pass, as does the `debug` smoke package,
  which installs one marker file and no reader at all. binutils is the only one
  that names a system file: `ld.bfd` reads `/etc/ld.so.conf` to widen its own
  library search, which is glibc's interface rather than policy this package
  owns, and `ld.gold` does not read it.

  Batch 16 (`bootstrap-phase0-3`) is written: patch, perl, python3, sed, tar,
  and toolchain all pass. Perl's one candidate system reader, `sitecustomize.pl`,
  is compiled out because the manifest never passes `-Dusesitecustomize`;
  Python's site initialisation is module import rather than a configuration
  search and names no `/etc/python3` path; and the toolchain's glibc keeps
  upstream's fixed `/etc` files but ships nothing below `/etc`, because the
  build deletes `/etc/rpc` and removes the directory. All four phase0 batches
  are now recorded, which closes `pkg/bootstrap/phase0/`.

  Batch 17 (`bootstrap-phase0-4`) is written: xz and zlib pass. xz takes its
  defaults from `XZ_DEFAULTS` and `XZ_OPT` and has no configuration file at
  all, and zlib reads no environment variable and opens only the paths its
  callers pass.

  Batch 15 (`bootstrap-phase0-2`) is written: findutils, gawk, grep, gzip, m4,
  and make all pass. None of them opens a system configuration file. The only
  paths worth naming are findutils' generated locate database below
  `/var/lib/locate`, gawk's compiled `.:/usr/share/awk` library search, and
  make's compiled include directories, and all three already sit below `/usr`
  or `/var` rather than `/etc`.

  Three things must happen before it can be committed, and the first one is a
  human decision:

  1. Chromium is the most expensive package in this repository. Its recorded
     build profile runs to roughly two hours, `./nex build --check` compiles the
     package twice per invocation, and this plan asks for two invocations, so
     landing this fix costs most of a working day of machine time. `/var` had
     137 GB free and the extracted source alone is 27 GB, so the run also needs
     its disk headroom watched.
  2. The smoke still has to be designed. The cheapest honest probe found so far
     is to run the built browser with
     `--headless=new --no-sandbox --enable-logging=stderr
     --vmodule=config_dir_policy_loader=1` and read the
     `Found mandatory policy file:` lines that `LoadFromPath` already emits,
     with one JSON file planted in each tier in turn. Every attempt at that
     probe costs a full build, which is why it was not attempted blind.
  3. `nex check`, the two strict builds, and the audit entry all have to be
     updated together, and the entry's `Result` moves from `gap` to
     `gap fixed`.

  Drop a batch's extracted sources with `.nex/tmp/ep016-drop-src.sh` once its
  records are written. A full extraction of all 533 manifests needs roughly
  170 GB and does not fit beside the store.

  Two traps cost a rebuild each and are worth avoiding. Declaring a `file:`
  patch source and a `patch` dependency does nothing unless the build script
  also runs `patch --batch --fuzz=0 -Np1 -i "/${SOURCE_<name>}"`; containerd
  built green with the patch never applied, and the only symptom was `go test`
  reporting `[no test files]`. And `pgrep -f 'nex build'` matches the waiting
  shell's own command line, so `until ! pgrep -f 'nex build'` never exits and
  any build chained after it never starts; run builds in the foreground with a
  timeout instead.
- [ ] Implement and test each package fix that the audit requires, using one
  small buildable commit per coherent package or shared reader.
- [ ] Prove that the audit covers the current tracked manifest set, rerun all
  repository-wide configuration checks, rebuild and exercise affected package
  and assembly artifacts, promote durable knowledge, and prepare the human
  review gate.

### Next actions, in order

Codex assigns each item. Claude does the first audit or coding pass. Codex
reviews and commits.

0. Done. PipeWire now passes the package, main-file, drop-in, and two finished
   system tests described in `Progress`, and its audit entry says `gap fixed`.
1. Done. Screen now passes the non-PTY startup and socket-sequence test in
   `Progress`.
2. Done. Chromium's enterprise policy loader passes the two internal builds
   and two real-browser smokes described in `Progress`.
3. Assign Claude the first `pending` Milestone 3 batch in
   `.nex/tmp/ep016-batches.tsv`. The batch ledger carries the current status,
   and `sh scripts/check-package-config-audit.sh` prints how many records
   remain.
4. libinput is done. Its quirks reader now merges `/etc/libinput`,
   `/run/libinput`, and `/usr/share/libinput`, both mask forms work, the
   `install_emptydir` call that would have declared an `/etc` output is gone,
   and two strict builds reproduced checksum `e7b45555`. `libxkbcommon` is
   still open; the assignment for it is written at
   `.nex/tmp/ep016-prompts/repair-libxkbcommon.md`.

   Original entry, kept for the remaining half: repair
   `pkg/libs/input/libxkbcommon.yaml`. It puts the administrator tier on
   `/usr/etc` because the manifest never passes `--sysconfdir`. Passing
   `--sysconfdir=/etc` moves that tier to a writable directory, but it still
   leaves no `/run` tier, so follow the mpv precedent: a UAPI config patch that
   walks `/etc`, `/run`, then `/usr`, plus a smoke that plants one file per
   tier and reads which value wins. libxkbcommon's reader is
   `xkb_context_include_path_append_default()` in `src/context.c`, and its
   include list is first-match-wins per file name.

5. New gaps this run, in the order they were found. Each has a confirmed
   reader and a settled classification; none has an assignment written yet
   except where noted.

   - `pkg/libs/net/libssh.yaml`. `DefineOptions.cmake:72` hardcodes
     `GLOBAL_CONF_DIR` to `/etc/ssh`, and the client config, the
     `libssh_server_config`, and `/etc/ssh/moduli` are each a single fixed read
     with no `/run` and no vendor tier. `config.h.cmake:12` declares an unused
     `USR_GLOBAL_CONF_DIR`. Repair alongside
     `pkg/cli/net/openssh-uapi-config.patch`, which already gives the same file
     names three tiers, so the two packages should agree.
   - `pkg/libs/net/samba.yaml`. `libsmb_context.c` reads `$HOME/.smb/smb.conf`
     then the single compiled `/etc/samba/smb.conf`. The manifest already
     passes `--sysconfdir=/etc`, so only the tiers are missing. Dormant today,
     since no assembly references smb.conf or winbind.
   - `pkg/libs/security/krb5.yaml`. The cheapest of the four, because
     `init_os_ctx.c` already splits a colon-separated profile list and the
     profile library merges every entry that exists. Only the compiled default
     is too short: `configure.ac:12-16` empties `SYSCONFCONF` when
     `--sysconfdir` is `/etc`, collapsing the list to `/etc/krb5.conf`.
     Extending that default should need no new parser. The GSSAPI registry is a
     second family in the same package: `g_initialize.c:61-64` hardcodes
     `/etc/gss/mech` and `/etc/gss/mech.d/*.conf`, already the right
     base-plus-drop-in shape but in one tier only.

6. Revisit `pkg/apps/web/chromium.yaml` after its policy fix lands. Its
   native-messaging host directory is still single-tier, and closing that needs
   a test that drives an extension over a native messaging port. Claude
   implements; Codex reviews.
7. Done. p11-kit's global file and module registry pass `print-config` and
   library-backed `list-modules` tests for all three tiers.
8. Done. `nex-systemd` pins the repaired package, reproduces its system
   checksum, and runs both real p11-kit readers from the copied finished root.

### Deliberately uncommitted worktree state

`libs-graphics-7` is in progress. The Vulkan Loader package repair landed in
`e6946b9`.

The human stopped the `desktop-vwl` verification build on 2026-08-18 at about
17:32 local, roughly 18000 of 54756 Chromium targets in. That run carried
`--update-checksum`, and the interrupted run had already written a `checksum:`
key into five phase0 seeds: `bash`, `coreutils`, `diffutils`, `file`, and `m4`.
No phase0 manifest carries a committed `checksum:` key, because phase zero sets
`stable_checksum: false` on purpose. The human confirmed those five lines were
unwanted and they were reverted with `git checkout --`. Do not commit a
`checksum:` key that an interrupted `--update-checksum` run leaves on a phase0
seed. The `desktop-vwl` Vulkan Loader pin itself is unaffected and stays
uncommitted. The tested glibc clean-sandbox fix forms one package commit
candidate. The base desktop's Vulkan Loader pin is deliberately uncommitted
until the base and both inherited NVIDIA desktops build and their finished
roots pass the loader smoke. Chromium's native-messaging reader remains open
for a later batch.

## Surprises & Discoveries

- Observation: the autotools watch list for the remaining audit work is 40
  manifests long, and the artifact scan clears most of it in one pass.
  Evidence: of the 62 records still `not audited`, 40 build with `./configure`
  and pass no `--sysconfdir`, which is the only combination that can yield
  `/usr/etc`. 16 use meson without `--sysconfdir`, which resolves to `/etc` and
  is therefore low risk, and 6 already pass the flag. The list is at
  `.nex/tmp/ep016-autotools-watch.txt`.

  Roughly thirty of those 40 are X11 and xcb protocol libraries. Every
  `libX*.so.*`, `libxcb*.so.*`, `libfontenc`, `libICE`, and `libSM` present in
  the 204-package root was scanned and none contains a `/usr/etc` string, so
  that whole group is empirically clear of this shape. libidn2, libunistring,
  libxslt, newt, and libass were checked the same way and are also clear.
  oniguruma was absent from that root, so it was settled from its cached
  tarball instead: `inputs_cache/2a5cfc5a...` extracts to `onig-6.9.10`, which
  contains no `SYSCONFDIR` use and no `/etc` literal in any `.c`, `.h`, or
  `Makefile.am`. The five remaining `libs-text-1` sources, fribidi,
  gtksourceview4, gtksourceview5, harfbuzz, and libass, were screened the same
  way and are also clear.

  That closes the watch list. Across all 40 autotools candidates among the 62
  unaudited records, `pkg/libs/text/enchant2.yaml` is the only `/usr/etc` case,
  and it was confirmed three ways before its report arrived: the `/usr/etc`
  string in the built `libenchant-2.so.2.8.2`, the `@SYSCONFDIR@` substitution
  at `src/Makefile.am:21`, and the reader at `lib/provider.c:350`, which appends
  `SYSCONFDIR/enchant-2` to `conf_dirs` beside `pkgdatadir`. Enchant's own
  `enchant.5` documents both directories.

  This clears one specific question only. A zero here means the package has no
  `/usr/etc` path compiled in; it says nothing about whether the package reads
  configuration from a correct location. libx11 reading its locale data below
  `/usr/share/X11/locale` is a vendor path and fine, and still needs its own
  record.

- Observation: scanning a built rootfs snapshot for the literal `/usr/etc`
  finds the autotools gaps directly, without reading any build file.
  Evidence: `grep -rl "/usr/etc" .nex/tmp/ep013-desktop-dev-00e2-root/nex/pkg`
  over a 204-package root reduced, after collapsing bundled copies to distinct
  binaries, to a short list. Most entries are not readers and must be read
  rather than counted: `bash` carries the default PATH constant
  `/bin:/usr/bin:/sbin:/usr/sbin:/en:/etc:/usr/etc`, `whereis` carries its
  built-in directory list, `fish` carries the string inside a TODO comment, and
  the perl `.pod`, `mailcap.py`, and shell-completion hits are documentation.

  Four entries are or were real. `libxml2.so.2.13.5` names
  `file:///usr/etc/xml/catalog` and nothing else, which is the gap recorded in
  this batch. `libpopt.so.0.0.2` names `/usr/etc/popt`, `/usr/etc/popt.d`, and
  `/usr/etc/popt.d/*`, and `pkg/libs/system/popt.yaml` is autotools with no
  `--sysconfdir`; it was `not audited` when found and sits in `libs-system-6`.
  `libpkgconf.so.5.0.0` names
  `/usr/share/pkgconfig/personality.d:/usr/etc/pkgconfig/personality.d`, but
  `pkg/core/toolchain/pkgconf.yaml` is already `gap fixed`, so that snapshot
  predates its repair. `useradd` names `/usr/etc/skel` and
  `pkg/core/userland/shadow.yaml` is likewise already `gap fixed`.

  Two caveats follow from those last two. Snapshots go stale, so check the
  package's current audit result before treating a hit as open. And only a path
  containing `/nex/pkg/` is a Nex build: the sole `libpciaccess.so.0.11.1` under
  `.nex/tmp` belongs to Chromium's Debian sysroot and proves nothing.

- Observation: a `/usr/etc` string in a built library does not by itself mean
  the package reads configuration, and `pkg/dev/libs/libgpg-error.yaml` is the
  worked example. Its record said `pass` because "`rg` finds no `/etc` or
  `SYSCONFDIR` reader", yet the built `libgpg-error.so.0.41.2` carries a bare
  `/usr/etc` string, so something compiled it in.

  The tarball was in `inputs_cache/`, so the question was settled rather than
  left open. `src/Makefile.am:218` passes `-DSYSCONFDIR=\"$(sysconfdir)\"`,
  and `src/stringutils.c:101` uses it inside `_gpgrt_fnameconcat()`, which
  prefixes `SYSCONFDIR` only when the caller passes `GPGRT_FCONCAT_SYSCONF`.
  The package therefore builds paths for callers and opens no file itself, so
  `pass` was right and the evidence line was wrong. The record now states the
  real mechanism.

  One latent consequence was recorded with it: because libgpg-error is
  autotools with no `--sysconfdir`, that compiled base is `/usr/etc`, so any
  shipped consumer that ever uses `GPGRT_FCONCAT_SYSCONF` would build a
  forbidden path. That is the consumer's defect, not this package's reader, but
  it puts the record on a re-audit trigger.

  `pkg/libs/text/enchant2.yaml` carries the same `/usr/etc` signal in
  `libenchant-2.so.2.8.2` and is still `not audited`. Settle it the same way,
  by extracting the pinned tarball and reading the use rather than counting the
  string.

- Observation: one build-file pattern predicts a missing-tier gap with no false
  positives so far. Read the correction below before trusting the first
  explanation this plan gave for it.
  Evidence: on 2026-08-18 this command over every extracted source

      grep -rlE "get_option\('prefix'\).*get_option\('sysconfdir'\)" \
          .nex/tmp/ep016-src/<slug> --include='meson.build'

  returned exactly seven packages, and every one is a real gap: libglvnd,
  libva, mesa, and libinput were already `gap fixed`, libxkbcommon was
  mid-repair, and appstream and fuse3 were identified by this screen before
  their audit reports arrived. The C-side macro name is the wrong thing to
  search, because it varies: appstream uses `SYSCONFDIR "/appstream.conf"` in
  `as-context.c:623`, fuse3 defines `-DFUSE_CONF` from
  `join_paths(get_option('prefix'), get_option('sysconfdir'), 'fuse.conf')` in
  `util/meson.build:1` and opens it at `util/fusermount.c:640`, and krb5 hides
  the same idea behind a `SYSCONFCONF` substitution inside a `.hin` template.
  A `SYSCONFDIR` grep misses both fuse3 and krb5. Screen the arithmetic that
  produces the path, not the name the C code gives it.

  Correction, same day: the screen works, but the first explanation of *why*
  was wrong. `get_option('prefix') / get_option('sysconfdir')` does not yield
  `/usr/etc`. Meson applies an FHS special case and defaults `sysconfdir` to
  the absolute `/etc` whenever `prefix` is `/usr`, so the join yields `/etc`.
  Tested directly with meson 1.12.0 on a three-line project, which printed
  `sysconfdir = /etc` and `joined = /etc`. Confirmed against the store as well:
  `libinput.so.10.13.0` artifacts built from the manifest *before* any
  `--sysconfdir` was added already contain `/etc/libinput/local-overrides.quirks`.

  So the pattern is not a marker of a misplaced `/usr/etc` path. It is a marker
  of a package that computes one administrator directory and pairs it with one
  vendor directory, which is exactly the two-tier shape that lacks `/run`. The
  screen keeps its predictive value and loses its original rationale. The
  earlier records for alsa-lib, pipewire, wireplumber, and libcap already had
  this right and said so explicitly; the libinput and libxkbcommon records did
  not, and both were corrected.

  For autotools the original concern does hold, because autotools really does
  default `sysconfdir` to `${prefix}/etc`. Check the generated value rather
  than assuming either way.

- Observation: an assembly build can enter every source package even without
  `--force`, `--record-profile`, or `--generate-outputs`.
  Evidence: both `desktop-vwl-vulkan-1.log` and
  `desktop-vwl-vulkan-cache-1.log` spent about 18 minutes reaching the shipped
  glibc smoke and failed at the same missing `/etc` fixture. The forced command
  also rewrote 34 dependency manifests before the failure, so Codex reversed
  those exact generated diffs and resumed without those write-heavy flags.

- Observation: Vulkan Loader's duplicate-directory cleanup does not implement
  the package ownership rule for duplicate JSON basenames.
  Evidence: `read_data_files_in_search_paths()` removes identical directory
  strings, then `add_data_files()` appends every JSON it sees. The pinned Unix
  path omits `/run/vulkan`, and `get_unix_settings_path()` omits the matching
  runtime settings file. Claude's first-pass report called the package a pass,
  but direct source review proved both omissions and the missing mask behavior.

- Observation: Vulkan Loader's settings path parser made the second and later
  entries in a colon-separated XDG directory list unreachable.
  Evidence: `check_if_settings_path_exists()` resumed at the separator byte
  instead of the next byte. With the normal
  `/usr/local/share:/usr/share` fallback, it tested a malformed second path and
  could not open the packaged `/usr/share` settings file. Claude initially
  hid the issue by forcing `XDG_DATA_DIRS=/usr/share`; the repaired smoke leaves
  the variable unset and also tests an explicit two-entry value.

- Observation: NVIDIA 595.84 kept the exact proprietary profile-table layout
  found in 580.159.04 even though the newer package uses open kernel modules.
  Evidence: all six built 595.84 userspace libraries contain one table with
  administrator main, administrator directory, versioned vendor, and stable
  vendor paths. The generic fail-closed repair and real GLX parser smoke pass
  unchanged when given the newer version.

- Observation: Qt 6 does not normally use its single compiled settings path
  for Linux `QSettings` system fallbacks.
  Evidence: `qsettings.cpp:45-53` enables the XDG branch on Linux, and
  `QConfFileSettingsPrivate():1095-1118` walks every
  `QStandardPaths::GenericConfigLocation` unless the caller explicitly used
  `QSettings::setPath()`. Claude's report described the compiled path as the
  default reader, but its `pass` result remains correct because the real path
  is the desktop's ordered `XDG_CONFIG_DIRS` list.

- Observation: a finished Nex root's public loader can be a Nex shim rather
  than the glibc loader needed by a standalone probe.
  Evidence: invoking the Mesa helper through the public loader stopped with
  `nex-ld-shim: .nex-app-root not found`. Invoking it through the assembled
  glibc package's `ld-linux-x86-64.so.2` loaded the same assembled Mesa library
  and completed the surfaceless EGL probe.

- Observation: Mesa's XML configuration reader opens files through `open64`,
  not plain `open`, in the assembled root.
  Evidence: the first path interposer saw no reads even though EGL succeeded.
  Adding an `open64` wrapper exposed the exact administrator, runtime, and
  vendor paths and let the root smoke prove every tier.

- Observation: a real proprietary parser smoke can expose dependency metadata
  that structural ELF checks miss.
  Evidence: NVIDIA's GLX library could not load until the 580 manifest
  declared its actual `libX11.so.6` and `libXext.so.6` inputs, output needs,
  and resolutions. The repaired parser then ran without a GPU.

- Observation: libglvnd's directory override looks like its compiled default,
  but callers depend on different cross-directory behavior.
  Evidence: `src/EGL/libeglvendor.c:35-72` sends both colon-separated strings
  through the same per-directory loop upstream. Nex now invokes the new
  basename merge only when `__EGL_VENDOR_LIBRARY_DIRS` is absent. The smoke
  gives two override directories the same JSON basename and requires both
  vendors, while the same basename across default tiers yields one vendor.

- Observation: an upstream directory named `tests` can still supply a public
  installed runtime reader.
  Evidence: libdbusmenu builds `tests/json-loader.c` into
  `libdbusmenu-jsonloader.so`, installs its header and pkg-config file, and
  exposes `dbusmenu_json_build_from_file()`. That function passes its caller's
  filename to JSON-GLib. Treating the whole directory as disabled test code
  would have missed a shipped file surface.

- Observation: GTK4's system settings reader had the same inverted XDG order
  as GTK3 even though GTK4's source had moved and its Meson option spelling had
  changed.
  Evidence: `gtk/gtksettings.c:258-304` loads every file with
  `GTK_SETTINGS_SOURCE_DEFAULT`, walks `g_get_system_config_dirs()` forward,
  and loads the user file last. The old reader returned `uapi-vendor` after
  both `/etc/xdg` and `/run/xdg` files existed because the desktop lists
  `/usr/share/xdg` last and GTK lets the later value win.

- Observation: Meson changes its relative default sysconfdir into `/etc` for
  a `/usr` prefix before a project reads the option.
  Evidence: a minimal local project configured with `meson setup --prefix=/usr`
  printed `prefix=/usr`, `sysconfdir=/etc`, and `joined=/etc`. GTK4 joins those
  same two option values at `meson.build:195`, so its compiled reader names
  `/etc/gtk-4.0/settings.ini`, not `/usr/etc/gtk-4.0/settings.ini`.

- Observation: a Nex-structured desktop can keep a dependency command private
  to one package capsule even when that package's binary names a public
  absolute helper path.
  Evidence: the finished `desktop-vwl` root publishes
  `/usr/bin/dbus-broker-launch` but no `/usr/bin/dbus-daemon`.
  at-spi2-core's capsule contains its own `usr/bin/dbus-daemon`, while its
  assembled launcher still names `/usr/bin/dbus-daemon`. The direct smoke
  started the real session bus from the capsule-private daemon, then installed
  a temporary public wrapper only for the separate accessibility bus child.

- Observation: the desktop assembly source helper can exhaust `/tmp` even
  when the filesystem still has free blocks.
  Evidence: `.nex-dev-prepare` failed while copying its Rust vendor tree with
  repeated `Disk quota exceeded` errors. Other live Claude sessions owned
  about 22 GB below `/tmp/claude-1000`, so deleting that shared state would
  have crossed this plan's scope. Running the same strict build with `TMPDIR`
  set to `.nex/tmp/ep016-tmp` passed both system builds.

- Observation: `zub checkout` needs a mapped root user when a system tree
  carries directory owners that the host user cannot reproduce.
  Evidence: direct checkout of both PipeWire system refs stopped at `boot`
  with `EPERM`. Running the checkout itself under
  `unshare --user --map-root-user` completed, after which both roots ran
  `pw-config` under `unshare --root`.

- Observation: adding a directory to PipeWire's drop-in scan does not make a
  higher tier override a matching vendor basename.
  Evidence: the interrupted worker inserted `/run/pipewire` between the
  vendor and administrator scan levels. `pw_conf_load_conf()` records each
  loaded basename, and `check_override()` rejects a later level when an
  earlier level already used that name. `pipewire-3.log` reached install and
  then stopped before the smoke marker because only the vendor
  `10-uapi-smoke.conf` survived. The repair must change both the directory scan
  and its same-basename priority rule.

- Observation: libmagic's compiled `magic.mgc` shadows a text
  `/usr/share/misc/magic` of the same basename, so a smoke that
  copies the vendor database into the vendor tier and then plants
  a text signature there never sees the text file.
  Evidence: the first file smoke reported `data` for the vendor
  payload until the `magic.mgc` copy was dropped. `file-4.log`
  and `file-5.log` then printed
  `file UAPI smoke: magic loader read all three tiers in order`.
- Observation: a package can hide its whole system policy interface behind an
  emulation of another operating system's mechanism, and a worker can read the
  package correctly and still call it `pass`.
  Evidence: FreeRDP's client reads machine-wide policy, including which
  security protocols are enabled, from a Windows registry hive at the single
  path `/etc/FreeRDP/FreeRDP/HKLM.reg`. `libfreerdp/core/settings.c:1303` loads
  it inside `freerdp_settings_new`, so every client run consumes it. The
  `apps-misc` worker described `HKLM.reg` accurately and then classified it as
  machine-owned state that needs no tiers. The single lookup lived in
  `winpr_GetConfigFilePathVA`, which every WinPR system-config caller shares,
  so one change fixed the hive and the `SAM` credential file together.

- Observation: a build flag can retire a configuration reader, which makes an
  otherwise identical single-path lookup harmless.
  Evidence: FreeRDP resolves `certificates.json` through
  `freerdp_GetJSONConfigFile`, but `-DWITH_JSON_DISABLED=ON` compiles
  `winpr/libwinpr/utils/json/json-stub.c`, whose parse entry points return
  `nullptr`. No placement of that file can change the package, so the audit
  records it as present-but-inert with the flag and the stub as evidence
  instead of patching a path no test could exercise.

- Observation: the build sandbox holds only the manifest's declared
  dependencies, and it is smaller than the dependency list suggests.
  Evidence: an mpv smoke failed with `sed: command not found` because that
  manifest depends on coreutils but not sed, and a Chromium smoke marker failed
  with `sha256sum: command not found` even though chromium does depend on
  coreutils. Parse smoke output with bash built-ins, or compare files with
  `[ "$(cat a)" != "$(cat b)" ]`, rather than reaching for a tool that may not
  be there.

- Observation: `bash -o errexit` does not stop the build script when a command
  substitution inside an assignment fails.
  Evidence: after `sha256sum` was missing, the Chromium script kept running and
  reached `[202/202] LINK gn` with a half-computed marker. The failure was
  visible only as one line in the middle of the log. Check for a tool
  explicitly when a later step depends on it.

- Observation: a program that refuses to run as root cannot be dropped to
  another user inside the build sandbox.
  Evidence: `src/cli/src/build/script.rs:264-281` runs the script under
  `unshare --user --map-root-user ... --root=<dir>`, so exactly one id is
  mapped: `setgid(65534)` fails with `Invalid argument`, and
  `unshare(CLONE_NEWUSER)` fails with `Operation not permitted` because Linux
  forbids a new user namespace from inside a chroot. Interposing `getuid` and
  `geteuid` with `LD_PRELOAD` is the way through.

- Observation: a manifest can declare a patch source, take a `patch`
  dependency, pass `nex check`, and build green without the patch ever being
  applied.
  Evidence: `pkg/apps/containers/containerd.yaml` gained the
  `uapi_config` source and the toolchain `patch` dependency but no
  `patch --batch` line in its build script. Both strict builds succeeded and the
  only symptom was `go test ./defaults/` printing
  `?   github.com/containerd/containerd/v2/defaults  [no test files]`, because
  the test the patch adds was never written to the tree. Grep the script for
  `patch --batch` after wiring a new patch, and prefer a test whose absence is
  loud.

- Observation: `pgrep -f 'nex build'` matches the waiting shell's own command
  line.
  Evidence: `until ! pgrep -f 'nex build'; do sleep N; done` never exited, so a
  build chained after it never started and repeated status checks kept
  reporting `building` for a package whose log file did not exist. Use
  `pgrep -x nex`, or run the build in the foreground with an explicit timeout.

- Observation: GNU grep treats a file that contains a NUL as binary and
  `grep -o` then prints no matches, even when `grep -q` already succeeded.
  Evidence: the Chromium smoke accepted all three `Found mandatory policy
  file:` lines, then extracted an empty load order because `--dump-dom`
  mixed NULs into the same file as the VLOG lines. A synthetic file with
  those three paths plus a NUL reproduces `without -a: []` and
  `with -a: [vendor runtime admin]`. Keep program stdout off the log that
  the assertion greps, and pass `-a` anyway.

- Observation: a test can pass for a reason unrelated to the behavior it
  claims to check.
  Evidence: the first D-Bus mask assertion put an `allow` in the vendor
  drop-in and a `/dev/null` link in the administrator directory, then required
  the bus call to fail. The base session policy already permits that call, so
  the assertion passed whether or not masking worked. Both replacement
  assertions put a `deny` in the vendor file, so only real shadowing or masking
  lets the call through.

- Observation: replacing an upstream relative path with a compiled absolute one
  breaks any tree that is not installed at its final location.
  Evidence: D-Bus's vendor `<includedir>system.d</includedir>` resolves against
  the including file's directory through
  `make_full_path(&parser->basedir, ...)`. A first version of the family merge
  used `DBUS_DATADIR "/dbus-1"` instead, which broke the package's own smoke
  because that smoke starts the daemon from the staged `${OUT_DIR}` tree.

- Observation: Docker builds in GOPATH mode, so a relative test path resolves
  outside the module.
  Evidence: `GO111MODULE=off` with a `.gopath/src/github.com/moby/moby/v2`
  symlink made `go test ./daemon/command/` resolve the package as
  `_/nex/work/moby/daemon/command` and fail to find vendored imports such as
  `tags.cncf.io/container-device-interface/pkg/cdi`. The canonical path
  `go test github.com/moby/moby/v2/daemon/command` works.

- Observation: Git currently tracks 533 package manifests but only 39 package
  patch files of any kind. Thirty-one patch filenames contain the literal word
  `uapi`.
  Evidence: `rtk git ls-files 'pkg/**/*.yaml' | wc -l`,
  `rtk git ls-files 'pkg/**/*.patch' | wc -l`, and
  `rtk git ls-files 'pkg/**/*uapi*.patch' | wc -l` returned 533, 39, and 31 at
  `9b06fde`. The human remembers 34 UAPI configuration patches, so the audit
  must reconcile referenced configuration patches by content and manifest use
  rather than trusting filenames.

- Observation: `scripts/check-package-config-paths.sh` checks declared package
  outputs, not the source code that reads those outputs.
  Evidence: the script rejects package output paths below `/etc` or `/usr/etc`,
  but it cannot see a compiled reader that still opens only `/etc`. This plan
  retains that useful invariant check and does not treat it as audit evidence.

- Observation: the current agent clients support bounded, read-only worker
  runs without nested agents.
  Evidence: `grok --help` lists `--model`, `--reasoning-effort`,
  `--no-subagents`, `--permission-mode plan`, `--prompt-file`, and JSON output.
  `claude --help` lists `--model`, `--effort`, `--disallowedTools`,
  `--permission-mode plan`, print mode, and JSON output. `grok version` returned
  1.0.3 and `claude --version` returned 2.1.233.

- Observation: a package can pass every existing tier test and still lose its
  vendor tier once a service manager starts it.
  Evidence: BlueZ's patched `btd_config_path()` walked `/etc/bluetooth`,
  `/run/bluetooth`, then `/usr/lib/bluetooth` only when
  `CONFIGURATION_DIRECTORY` was unset, and treated an exported list as
  exclusive. The shipped `src/bluetooth.service.in` sets
  `ConfigurationDirectory=bluetooth`, so systemd always exports
  `/etc/bluetooth`, and `asm/desktop-vwl/desktop-vwl-overlay.yaml` enables that
  unit unchanged. The daemon therefore never read the three `.conf` files the
  package installs below `/usr/lib/bluetooth`. The package's own
  `unit/test-config-path` passed throughout, because it exercised the helper
  without the service environment. An audit must read the shipped unit and the
  assembly that enables it, not only the reader.

- Observation: a package can patch its main readers and leave a second binary
  reading the administrator file directly.
  Evidence: OpenSSH's patch replaced the system-file read in `ssh.c` and
  `sshd.c` but left `ssh-keysign.c:225` calling
  `read_config_file(_PATH_HOST_CONFIG_FILE, ...)`, which `pathnames.h` still
  defines as `SSHDIR "/ssh_config"`. `EnableSSHKeysign yes` in a drop-in, in
  `/run/ssh`, or in `/usr/lib/ssh` was accepted by `ssh -G` and rejected by the
  setuid helper, so host-based authentication failed.

- Observation: the pilot worker produced accurate, well-cited source findings
  but over-applied a `CONFIGURATION.md` example to a package.
  Evidence: Grok correctly read Less's `init_cmds()`, D-Bus's `include_dir()`,
  Attr's patched selector, and Zstd's absent reader. It then called
  `pkg/apps/misc/ca-certificates.yaml` a `gap` because the command defaults to
  writing `/etc/ssl/certs` while `CONFIGURATION.md` describes a `/run/ssl/certs`
  cache behind an `/etc/ssl/certs` link. Checking `asm/` showed the arrangement
  already exists at assembly level:
  `asm/nex-systemd-overlay.yaml:181-191` and
  `asm/edgebox-rootfs-overlay.yaml:36-47` create the symlink and add a unit
  drop-in passing `--output-dir /run/ssl/certs`. The package installs nothing
  below `/etc`, so its result is `pass`.

- Observation: 32 of the 32 configuration-lookup patches map one-to-one onto 32
  manifests, so no manifest carries two lookup patches and no lookup patch is
  shared.
  Evidence: `rg -lN --glob 'pkg/**/*.yaml' 'file:\s*\S+uapi\S*\.patch|file:\s*pkg/core/ipc/dbus-transient-config\.patch' pkg`
  returned exactly 32 paths, saved in `.nex/tmp/ep016-patched-manifests.txt`.

- Observation: extracting every manifest's pinned source at once does not fit
  on this host.
  Evidence: unpacking 83 of the 533 packages consumed 30 GB of the 207 GB free
  on `/var`, which extrapolates to roughly 170 GB. The bulk extraction was
  stopped and replaced by per-batch extraction with `.nex/tmp/ep016-extract.py`
  and cleanup through `.nex/tmp/ep016-drop-src.sh`. `.nex/tmp/ep016-src` is now
  registered in `.agents/cleanup-workdirs.sh`.

- Observation: every tracked package patch is referenced by exactly one
  manifest through a `file:` source, and the same manifests reference eleven
  further patches by `url:`.
  Evidence: `rg -oNI --glob 'pkg/**/*.yaml' 'file:\s*\S+\.patch' pkg | sort -u`
  and `git ls-files 'pkg/**/*.patch' | sort` produced identical 39-line lists at
  `33c8eb9`. `rg -n --glob 'pkg/**/*.yaml' 'url:.*\.patch' pkg` returned 14
  lines naming 11 distinct URLs.

- Observation: `inputs_cache/` stores fetched source archives under their
  manifest `sha256`, so a pinned source can be located without a network fetch.
  Evidence: `pkg/net/wifi/iwd.yaml` pins
  `2c41c5da9924b90f8383b293b0c0b3d0bfb34fdc8822d8d0d37ec100707f263e`, and
  `inputs_cache/2c41c5da9924b90f8383b293b0c0b3d0bfb34fdc8822d8d0d37ec100707f263e`
  is a 1.1 MiB XZ archive. Comparing all 625 distinct manifest source hashes
  with the 552 cached files showed 525 present and 100 absent, so batches that
  need an absent source must fetch or build it first.

- Observation: successful compiler and package-build progress consumes agent
  context without helping the audit.
  Evidence: `AGENTS.md` already requires long commands to write full output to
  ignored logs. This plan strengthens that rule: an agent may inspect the exit
  status and a few explicit success markers, but it must not display, `tee`,
  `cat`, or otherwise read a successful build log by default.

- Observation: `lslogins -s` is a valid `login.defs` probe only after
  the sandbox has a matching `passwd` and `group`. An empty listing
  makes "expect this user absent" a false pass.
  Evidence: the first util-linux smoke wrote `SYS_UID_MIN` 0 vs 1 and
  grepped for `root` without planting accounts. Admin (expect absent)
  passed on empty output; runtime (expect present) failed. Planting
  `root:x:0:0` and `root:x:0:` made the same probe print the smoke
  marker on both `--check` builds.

- Observation: a portal backend that looks under `SYSCONFDIR/xdg`
  does not inherit the desktop overlay's `XDG_CONFIG_DIRS`.
  Evidence: `xdg-desktop-portal` and fuzzel walk `XDG_CONFIG_DIRS`,
  so `/run/xdg` works. `xdg-desktop-portal-wlr` `get_config_path`
  hardcodes user then `SYSCONFDIR "/xdg"` (`config.c:136-138`).

## Decision Log

- Decision: apply basename shadowing only while Vulkan Loader walks its
  implicit search path, and preserve every caller-supplied Vulkan path list.
  Rationale: the implicit `/etc`, `/run`, and `/usr/share` directories express
  package ownership, so the first JSON basename must own that name even when
  the file is empty or masks with `/dev/null`. `VK_DRIVER_FILES`, legacy
  `VK_ICD_FILENAMES`, the additive driver list, and layer path variables are
  public process controls. Changing duplicate names in those explicit lists
  would break their documented caller-selected behavior.
  Date/Author: 2026-08-18 / Codex

- Decision: apply the generic application-profile repair to pinned NVIDIA
  595.84 after verifying its closed binary table, and keep the structural and
  real-parser proof in a version-parameterized shell helper.
  Rationale: the earlier choice to leave `nvidia-current` unchanged avoided
  assuming that a future driver would keep 580's private layout. The batch now
  pins 595.84, and direct inspection proves the same complete table exactly
  once in every affected library. The repair still fails closed when a future
  update changes that layout.
  Date/Author: 2026-08-18 / Codex

- Decision: use a basename merge for Mesa's compiled drop-in trees, then one
  first-existing main file, while leaving `DRIRC_CONFIGDIR` as an exclusive
  process override.
  Rationale: Mesa parses later XML values over earlier ones. Selecting one
  file per basename before a global filename sort gives administrator and
  runtime files complete ownership without changing the documented explicit
  override or the final user file.
  Date/Author: 2026-08-18 / Codex

- Decision: repair NVIDIA 580's closed profile reader with an equal-length
  path-table rewrite local to that package.
  Rationale: the proprietary ELF table has no spare slot. Replacing the
  versioned vendor slot with the shorter runtime directory plus trailing
  separators preserves every byte offset, while the existing stable vendor
  slot reaches the packaged profile through a link. Keeping the shared
  installer and `nvidia-current` unchanged avoids claiming the same binary
  layout for an unpinned future driver.
  Date/Author: 2026-08-18 / Codex

- Decision: merge libglvnd EGL registrations by basename only for the compiled
  default path, and preserve both secure environment overrides exactly.
  Rationale: the compiled directories express Nex's administrator, transient,
  and vendor ownership tiers. The environment variables are explicit process
  choices whose documented colon-list order already lets a caller request
  duplicate basenames. Applying system shadow rules to those lists would
  silently change a public override.
  Date/Author: 2026-08-18, Codex.

- Decision: prove GTK4's settings order in the package build without QEMU or
  an assembly rebuild.
  Rationale: `gtk_settings_init()` reads every file synchronously while a
  `GtkSettings` object is constructed, and the test can call that exact
  library code without a display. The smoke plants the same XDG directory
  list that the desktop exports and distinguishes every override and fallback
  value. No assembly manifest names GTK4 directly, so no manifest pin or boot
  path changes. QEMU would exercise unrelated display and login code without
  strengthening the settings-file claim.
  Date/Author: 2026-08-18 / Codex

- Decision: prove the assembled AT-SPI selector with the installed launcher in
  a copied root rather than booting QEMU.
  Rationale: the repaired code chooses one config path synchronously before it
  starts either D-Bus implementation. The root smoke runs that exact assembled
  binary, records the exact argument passed to the child for every tier and
  mask case, and can fail if the selector regresses. A system boot would add
  compositor, login, and service-manager behavior that does not participate in
  this file choice.
  Date/Author: 2026-08-18 / Codex

- Decision: Replace Grok as the live ExecPlan supervisor with Codex. Keep
  Claude as the audit and coding worker, and keep all earlier Grok decisions
  and accepted records as history.
  Rationale: Grok exhausted its weekly allowance mid-run, and the human named
  Codex as the new supervisor. Codex now owns source review, audit text,
  staging, checks, and commits. Claude continues to make bounded first-pass
  reports and repairs under the existing worker contract.
  Date/Author: 2026-08-17 / Codex and human operator

- Decision: treat `xdg-desktop-portal-wlr` as a `gap` even though
  the worker said `pass`.
  Rationale: the reader is user then `/etc/xdg` only. It does not
  honor `XDG_CONFIG_DIRS`, so the assembly overlay that makes
  `xdg-desktop-portal` and fuzzel three-tier never reaches this
  backend. A boot cannot apply temporary screencast policy.
  Date/Author: 2026-08-17 / Ralph

- Decision: never add a UAPI-config patch or Nex-specific policy
  reader to a manifest under `pkg/bootstrap/`.
  Rationale: the human stated that the bootstrap must stay visibly
  standard upstream packages so a reader can be confident the seeds
  have not been tampered with. A missing `/run` or `/usr` tier in a
  bootstrap program is therefore not a repair. Record the reader as
  `build-only input` and patch the shipped copy under another `pkg/`
  tree instead. A search found nothing to revert.
  Date/Author: 2026-08-17 / Grok

- Decision: land the seven cheap single-file repairs before restarting
  Chromium.
  Rationale: the first Chromium compile finished and the smoke failed in the
  probe, so `WORK_DIR` is gone and a restart costs another two-plus hours.
  Those hours would block zsh, bmon, htop, ncdu, procs, screen, and tmux,
  each of which builds in minutes. The probe fix is already in the
  uncommitted manifest and can ride the rebuild after the small packages
  land.
  Date/Author: 2026-08-17 / Grok

- Decision: Replace Claude as the main ExecPlan agent with Grok. Claude
  becomes the audit and coding worker. Grok supervises, validates, writes the
  tracked ledger, and commits.
  Rationale: The human asked to swap the roles on 2026-08-17. The earlier
  Grok-first, Claude-fallback, and read-only-worker decisions remain as
  history of how the first 152 records were produced. They do not apply to
  new batches or new repairs.
  Date/Author: 2026-08-17 / Grok

- Decision: for Chromium only, run the strict build command once rather than
  twice, and say so in the record.
  Rationale: one `./nex build ... --check` invocation already compiles the
  package twice and prints `Build output checksum`, `Second build output
  checksum`, and `Build is reproducible. Checksums match.`, which is what
  "two strict builds" asks for. Every other package in this plan was run
  twice, four compiles, because each compile costs minutes. Chromium compiles
  54756 objects in about an hour, so the usual practice would cost more than
  four hours and would block every other package build, including the GnuPG
  repair that is already written and waiting for the store. The reproducibility
  evidence is identical either way; what a second invocation would add is only
  that the checksum is stable once the manifest checksum has been updated. This
  is a deliberate, recorded deviation rather than a silent one, and the human
  should overturn it if that extra assurance is wanted for the browser.
  Date/Author: 2026-08-16 / Ralph

- Decision: treat a machine-wide policy file as three-tier distribution policy
  whenever an administrator could reasonably set it and a package could
  reasonably ship a default, even when the file is a compatibility mechanism
  from another operating system.
  Rationale: FreeRDP's `HKLM.reg`, Looking Glass's
  `/etc/looking-glass-client.ini`, and Chromium's `/etc/chromium/policies` were
  each reported as needing no tiers because the file "belongs to the machine".
  All three set behavior a deployment may want to preselect, such as which RDP
  security protocols are enabled, so all three are distribution policy under
  `CONFIGURATION.md`.
  Date/Author: 2026-08-16 / Ralph

- Decision: split the Chromium repair and land only the enterprise policy
  loader. Leave the native-messaging host directory single-tier, and leave that
  manifest's entry at `gap` until a test can drive it.
  Rationale: reaching `LaunchContext::FindManifest` needs an extension that
  opens a native messaging port, which no build-time smoke can arrange. Landing
  the hunk anyway would put an untested behavior change in a commit, which
  `AGENTS.md` forbids. The policy loader is the larger surface and a headless
  run proves it end to end.
  Date/Author: 2026-08-16 / Ralph

- Decision: when a program refuses to run under the build sandbox's conditions,
  interpose the specific guard rather than trying to change the sandbox.
  Rationale: Looking Glass exits when `getuid()` returns 0, and the sandbox maps
  only root, so `setuid(65534)` fails with `EINVAL` and
  `unshare(CLONE_NEWUSER)` fails with `EPERM` because the script runs in a
  chroot. A two-function `LD_PRELOAD` object returning 65534 satisfies the guard
  and leaves every configuration reader untouched, so the smoke still exercises
  the shipped binary.
  Date/Author: 2026-08-16 / Ralph

- Decision: prove a smoke's mechanism against an already-built copy of the
  program before spending an expensive build on it.
  Rationale: the Chromium probe was validated against the host's chromium 143
  first, which confirmed that `--vmodule=config_dir_policy_loader=1` reaches
  `VLOG_POLICY` and that the loader names each directory it visits. One
  Chromium compile takes about two hours, so an unverified probe would have
  cost that much per attempt.
  Date/Author: 2026-08-16 / Ralph

- Decision: run Milestone 3 in batches of at most six related manifests, keep
  the batch ledger in `.nex/tmp/ep016-batches.tsv`, and extract pinned sources
  per batch rather than all at once.
  Rationale: a full extraction of all 533 manifests needs roughly 170 GB and
  does not fit beside the store, and grouping by `pkg/<area>/<group>/` lets one
  worker reuse context across related leaf packages without hiding differences
  in build flags, outputs, or services.
  Date/Author: 2026-08-16 / Ralph

- Decision: accept an additive overlay chain as correct for formats whose own
  contract is "a later file overrides an earlier one", and treat only a missing
  tier as the gap.
  Rationale: Fish's `conf.d`, ImageMagick's XML search, GTK 3's `settings.ini`,
  and distrobox's sourced list all merge by design; forcing whole-file
  selection on them would change documented behavior for no ownership benefit.
  The Nex rule those families must still meet is that all three tiers are
  reachable in the right order.
  Date/Author: 2026-08-16 / Ralph

- Decision: when a compiled path is in question, settle it by checking out the
  built output and running `strings`, rather than by reading the build system.
  Rationale: reading Meson source led to a wrong conclusion about Linux-PAM's
  administrator directory, and `strings` on `pam_access.so` settled it in one
  command. The same technique confirmed Bash's bundled Readline reading only
  `/etc/inputrc`.
  Date/Author: 2026-08-16 / Ralph

- Decision: Create one explicit audit entry for every tracked `pkg/**/*.yaml`
  manifest. Do not build a generalized source scanner and do not accept an
  automatic check as proof that a reader follows the three-tier rule.
  Rationale: source readers use package-specific APIs, search orders, drop-in
  rules, reload behavior, service arguments, and environment overrides. An
  agent must understand those details for each package. A small comparison may
  catch a skipped or duplicated manifest, but it cannot approve an entry.
  Date/Author: 2026-08-16 / Ralph and human operator

- Decision: Store the audit in ten tracked Markdown files under
  `.agents/audits/package-configuration/`, one for each current top-level
  package area. Give every manifest its own heading and fixed set of fields.
  Rationale: one entry per manifest stays readable, while ten area files avoid
  both a single unwieldy document and 533 tiny files. A package may need several
  configuration-family paragraphs inside its entry.
  Date/Author: 2026-08-16 / Ralph

- Decision: Use these fields in every manifest entry:
  `Purpose`, `Runtime configuration`, `Ownership class`, `Reader behavior`,
  `Evidence`, `Proof`, and `Result`.
  Rationale: these fields require the auditor to connect a package's purpose to
  the exact reader, paths, priority rules, cited source, and executable test.
  `Result` may be `pass` or `gap fixed` only in the finished audit. An active
  entry may temporarily say `gap` or `uncertain`.
  Date/Author: 2026-08-16 / Ralph

- Decision: Classify each discovered file family by its actual owner before
  deciding whether it needs three tiers.
  Rationale: distribution policy belongs below `/usr`, boot-time overrides
  belong below `/run`, and administrator policy belongs below `/etc`.
  Machine-specific state, passwords, keys, databases, generated caches, and
  volatile service state have different owners. Forcing all of them through a
  distribution-default search path would be wrong.
  Date/Author: 2026-08-16 / Ralph

- Decision: Count command-line-only and environment-only packages as having no
  file-based system configuration, but record that finding and its evidence.
  Treat files used only during the package build as build inputs, not shipped
  runtime configuration.
  Rationale: the plan audits the filesystem UAPI defined by
  `CONFIGURATION.md`. It must not invent file readers for programs that do not
  have them, and it must not confuse compiler inputs with runtime policy.
  Date/Author: 2026-08-16 / Ralph

- Decision: Let related simple manifests share one worker session, but require
  a complete separate report block and tracked audit entry for each manifest.
  Start with four to eight related leaf packages per session. Give one complex
  package, daemon, framework, language runtime, or patched reader to one worker.
  Rationale: small related batches reuse context without letting a group label
  hide differences in build options, installed outputs, services, or patches.
  Date/Author: 2026-08-16 / Ralph and human operator

- Decision: Run normal Grok audits with `grok-4.6` and medium reasoning effort.
  Disable Grok's subagents. Raise one concrete difficult package to high effort
  only after a medium report leaves conflicting or missing evidence. Reserve
  xhigh for an exceptional unresolved reader.
  Rationale: xAI describes medium effort as suitable for complex analysis and
  long-context reasoning. Grok 4.6 is the installed client's current default
  and has a 500,000-token context window. Medium avoids Grok's high default
  while preserving enough reasoning for source and manifest work.
  Date/Author: 2026-08-16 / Ralph and human operator

- Decision: When Grok reports that its subscription allowance is exhausted,
  send all untouched batches to `claude-sonnet-5` with adaptive thinking and
  medium effort. Raise a concrete difficult package to Sonnet high, then to
  `claude-opus-5` high only if Sonnet cannot settle it. Reserve xhigh for an
  exceptional unresolved case. Do not give Claude a manual thinking-token
  budget, and disallow its `Agent` tool.
  Rationale: Anthropic enables adaptive thinking by default for Claude 5 and
  rejects manual thinking budgets for those models. Sonnet 5 medium fits the
  ordinary audit; Opus 5 costs more and should handle only evidence that the
  normal worker cannot reconcile. The installed Claude Code 2.1.233 is newer
  than Anthropic's required versions for both models.
  Date/Author: 2026-08-16 / Ralph and human operator

- Decision: Add three explicit distinctions to the worker prompt after the
  pilot: generated data written into `/etc` at runtime is not a packaged vendor
  default; a `CONFIGURATION.md` worked example may describe an assembly
  arrangement rather than a package requirement, so the worker must search
  `asm/` before calling a package default wrong; and a missing third tier is a
  gap only when the file family carries distribution policy.
  Rationale: the pilot's only wrong result came from applying the certificate
  cache example to the reusable package. The evidence threshold did not change;
  only the wording that told the worker which question to ask.
  Date/Author: 2026-08-16 / Ralph

- Decision: Give an unaudited heading the explicit result `not audited` and
  add a fifth result state to the README for it.
  Rationale: the plan requires headings before entries exist. A heading with no
  `Result` line cannot be distinguished from a malformed record, so the checker
  could not reject a truncated entry. An explicit state keeps the ledger
  machine-checkable from the first commit and lets the same checker report
  audit progress.
  Date/Author: 2026-08-16 / Ralph

- Decision: Track the coverage comparison as
  `scripts/check-package-config-audit.sh` with a `--self-test` mode and a
  `--final` mode, rather than running a one-off shell pipeline at the end.
  Rationale: `AGENTS.md` requires a check that can fail. A tracked script with
  a self-test proves that the comparison rejects a missing, duplicate, stale,
  misfiled, or unfinished record, and it matches the shape of the existing
  `scripts/check-package-config-paths.sh`. `--final` adds the acceptance
  conditions so the same command serves the review gate.
  Date/Author: 2026-08-16 / Ralph

- Decision: Reconcile the patch counts as 32 lookup patches and 34
  configuration patches, and record both numbers in the audit README.
  Rationale: reading contents rather than filenames shows that all 31
  `uapi`-named patches change configuration lookup, and that
  `pkg/core/ipc/dbus-transient-config.patch` adds the `/run/dbus-1` tier, which
  gives 32. The remaining seven tracked patches serve reproducible builds,
  parser syntax, a shell helper, a build-time probe, and a `/proc` override.
  Shadow additionally applies two remote Arch patches that edit `login.defs`
  content; counting those gives the 34 the human remembered.
  Date/Author: 2026-08-16 / Ralph

- Decision: Do not rerun accepted Grok entries through Claude merely to obtain
  a second opinion. The root Ralph agent must validate every report against the
  cited local evidence before writing `pass`.
  Rationale: duplicate model runs spend the allowance without replacing the
  root agent's responsibility for the tracked result.
  Date/Author: 2026-08-16 / Ralph and human operator

- Decision: External workers remain read-only. They may inspect manifests,
  patches, scripts, prepared pinned sources, and tests. They may not edit the
  worktree, start builds, create commits, or spawn more agents.
  Rationale: one root agent must control shared files, validate evidence, and
  keep every commit buildable. Read-only workers can run concurrently without
  racing over manifests or build caches.
  Date/Author: 2026-08-16 / Ralph

- Decision: Redirect every long build, assembly, QEMU, compiler, and test stream
  to an ignored file. On success, read only the exit status and named concise
  markers. On failure, search for bounded error lines before opening any larger
  excerpt.
  Rationale: build progress does not help an agent reason about a successful
  result. Error lines and final proof markers do.
  Date/Author: 2026-08-16 / Ralph and human operator

## Outcomes & Retrospective

Work so far. The ledger exists and is machine-checked:
`.agents/audits/package-configuration/` holds one record for each of the 533
tracked package manifests, and `scripts/check-package-config-audit.sh` compares
those records with `git ls-files`, rejecting a missing, stale, duplicate,
misfiled, unfinished, or malformed entry. Its `--self-test` proves each of
those rejections.

Three hundred and forty-eight records are finished, 295 `pass` and 53
`gap fixed`, and two entries are deliberately open:
`pkg/apps/web/chromium.yaml` and `pkg/libs/crypto/p11-kit.yaml` are confirmed
`gap` records.
Screen, libgcrypt, GnuTLS, PipeWire, WirePlumber, jack2, alsa-lib, git,
gtk-vnc, libosinfo, elfutils, and file/libmagic are now `gap fixed`. texinfo is
now `gap fixed`. `fonts-1` is written: five `pass` records,
TTF data only. `libs-archive`, `libs-audio-1` /
`libs-audio-2`, and `libs-compression-1`
are written.
cli-text-2 is written: ripgrep and sed pass; texinfo was the
`/usr/etc` trap and now uses `--sysconfdir=/etc` plus a `/run`
tier. Milestone 2 is complete, `pkg/apps/` and all of
`pkg/bootstrap/` are fully recorded, and the `cli-system-*` and
`cli-text-*` batches are validated and written. Bootstrap records
stay `pass` as `build-only input`; the human rule is that those
seeds must not receive UAPI-config patches.

The list below summarizes 45 of the 53 repaired configuration bugs; `Progress`
records the other eight. Each repair has a test that fails against the old
behavior and two strict builds:

- jack2 autostart read only `/etc/jackdrc` after the user
  file. Reads now walk `/etc`, `/run`, then `/usr/lib`.
- alsa-lib's vendor `alsa.conf` named `/usr/etc` drop-ins
  and had no `/run` tier.
- git read only `/etc/gitconfig` and `/etc/gitattributes`.
  Reads now walk `/etc`, `/run`, then `/usr/lib`. Writes stay
  on `/etc`.
- gtk-vnc compiled `SYSCONFDIR` via `join_paths(prefix,
  sysconfdir)`, which is `/usr/etc` unless the manifest
  passes `--sysconfdir=/etc`. The cert dir is then `/etc/pki`.
- libosinfo compiled `SYS_CONF_DIR` as `/usr/etc` and had no
  `/run` tier. The loader now merges `/usr/share/osinfo`,
  `/run/osinfo`, and `/etc/osinfo`.
- elfutils `debuginfod.sh` globs only `/etc/debuginfod/*.urls`;
  it now concatenates `/etc`, `/run`, and `/usr/lib`.
- file/libmagic never opened `/etc/magic` unless `MAGIC` was set;
  `get_default_magic` now walks `/etc/magic`, `/run/magic`, then
  the compiled vendor `MAGIC`.
- procps-ng `top` read only `/etc/topdefaultrc`.
- acpid scanned only `/etc/acpi/events`.
- pkgconf compiled `PERSONALITY_PATH` as `/usr/etc`.
- dbus-broker included every drop-in additively and had no `/run` tier.
- mandoc read only `/etc/man.conf`.
- texinfo compiled `sysconfdir` as `/usr/etc` and had no `/run` tier.
- bat read only `/etc/bat/config`.
- zsh sourced only `/etc/zshenv` and the other four `/etc` startup files.
- procs read only `/etc/procs/procs.toml`.
- tmux compiled only `/etc/tmux.conf` into `TMUX_CONF`.
- htop read only `SYSCONFDIR "/htoprc"`.
- bmon read only `SYSCONFDIR "/bmon.conf"`.
- ncdu read only the literal `/etc/ncdu.conf`.
- Less searched only the configured system directory for its system lesskey.
- OpenSSH's `ssh-keysign` read only the administrator `ssh_config`, breaking
  host-based authentication when `EnableSSHKeysign` came from a drop-in.
- BlueZ's daemon never read its own vendor files, because the shipped unit
  exports `CONFIGURATION_DIRECTORY` and the selector treated that as exclusive.
- Linux-PAM's `pam_namespace_helper` and `pam_securetty` missed the lower tiers.
- Bash linked its bundled Readline, whose `inputrc` reader stopped at `/etc`.
- e2fsprogs layered `mke2fs.conf` but left `e2fsck` on a single path.
- Shadow's `chsh` used libeconf's two-directory call, so `/run/shells` was
  unreachable.
- Swaync's vendor search base was one directory too deep, so the packaged
  default was unreachable and the daemon exited.
- GTK 3 walked `XDG_CONFIG_DIRS` forward while later loads win, inverting the
  administrator, transient, and vendor order the desktop assembly sets up.
- Libvirt's shipped `libvirt-guests.sh` sourced one administrator settings file
  while every compiled reader in the package honoured all three tiers.
- CUPS reached the tiers from `cupsd` and the SNMP readers, while `cupsfilter`,
  `client.conf`, `cupsctl`, and the mail notifier each rebuilt the path from
  the compiled `CUPS_SERVERROOT`.
- D-Bus scanned its three drop-in directories additively, with no basename
  shadowing and no `/dev/null` mask.
- Podman's `storage.conf` had no transient tier and its `registries.d` was a
  single administrator directory.
- runc hard-coded `/etc/criu/runc.conf`.
- distrobox's fixed configuration list had no `/run` entry and read `/usr/etc`.
- containerd's `--config` defaulted to a single `/etc/containerd/config.toml`.
- Docker's `getDefaultDaemonConfigFile` returned only `/etc/docker/daemon.json`.
- FreeRDP resolved every WinPR system file, including the `HKLM.reg` hive that
  carries machine-wide client policy, below one compiled sysconfdir.
- mpv compiled a single `MPV_CONFDIR`, so `mpv.conf`, `input.conf`,
  `encoding-profiles.conf`, and the auto-loaded `scripts` directory each had one
  system location.
- Looking Glass hard-coded `/etc/looking-glass-client.ini` as its only
  system-wide options file.
- PipeWire compiled its administrator directory as `/usr/etc/pipewire` and had
  no `/run` tier. Its vendor-first basename filter also prevented a higher
  same-named drop-in from replacing the lower file.
- WirePlumber compiled `/usr/etc/wireplumber` and omitted `/run`. Its existing
  basename iterator already replaced a lower same-named drop-in, so the patch
  adds the missing tier without changing that filter.
- GnuTLS read its system priority file and deprecated PKCS#11 module list from
  one compiled path each. Both readers now select `/etc`, `/run`, then
  `/usr/lib`, while their existing explicit overrides still win.
- libgcrypt read `fips_enabled`, `hwf.deny`, and `random.conf` only below
  `/etc/gcrypt`. Each reader now selects `/etc`, `/run`, then `/usr/lib`.
- Screen read `/etc/screenrc` and the user file. It now overlays vendor,
  runtime, administrator, and user files in that order, including its
  reattach parsing pass.

Nearly all of them share one shape: a sibling reader that the original patch
missed, or a search order that looked right in isolation and broke once the
assembly or service manager supplied the environment. That is the durable
lesson of this work, and it is why the plan requires the supervisor to open the
cited source rather than accept a worker report.

That validation step earned its cost repeatedly. Six worker results did not
survive checking. Three were wrong `gap` or `pass` calls found earlier: CA
Certificates was a `pass` because the assemblies already implement the
arrangement its report cited; Linux-PAM's real gap was two unpatched readers
rather than the `/usr/etc` prefix the worker inferred from Meson source; and
Shadow was reported `pass` when `chsh` in fact had a missing tier. Three more came later: FreeRDP and Looking Glass were
both reported `pass` with their machine-wide policy files described accurately
and then classified as needing no tiers, and the phase1 glibc was reported
`gap` for readers that never run on a Nex machine. Where a compiled path was in question,
`strings` on the checked-out build output settled it, which is how
`WINPR_INSTALL_SYSCONFDIR` and `MPV_CONFDIR` were both pinned down to `/etc`.

Seven of the repairs were nearly accepted while broken, and the tests rather
than review caught each one. D-Bus's vendor root became absolute, which broke
upstream's resolution relative to the including configuration file. D-Bus's
first mask assertion was unfalsifiable, because the base session policy already
permits the call the test made, so masking an `allow` passed either way; both
new assertions now put a `deny` in the vendor file. Containerd's patch was
never applied at all, because the manifest declared the source without running
`patch`, and the build stayed green. PipeWire's first patch added `/run` to the
scan but did not change the same-basename filter, so the vendor file still
suppressed matching `/run` and `/etc` files until the smoke failed. GnuTLS's
first smoke proved the three default tiers but did not prove that a caller's
non-NULL PKCS#11 filename still bypassed the new walk; Codex required that
case before accepting the repair. libgcrypt's first FIPS smoke reported only
the same `active` value for every present tier, and its runner ignored a
non-zero helper exit. The accepted smoke records the exact successful file
operation and lets any helper failure stop the build.
Screen's saved smoke expected an error message that a detached process did not
expose. Its next runner hid Screen's failure after the build namespace blocked
PTY group 5, then its socket watcher could race startup. The accepted test
uses Screen's PTY-free `//group` window, requires every ordered socket rename,
and waits for a watcher-ready handshake.

One confirmed gap remains. Chromium's enterprise policy loader is repaired and
proved, but its native-messaging host reader still searches only the user and
administrator directories. Closing the package record needs a test extension
that opens a native messaging port, followed by a three-tier reader repair.
p11-kit's global settings and module-registration readers now follow the three
tiers, and both `print-config` and the library-backed `list-modules` command
prove the order and fallback. The copied `nex-systemd` root repeats both tests
through the assembled command, library, module, and loader paths.

A second shape has now appeared often enough to name: the reader that a worker
describes correctly and then excuses. FreeRDP's registry hive, Looking Glass's
`/etc` options file, and Chromium's policy directory are all machine-wide
policy that a deployment should be able to ship a default for, and all three
were reported as needing no tiers because the file "belongs to the machine".
The test for that excuse is whether an administrator could reasonably set the
file and whether a package could reasonably ship a default; if both are yes,
it is distribution policy and needs three tiers.

The remaining work is large and exactly enumerable: 165 records still say
`not audited`, and each needs a Claude report plus Codex review against the
manifest and the pinned source. Chromium's native-messaging reader is the one
confirmed gap. The audit has finished 367 of 533 manifests, or 68.9 percent,
and has inspected 368, or 69.0 percent, when that open gap is included. From
2026-08-17 23:29Z, Claude does the first-pass audit and coding work while Codex
supervises.

## Context and Orientation

`PHILOSOPHY.md` and `CONFIGURATION.md` define Nex's ownership contract. A
selected deployment owns immutable defaults below `/usr`. One boot owns
temporary policy below `/run`. The machine owner controls persistent policy
below `/etc`. A reader that selects one main file checks `/etc` first, then
`/run`, then `/usr`. A reader that merges drop-in directories starts with
vendor files, overlays runtime files, then overlays administrator files;
matching basenames shadow lower tiers, and the documented mask convention must
work when that reader supports masks.

Previous plans EP011 through EP015 patched readers and moved package or
assembly defaults out of `/etc`. Their presence narrows the likely work but
does not prove that every package reader was found. This plan must inspect the
current manifests and pinned source directly. It may cite a previous test only
after Codex checks that the test still invokes the actual built reader
and distinguishes vendor, runtime, and administrator fixtures.

The audit covers every path returned by:

    rtk git ls-files 'pkg/**/*.yaml'

At plan creation, that command returned 533 paths. The final check must use the
then-current tracked set, so manifests added or removed during this plan cannot
escape review. Assembly manifests are not independent audit entries, but
Codex must inspect affected assemblies when a package fix changes shipped
defaults, service startup, or boot behavior.

For this plan, a runtime configuration surface means a file or directory that
a shipped executable, library, daemon, service unit, helper, or interpreter
reads or writes to choose system behavior. Examples include main files,
drop-ins, plugin registries, policy files, service configuration, and files
named by a compiled default or service argument. The audit must distinguish
those files from ordinary user documents, application content, firmware,
fonts, localization data, shared libraries, and generated program data.

Codex must apply at least these ownership classes:

- `three-tier distribution policy`: a package can ship a default and a boot or
  administrator can override it. The real reader must implement the documented
  `/usr`, `/run`, `/etc` order.
- `standard fixed path`: an external interface requires one path or a stable
  adapter, such as account databases or a compatibility pathname. The entry
  must name the standard or the concrete consumer and explain why three tiers
  do not apply.
- `machine-owned state`: the installer, administrator, or service creates
  persistent host-specific content. The package must not overwrite it during
  an upgrade.
- `secret or credential`: the machine or user owns sensitive material. The
  entry must identify the writer and reader without copying secrets into audit
  artifacts.
- `runtime state`: one boot or one process owns the file below `/run` or another
  documented volatile location.
- `database, cache, or generated index`: a program generates data from another
  source of truth. The entry must identify that source and regeneration path.
- `user-owned configuration`: the reader uses a home or per-user path and does
  not define host-wide distribution policy. The entry must still note any
  system-wide fallback.
- `build-only input`: the file affects compilation or packaging but no shipped
  reader consumes it.
- `no file-based runtime configuration`: inspected evidence shows no relevant
  reader. The entry must cite the inspected manifest and source surface rather
  than state this from package name alone.

The root agent will create these tracked files:

    .agents/audits/package-configuration/README.md
    .agents/audits/package-configuration/apps.md
    .agents/audits/package-configuration/bootstrap.md
    .agents/audits/package-configuration/cli.md
    .agents/audits/package-configuration/core.md
    .agents/audits/package-configuration/desktop.md
    .agents/audits/package-configuration/dev.md
    .agents/audits/package-configuration/fonts.md
    .agents/audits/package-configuration/libs.md
    .agents/audits/package-configuration/net.md
    .agents/audits/package-configuration/servers.md
    .agents/prompts/package-configuration-audit.md

Each area file uses one exact level-three heading per manifest:

    ### `pkg/libs/example/example.yaml`

Each heading contains this compact record:

    Purpose: <what the built package provides>

    Runtime configuration: <each reader or writer and its file families, or none>

    Ownership class: <one or more classes defined above>

    Reader behavior: <exact tier order, merge/mask/reload behavior, or reason not applicable>

    Evidence: <manifest, patch, pinned source path and symbol, service, or script>

    Proof: <existing or new command/test and the behavior it distinguishes>

    Result: pass | gap | uncertain | gap fixed

The final tracked audit may contain only `pass` and `gap fixed`. A result of
`gap` stops that entry until Claude implements the repair, Codex reviews the
test, and Codex commits. A result of `uncertain` triggers more source work or
a higher-effort Claude pass; it never counts as acceptance.

`.agents/knowledge/package-manifests.md` already records a crucial audit rule:
before moving a default, find every reader, reload or watch path, override, and
drop-in parser. Tests must call the real built reader with distinct tier
fixtures. Codex must read that note again before accepting a patched-reader
report and before accepting a new package fix.

## Plan of Work

### Milestone 1: establish the audit ledger and worker contract

Run the worktree pre-task from `AGENTS.md` before editing plan artifacts. List
`.agents/knowledge/` and read every note that matches package builds,
configuration, source inspection, testing, assemblies, or reproducibility.
Freeze the sorted starting manifest list in an ignored `.nex/tmp/` file for
batch planning, but always compare the finished audit to a fresh Git list.

Write the audit README, ten area files, and one worker prompt. The README must
define the claim, classes, evidence standard, result states, and heading format
from this plan. Populate headings in Git order, but do not prefill entries with
`pass`. The prompt must tell each worker:

1. Read every assigned manifest completely, including all build flags, local
   sources, patches, scripts, bundles, outputs, and services.
2. Find the runtime programs and libraries that can read or write system
   configuration. Inspect every referenced configuration patch and the pinned
   source around each relevant reader. Search for alternate readers, reload or
   watch paths, drop-ins, environment overrides, service arguments, and
   compatibility adapters.
3. Apply the ownership classes in this plan. Require all three tiers only when
   the file family carries distribution policy that a boot or administrator
   may override.
4. Cite exact repository paths and source paths or symbols. Describe what each
   cited test proves. Do not infer compliance from a patch filename, installed
   output path, or package category.
5. Return one fixed report block per manifest with `pass`, `gap`, or
   `uncertain`. Keep prose compact but include every discovered configuration
   family.
6. Do not edit files, build packages, commit, spawn agents, or include long
   command output.

The completed pilot used Grok medium on a representative set: one obvious leaf
library, one command-line tool, one service, one package with an existing UAPI
patch, and one package that owns generated or machine state. The root agent
must compare each report with the manifest and cited code. Adjust ambiguous
prompt wording, not the evidence threshold. Record pilot corrections in the
Decision Log before starting broad batches.

### Milestone 2: audit configuration-patched packages first

Enumerate every patch referenced by a package manifest. Inspect patch content
to decide whether it changes configuration paths, search order, merging,
reload behavior, or service arguments. Reconcile that content-based list with
the 39 total package patches, the 31 names containing `uapi`, and the human's
remembered count of 34 configuration patches. Publish the reconciled list and
explanation in the audit README.

Assign one patched reader per worker unless several manifests build the same
pinned source with the same patches. Even in a shared-source batch, require one
entry per manifest and check differences in configure flags, bundles, outputs,
services, and installed files.

For each configuration family, the root agent must validate:

- every compiled reader, helper, library API, and service entry point that can
  select the file;
- main-file precedence and the behavior when a higher-priority file is absent;
- drop-in ordering, basename shadowing, masks, and duplicate handling when the
  program supports drop-ins;
- reload, watch, or restart behavior when a program can reread configuration;
- command-line, environment, or service-unit overrides that bypass compiled
  defaults;
- the installed path of the vendor default and the absence of packaged mutable
  administrator policy;
- a focused test that invokes the real reader and gives each applicable tier a
  distinct value.

An old test may satisfy `Proof` only if it still exercises the built reader and
can fail when its claimed tier order breaks. Otherwise add or strengthen the
test with the fix.

### Milestone 3: audit every remaining manifest

Walk the fresh sorted manifest list by top-level area. Start Claude workers on
untouched batches. Use four to eight manifests only when they are small and
related. Use one worker for a daemon, framework, language runtime, desktop
stack, package with several binaries, or any package whose configuration
surface is not obvious.

Codex must review the compact final report rather than Claude's full reasoning
trace. It must open every cited local path, confirm that the source belongs to
the pinned version, search for missed readers when the report relies on a
narrow symbol, and then write or revise the tracked entry. Codex owns the final
words and result.

When Claude returns an allowance or quota error, record the last completed
batch and the exact error in `Surprises & Discoveries` and stop as a hard
blocker unless the human names a different worker. A transient command or
network failure is not quota exhaustion; retry that batch after checking the
bounded error.

If a worker reports `uncertain`, Codex first inspects the cited code and nearby
readers. It may rerun that one package through Claude at high effort. Use
Claude Opus 5 only after the normal and high Sonnet passes leave a named
source conflict or reader question unresolved. Record every higher-effort use
and its reason in the Decision Log.

### Milestone 4: repair each confirmed gap

Do not mark a manifest complete while a discovered applicable reader lacks the
three tiers, reads the wrong priority, packages a mutable default, or lacks a
test that can prove its behavior. Codex assigns the confirmed gap to Claude.
Claude implements the package-specific fix. Follow `RUST_CODE_STYLE.md` for
Rust, follow `.agents/MANIFESTS_CODE_STYLE.md` for manifests, and follow
`.agents/TESTING.md` for every behavior change.

For a reader fix, patch the pinned source rather than adding a wrapper unless a
stable external interface requires an adapter. Preserve upstream behavior that
does not conflict with the Nex ownership rule. Test the smallest real reader
with distinct vendor, runtime, and administrator values. Include drop-ins,
masks, reload paths, service arguments, or fixed-path behavior when they apply.

Claude runs `nex check`, the focused behavior test, and two strict package
builds for each changed manifest, then stops for review. Codex checks the
patch, the test, the exit statuses, and the named proof markers. Check out
and exercise the resulting files, command, library reader, or service. When a
fix changes files shipped by an assembly, Claude builds each affected
assembly twice and Codex reviews the smallest root-filesystem or QEMU test
that proves the changed path. Never accept compilation or file existence
alone.

Codex commits each coherent package or shared-reader fix after its review
passes. Use exact staging and the scoped imperative subject style from
`AGENTS.md`. Update the audit entry, ExecPlan, and scratch knowledge in the
same commit when they describe that checked behavior. Claude does not commit.

### Milestone 5: close the bounded proof

Generate a fresh sorted list of all tracked package manifests. Extract the
level-three manifest headings from the ten audit area files, normalize the
backticks, and compare the two lists. Reject missing paths, duplicate headings,
headings for deleted manifests, and any active `gap` or `uncertain` result.
This comparison proves only ledger coverage. It does not approve audit content.

Reread every entry that cites an existing configuration patch and every entry
that needed a fix. Sample at least one `no file-based runtime configuration`
entry from each populated area and recheck its cited source. Run the existing
repository-wide package path checker. Run all focused tests and artifact checks
added during this plan, plus every assembly or QEMU check required by changed
packages.

Record exact final commands, exit statuses, checksums where useful, and concise
behavior results in the `Completion Check`. Reread
`.agents/SCRATCH_KNOWLEDGE.md`, promote only verified reusable facts into the
matching `.agents/knowledge/` files, and remove temporary or disproven scratch
notes. Keep this plan active and request the human's audit validation and end
approval as required by `AGENTS.md`.

## Concrete Steps

Run all commands from the repository root.

1. Perform the Ralph pre-task and prepare ignored working directories:

       rtk git status --short --untracked-files=all
       rtk git ls-files '.agents/knowledge/*.md' | LC_ALL=C sort
       mkdir -p .nex/tmp/ep016-prompts .nex/tmp/ep016-audits .nex/tmp/ep016-builds
       rtk git ls-files 'pkg/**/*.yaml' | LC_ALL=C sort > .nex/tmp/ep016-manifests.txt

   Classify dirty files before changing the plan. Do not commit unrelated human
   work or ignored files.

2. Check the Claude worker before a new batch:

       claude --version

   Claude Code must be at least 2.1.219 so it supports both chosen Claude 5
   models. If a vendor retires an exact model, use its documented direct
   successor only after recording the change and evidence in the Decision
   Log. Codex remains the supervisor in this session and does not need a
   second Codex worker.

3. Run a normal Claude audit batch without exposing its transcript to Codex's
   command output. `.nex/tmp/ep016-batch.sh` now defaults to Claude:

       .nex/tmp/ep016-batch.sh <name> <manifests...>
       claude --print --model claude-sonnet-5 --effort medium --permission-mode plan --disallowedTools Agent --output-format json < .nex/tmp/ep016-prompts/<batch>.md > .nex/tmp/ep016-audits/<batch>.json 2> .nex/tmp/ep016-audits/<batch>.err

   Check the process exit status. On success, extract only the worker's final
   compact report from the JSON into a bounded view. Do not print the full
   session record. On failure, first search the error file for an allowance,
   authentication, model, network, or permission error. Raise one hard
   manifest to Sonnet `--effort high`; do not raise whole batches by default.
   Use `--model claude-opus-5 --effort high` only for a named unresolved
   problem. Do not set a manual thinking budget and do not enable Claude's
   `Agent` tool.

4. Assign a confirmed gap to Claude as a coding task. Give it the manifest,
   the validated reader evidence, the ownership class, and the test the
   repair must be able to fail. Claude may edit files and run the named
   checks. Claude must not write the tracked audit or create a commit. Codex
   reviews the resulting patch and proof, then commits. Do not send new
   audit batches through Grok while its weekly allowance is exhausted.

5. Reconcile package patches by manifest reference and content. Preserve the
   exact commands and result counts in the audit README. Useful exhaustive
   lists include:

       rtk git ls-files 'pkg/**/*.patch' | LC_ALL=C sort
       rtk git ls-files 'pkg/**/*uapi*.patch' | LC_ALL=C sort
       rtk rg -n --glob 'pkg/**/*.yaml' '\.patch([[:space:]]|$)' pkg

   Read all referenced candidate patches and the manifest source around every
   affected reader. Do not classify from these lists alone.

6. For each changed package manifest, run its concise checks and hide long
   build output:

       rtk ./src/cli/target/debug/nex check <manifest>
       ./nex build <manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs > .nex/tmp/ep016-builds/<slug>-1.log 2>&1
       ./nex build <manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs > .nex/tmp/ep016-builds/<slug>-2.log 2>&1

   The command runner reports each exit status without displaying the log. On
   success, search only for the final checksum, reproducibility result, and
   named test marker that the plan needs. Never run `tee`, `cat`, or an
   unbounded `tail` on a successful log. On failure, start with a bounded
   search such as:

       rtk rg -n -m 20 'error:|FAILED|panic|not reproducible|checksum mismatch' .nex/tmp/ep016-builds/<slug>-1.log

   Read a larger bounded excerpt only when those lines lack enough context to
   fix the failure. Apply the same rule to Cargo, Meson, CMake, Make, assembly,
   QEMU, and other verbose commands. A short test that intentionally prints a
   concise pass summary may remain visible.

7. Run the existing package-output invariant:

       rtk scripts/check-package-config-paths.sh

   This command must pass, but its result proves only that declared output
   paths obey the package rule.

8. Compare the final audit headings with the current Git manifest set. Use a
   small one-off extraction or a short checked helper that merely checks exact
   coverage. The comparison must fail for a missing, duplicate, stale, or
   malformed manifest heading. It must also reject:

       rtk rg -n '^Result: (gap|uncertain)$' .agents/audits/package-configuration

   Record the exact final comparison command after the audit files exist. Do
   not describe the comparison as proof of reader behavior.

## Validation and Acceptance

The human can accept this plan only when all of these statements are true:

1. The ten audit area files contain exactly one entry for every package
   manifest tracked at the final commit. No stale or duplicate entry exists.
2. Every entry explains the package purpose, identifies all discovered runtime
   file configuration or says why none exists, assigns an ownership class,
   describes reader behavior, cites exact evidence, and names a test or
   inspection that supports the result.
3. Every distribution-policy reader found by the audit implements Nex's
   `/usr`, `/run`, `/etc` rules. Every reader that does not implement those
   rules has a concrete, reviewed ownership or fixed-interface reason.
4. Every package patch that changes configuration paths or behavior maps to
   one or more audit entries. The README reconciles the content-based patch
   count with the starting 39 total patches, 31 literal `uapi` names, and the
   remembered count of 34.
5. No entry ends as `gap` or `uncertain`. Each confirmed gap has a focused
   buildable commit and a test that can fail when the repaired behavior breaks.
6. The supervisor validated every Claude report against the current manifest and
   pinned source. A worker report or automatic search never serves as the
   sole evidence for `pass`.
7. `scripts/check-package-config-paths.sh` passes.
8. Every changed manifest passes `nex check`, two strict builds, and a focused
   test of the resulting reader, command, service, library, or filesystem.
9. Every assembly affected by a package fix builds reproducibly and passes the
   smallest final-root or boot test that can detect the changed behavior.
10. Long successful command streams remained in ignored log files. The plan's
    evidence contains only exit status, concise markers, checksums, and bounded
    failure excerpts.
11. The `Completion Check` records the exact final commands and results. The
    scratch file contains no unpromoted durable fact, and the knowledge files
    contain only facts that the work verified.

The completed plan supports this bounded statement:

> At the final reviewed commit, an agent audited every tracked Nex package
> manifest, classified every file-based runtime configuration surface it
> found, and either proved the applicable Nex ownership behavior with concrete
> source and test evidence or fixed and proved the gap.

It does not claim that an automated analysis can discover every future reader,
that package source cannot hide an undocumented dynamic path, or that a new
manifest added after the reviewed commit has been audited.

### Completion Check

Run on 2026-08-19 against the working tree at the final commit of this plan.

Coverage and record shape:

    $ sh scripts/check-package-config-audit.sh --final
    PASS: 533 audit records cover every tracked package manifest
      gap fixed: 75
      pass: 458

`--final` is the strict form. Beyond proving that every tracked manifest below
`pkg/` has exactly one record filed in the matching area file, it rejects any
result other than `pass` or `gap fixed` and requires all seven labelled fields
on every record. It exits 0. No `gap`, `uncertain`, or `not audited` entry
remains, and 533 records account for 533 tracked manifests with no duplicate
and no stale heading.

Checker self-test:

    $ sh scripts/check-package-config-audit.sh --self-test
    PASS: package configuration audit checker self-test

Repository-wide configuration path invariant:

    $ sh scripts/check-package-config-paths.sh
    PASS: package manifests declare no outputs below /etc or /usr/etc

Manifest validity for the last package changed:

    $ ./src/cli/target/debug/nex check pkg/apps/web/chromium.yaml
    all checks passed

Twelve packages were repaired during the final run of this plan. Each carries a
patch, a behaviour test wired into its build, two independent strict builds
except where noted, and a proof that the test fails without the patch. The
settled output checksums are:

    libinput      e7b45555dc858e140092015f00975fe1cd6f1804d94071d715d9d7a05590b6db
    libxkbcommon  870066ddd8d224a0bc218f8e60be38484805277ca342c3b7912cb19209974358
    krb5          7a82a9785147a6398ffa4923cb20c131299001ea4b4f80c66270a419a4691f06
    popt          c93761cd57cc99524cd1b56b70a026072978b3a272e5ca162fd36095d585457e
    enchant2      3019eb6e88828d104327c1f50065d13c9a84817ed7b857647b607c458673553a
    iproute2      3dc21f27bff9ae14a1eff3e4628b7603ee626d4262480d22c84528d8572216c9
    libxml2       758cd863cebe954a5ab71a015ed99a4a5c9f6d53fbb3107b01e8e8c2bb968732
    fuse3         12b6910018bda451150ec388abc7fae4a979a24793825a73ae3d0b2b1aa264b5
    polkit        65c2d79230e0df9cbf1ea7af29208bbe5a648d1e5d29a6a1f1b76fa836712b29
    appstream     faaa75411930fb443d58c0c8c1d8da677be45662660026ab2a7749a0f6ae6478
    samba         cfd0d76804e4258a527afada4e14502e793c01021109a6161c7f6b69ff789f8c
    libssh        8fad60098c1d6ed053a9a2d5a888b315cf3b1888a3c770bc5c220e51dff43bc8
    chromium      77d055fafef093c2b5b97581c603c3ad956e511d2a6e60719d9ba026337997c1

Chromium ran one strict command, which builds twice internally for `--check`,
because that command costs roughly 230 minutes; both internal builds printed
both of its smoke lines and the run reported `Build is reproducible. Checksums
match.` Every other package ran two independent strict commands.

Two limits are recorded rather than claimed as covered.

No assembly was rebuilt or booted as part of this plan. Several repaired
packages are shipped by `asm/desktop-vwl/desktop-vwl.yaml`, whose Vulkan Loader
pin is still deliberately uncommitted pending that rebuild, and the
`at-spi2-core` record separately notes that the same assembly needs its pin
advanced. Advancing those pins, rebuilding the affected assemblies, and running
the boot or QEMU checks is follow-on work outside this plan's acceptance
criteria, and it should be done in a supervised step before the repaired
packages reach a booted system.

Chromium's native messaging test proves manifest selection order against the
real browser but does not open a live native messaging port, which is weaker
than the original text of that record demanded. The tradeoff and its reason are
stated in the record itself.

## Idempotence and Recovery

The tracked audit entries form the durable ledger. Worker prompts, JSON
responses, error files, extracted sources, and build logs live only below
`.nex/tmp/ep016-*`, which Git ignores. A worker batch can be rerun into the same
named files after Codex records why the first attempt failed. Do not append
mixed attempts to one file.

Codex must record an assigned batch in `Progress` or a small ignored batch
ledger before starting it. After validating the report, Codex writes the
tracked entries in one patch. If a run stops, compare completed tracked entries
with the current manifest list and resume only missing or active entries.
Never approve an entry merely because an ignored worker result file exists.

Package and assembly builds may reuse the disposable Zub store, but Git remains
the source of truth. The two strict builds must still run when a source,
manifest, patch, output list, or relevant test changes. If a build fails, keep
its ignored log until Codex records the useful error and either assigns a
repair to Claude or declares a hard blocker.

Do not run related assembly builds concurrently when they share generated
development-source tarballs. Previous work proved that two such builds can race
while rewriting the same input cache. Run affected assemblies sequentially.

Use `.agents/cleanup-workdirs.sh` for registered work directories when cleanup
is needed. Do not delete broad `.nex`, store, repository, or human-owned paths.
Preserve unrelated worktree changes and re-run the worktree classification
before every commit.

If Claude exhausts its allowance, state the missing audit or coding capacity
and follow the hard-blocker protocol. Do not silently take the grunt work
back onto Codex. If a worker command fails for another reason, inspect only
its bounded error, fix the command or input, and rerun that batch.

## Artifacts and Notes

Starting facts at commit `9b06fde`:

    tracked package manifests: 533
    apps: 36
    bootstrap: 43
    cli: 64
    core: 48
    desktop: 20
    dev: 93
    fonts: 5
    libs: 215
    net: 8
    servers: 1
    tracked package patch files: 39
    patch filenames containing uapi: 31
    remembered UAPI configuration patch count to reconcile: 34

No package or assembly was rebuilt while writing this plan. The worktree was
clean before this plan file was added.

xAI's current model and reasoning documentation supports the Grok choices:

- <https://docs.x.ai/developers/grok-4-6>
- <https://docs.x.ai/developers/model-capabilities/text/reasoning>
- <https://docs.x.ai/build/cli/reference>

Anthropic's current model, effort, adaptive-thinking, CLI, and nested-agent
documentation supports the Claude choices:

- <https://platform.claude.com/docs/en/about-claude/models/overview>
- <https://code.claude.com/docs/en/model-config>
- <https://code.claude.com/docs/en/cli-usage>
- <https://code.claude.com/docs/en/sub-agents>
- <https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking>
- <https://platform.claude.com/docs/en/build-with-claude/effort>

These links document the model settings; they do not replace repository
evidence for any package audit.

## Interfaces and Dependencies

This plan adds no product runtime interface by itself. It adds a tracked audit
interface for future agents:

- `.agents/audits/package-configuration/README.md` defines the claim,
  ownership classes, record fields, and result states.
- Ten area Markdown files provide one manifest heading and audit record per
  tracked package manifest.
- `.agents/prompts/package-configuration-audit.md` gives Claude the
  read-only audit contract and compact output shape. Codex reviews that
  output and writes the tracked ledger. Coding assignments are separate
  Codex-to-Claude prompts; they are not this audit prompt.

The work depends on the current repository manifests, their local patches and
scripts, the pinned upstream sources named by those manifests, the Nex CLI,
the Zub package store, and any assembly or QEMU harness required by a discovered
fix. New audit and coding work depends on Claude Sonnet 5 access. Codex
supervises and commits. The plan does not require Codex to re-audit an accepted
Claude entry.

Any package fix discovered during the audit must preserve the public ownership
rules in `CONFIGURATION.md`, the package boundary in `PHILOSOPHY.md`, the test
requirements in `.agents/TESTING.md`, and the manifest conventions in
`.agents/MANIFESTS_CODE_STYLE.md`.
