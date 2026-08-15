# Complete the first UAPI configuration parser wave

This ExecPlan is a living document. Keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current while the
work proceeds.

Maintain this plan according to `.agents/PLANS.md`. Keep candidate facts in
`.agents/SCRATCH_KNOWLEDGE.md`, then promote verified reusable facts into
`.agents/knowledge/` before the human review gate.

## Purpose / Big Picture

After this plan, BlueZ, PulseAudio, OpenSSH, and Glibc will read packaged
vendor policy from the active read-only `/usr` tree while preserving temporary
machine policy in `/run` and persistent administrator policy in `/etc`.
Changing or rolling back a Nex deployment will therefore change the vendor
defaults without copying those defaults into the host-owned `/etc` tree.

A person can prove the result by placing distinct values at the vendor,
transient, and administrator paths and observing each program choose `/etc`
before `/run` before `/usr`. Empty higher-priority files must mask lower files.
Explicit command-line and environment overrides must keep their upstream
meaning. Built Nex root filesystems must contain the vendor files below `/usr`
and no package-owned copies of these files below `/etc`.

This plan finishes the parser-heavy package wave named by the first UAPI audit.
Bash and e2fsprogs are already complete. The four remaining packages are
BlueZ, PulseAudio, OpenSSH, and Glibc. This plan does not absorb the later
D-Bus, Slang, Libvirt, OpenSSL, certificate, Fontconfig, PAM-module, or account
database batches recorded in `tmp/UAPI_TODO.md`.

## Progress

- [x] (2026-08-14 21:32Z) Wrote this ExecPlan from the verified package
  manifests, `.agents/kb.md`, and `tmp/UAPI_TODO.md`.
- [x] (2026-08-14 21:32Z) Fixed the scope to the four unfinished packages from
  the first parser wave and kept the later UAPI inventory out of this plan.
- [x] (2026-08-14 21:43Z) Ran the Ralph worktree pre-task, found a clean
  worktree, listed the durable knowledge files, and read the UAPI, package,
  assembly, reproducibility, builder, workflow, and test notes.
- [x] (2026-08-14 21:51Z) Verified all four pinned source archives and mapped
  every main-file reader, explicit override, drop-in path, reload path, RPC
  open, and nscd trace point described under `Surprises & Discoveries`.
- [x] (2026-08-14 21:57Z) Patched every BlueZ 5.85 reader, passed the
  focused tier and override test in both strict builds, ran the installed
  `bluetoothd --version`, and verified the packaged vendor paths.
- [x] (2026-08-14 22:18Z) Patched PulseAudio 17.0, passed the main-file and
  drop-in matrix in both strict builds, asserted an installed `--dump-conf`
  value, and ran the final packaged daemon's version command.
- [x] (2026-08-14 22:44Z) Patched OpenSSH 9.9p1, passed the client and server
  main-file, drop-in, explicit-path, Include, moduli, and live SIGHUP matrix in
  both strict builds, then ran both installed version commands.
- [x] (2026-08-14 23:04Z) Patched Glibc 2.39, passed the NSS and RPC
  precedence and live-change matrix in both strict builds, and verified that
  its split output contains only the two vendor files below `/usr/lib`.
- [x] (2026-08-15 00:11Z) Refreshed every missing `desktop-dev` package ref,
  rebuilt all 11 affected assemblies twice with matching checksums, checked
  the Edgebox and desktop roots, checked the three standalone base roots, and
  booted `nex-systemd` to `ASSERT-BOOT-PASS` in QEMU.
- [x] (2026-08-15 00:20Z) Re-ran the exhaustive `/etc` output scan, recorded
  the exact 31 manifests, updated `.agents/kb.md`, both durable knowledge
  themes, and the local `tmp/UAPI_TODO.md`, and found no product or build-
  system policy in the four reusable package patches and manifests.
- [x] (2026-08-15 00:20Z) Recorded the final commits, checksums, behavior
  tests, system checks, inventory, and remaining caveats below, then stopped
  at the Ralph human review gate.

## Surprises & Discoveries

- Observation: The current BlueZ manifest packages 5.85, although an earlier
  local audit note said 5.79.
  Evidence: `pkg/net/bluetooth/bluez.yaml` names version `5.85` and the
  `bluez-5.85.tar.xz` source.
- Observation: BlueZ does not route all three files through one reader.
  Evidence: `src/main.c` can inspect `CONFIGURATION_DIRECTORY` for
  `main.conf`, while the input manager, HOG plugin, and network plugin open
  compiled paths for `input.conf` or `network.conf`.
- Observation: PulseAudio installs four main files in its `conf` output but
  omits that output from both public bundles.
  Evidence: `pkg/libs/audio/pulseaudio.yaml` lists `client.conf`,
  `daemon.conf`, `default.pa`, and `system.pa` below `/etc/pulse`, while its
  `dev` and `full` bundles contain only `bin`, `dev`, and `lib`.
- Observation: Glibc supplies nearly every package build root, but the planned
  source change should alter only its configuration output and NSS behavior.
  Evidence: 494 manifests name a Glibc 2.39 dev bundle, while assemblies
  normally select its library or command outputs rather than its `conf`
  output.
- Observation: BlueZ has four calls that read the three packaged files.
  Evidence: `src/main.c:load_config` reads `main.conf` and preserves `-f`;
  `profiles/input/manager.c:input_init` and
  `profiles/input/hog.c:hog_read_config` separately read `input.conf`; and
  `profiles/network/manager.c:network_init` reads `network.conf`. Only the
  main-file reader inspects `CONFIGURATION_DIRECTORY`, and it truncates the
  value at the first colon without trying later entries. BlueZ has no reload
  path for these files.
- Observation: PulseAudio routes the four main files through two shared file
  helpers but routes structured drop-ins through one selected pathname.
  Evidence: `src/pulse/client-conf.c:pa_client_conf_load` and
  `src/daemon/daemon-conf.c:pa_daemon_conf_load` call
  `pa_open_config_file`, then `pa_config_parse(..., true, ...)` scans only
  `<selected-main>.d`. `pa_daemon_conf_get_default_script_file` and
  `pa_daemon_conf_open_default_script_file` call `pa_find_config_file` or
  `pa_open_config_file` for `default.pa` and `system.pa`. The helpers preserve
  `PULSE_CONFIG_PATH`, home files, `PULSE_CLIENTCONFIG`, `PULSE_CONFIG`, and
  `PULSE_SCRIPT`. Optional `match.table` and `stream-restore.table` readers
  also use `pa_open_config_file`, so the main-file patch must not accidentally
  broaden their policy.
- Observation: OpenSSH keeps explicit main-file arguments through reload but
  does not ship standard drop-in includes upstream.
  Evidence: `ssh.c:process_config_files` reads `-F` alone or reads the user
  file before `_PATH_HOST_CONFIG_FILE`. `sshd.c` initializes
  `config_file_name` from `_PATH_SERVER_CONFIG_FILE`, replaces it for `-f`,
  and `execv`s the saved argument vector on SIGHUP. `readconf.c` and
  `servconf.c` implement arbitrary `Include` globs with first-obtained-value
  semantics. The pinned `ssh_config` and `sshd_config` templates contain no
  `.d` include. `servconf.c` defaults `ModuliFile` to `_PATH_DH_MODULI`, and
  `dh.c` opens that path unless the server option replaces it.
- Observation: OpenSSH's first-obtained-value parser reverses the usual
  drop-in feed order needed to make later filenames win.
  Evidence: the client and server only fill options that remain unset. The
  new shared selector first removes shadowed lower-tier basenames, then sorts
  the surviving paths in descending lexical order. Its focused test passes
  for vendor, transient, administrator, empty, and `/dev/null` cases, and a
  host-side build links both `ssh` and `sshd` against the selector.
- Observation: `sshd` reparses its saved configuration text for `Match`
  blocks and sends the same text through its re-exec path.
  Evidence: `servconf.c:parse_server_match_config` consumes the saved `cfg`
  buffer after startup. The patch therefore stores ordered absolute `Include`
  lines in that buffer for default-path startup instead of parsing drop-ins in
  a one-shot loop.
- Observation: A strict OpenSSH build root has no `/etc/passwd`, so even
  `ssh -G` exits before reading configuration because UID 0 has no name.
  Evidence: the first strict build reached the client behavior test and
  printed `No user exists for uid 0`. The package test now creates temporary
  root and sshd entries inside the disposable build root before it runs any
  client or daemon command; those files are outside `OUT_DIR`.
- Observation: The OpenSSH package builds `ssh-keygen` before its behavior
  test but has no installed `ssh-keygen` command in the build-root `PATH`.
  Evidence: the second strict attempt passed every client assertion, then
  stopped at `ssh-keygen: command not found`. The test now invokes the freshly
  built `./ssh-keygen` directly.
- Observation: `sshd -T` validates the privilege-separation directory before
  it prints effective configuration.
  Evidence: the third strict attempt passed the client matrix and generated a
  throwaway key, then reported `Missing privilege separation directory:
  /var/lib/sshd`. The test now creates that disposable directory before its
  first server assertion.
- Observation: A pre-install OpenSSH daemon validates the compiled
  `/usr/libexec/sshd-session` path before it starts listening.
  Evidence: the fourth strict attempt passed the complete client and
  `sshd -T` matrix, then the reload daemon logged that the session helper did
  not exist. The reload-only configs now set `SshdSessionPath` to the freshly
  built `${WORK_DIR}/sshd-session`; package defaults keep the installed path.
- Observation: OpenSSH's complete strict check now passes and packages no SSH
  defaults below `/etc`.
  Evidence: both builds matched checksum
  `b9b50e17f25c18b3bb3e19f4e4bc4f2ae9e9de278b53daa2bd493b85f3f91558`.
  The behavior matrix passed twice, including system-main `Include` files and
  a live default-path SIGHUP change from port 40222 to 40223. The split output
  contains `ssh_config` and `sshd_config` under `/usr/lib/ssh`, `moduli` under
  `/usr/share/ssh`, and no `etc` tree. Both packaged commands print OpenSSH
  9.9p1 with the build-root OpenSSL 3.3.1.
- Observation: Glibc keeps both NSS selection and file-backed RPC paths in
  shared internals.
  Evidence: `nss/nss_database.c` opens and change-detects only
  `_PATH_NSSWITCH_CONF`; `nss/nss_module.c` registers that same path five
  times for nscd caches. `nss/nss_files/files-XXX.c` defines every database
  path as `/etc/<database>`, so the RPC patch must select a path without
  changing passwd, group, hosts, or other host databases. The RPC enumeration
  stream remains open between `setrpcent` and `endrpcent`, while keyed RPC
  lookups open a fresh stream.
- Observation: Glibc's file metadata cache cannot represent UAPI selection by
  itself because it deliberately treats every empty or missing file as the
  same content.
  Evidence: `io/file_change_detection.c:__file_is_unchanged` returns true when
  both sizes are zero. The patch therefore stores the selected tier beside
  the file metadata. The long-lived test observes higher files appearing,
  changing, and disappearing without restarting.
- Observation: nscd can register several traced files for one database and
  watches parent directories when a target does not yet exist.
  Evidence: `struct traced_file` links several records through `next`, and
  `nscd/connections.c` installs file and directory watches. The Glibc patch
  registers `/etc`, `/run`, and `/usr/lib` nsswitch paths for each of the five
  nscd cache types even though this package builds with `--disable-nscd`.
- Observation: Glibc's install target creates a target-root `ld.so.cache` in
  addition to the FHS patch's `/etc/rpc`.
  Evidence: the first install-layout assertion found only `ld.so.cache` after
  moving `rpc`. The manifest deletes that generated runtime cache and removes
  the empty output `/etc` directory.
- Observation: Glibc now passes the complete strict check and packages no
  defaults below `/etc`.
  Evidence: both builds matched checksum
  `bf348eabcec257edace3e1e05458bf79ddad1a5164f25e706b7e50d93b25190d`.
  The same C process covered compiled defaults, all three nsswitch tiers, an
  empty mask, replacements, removals, and all three RPC tiers inside a private
  chroot. The split `conf` output contains only `/usr/lib/nsswitch.conf` and
  `/usr/lib/rpc`.
- Observation: The audit source bytes exactly match all four manifest pins.
  Evidence: SHA-256 was `ad028e49...` for BlueZ 5.85, `053794d6...` for
  PulseAudio 17.0, `b343fbcd...` for OpenSSH 9.9p1, and `f77bd47c...` for
  Glibc 2.39.
- Observation: A local patch source does not add the `patch` command to the
  build root.
  Evidence: BlueZ's first strict build stopped at `patch: command not found`.
  Adding `x86_64/pkg/core/toolchain/patch/2.7.6/bundles/dev` let both strict
  builds apply the patch with zero fuzz and complete with matching checksum
  `f4b4a8aeac62ad3283a2f61ee7d895964372f09f92c3d72f42f5df2a9e0d7016`.
- Observation: The BlueZ package now tests one selector across every packaged
  filename.
  Evidence: `unit/test-config-path.c` ran during both strict builds for
  `main.conf`, `input.conf`, and `network.conf`; it covered vendor, transient,
  administrator, empty administrator, ordered `CONFIGURATION_DIRECTORY`, and
  missing explicit-directory cases. The retained installed daemon printed
  `5.85`; the packaged tree contained all three files under
  `/usr/lib/bluetooth` and no file under `/etc`.
- Observation: PulseAudio's installed daemon uses `DT_RPATH` for
  `/usr/lib/pulseaudio`, so a host-side split-output smoke cannot replace an
  older library through `LD_LIBRARY_PATH` alone.
  Evidence: the first post-package command loaded the workstation's
  `libpulsecommon-17.0.so` and reported the new helper as undefined. The
  packaged library did export all three new symbols. Running the loader with
  `--inhibit-rpath '' --library-path <package-lib-dirs>` selected the packaged
  libraries and printed `pulseaudio 17.0`.
- Observation: PulseAudio now keeps complete startup scripts separate from
  the two structured drop-in families.
  Evidence: the production-linked `layered-config-test` exercised all four
  main files, `PULSE_CLIENTCONFIG`, `PULSE_CONFIG`, `PULSE_SCRIPT`, both
  upstream user directories, all three system tiers, lexicographic drop-in
  order, same-basename priority, an empty mask, and a `/dev/null` mask in both
  strict builds. The installed `pulseaudio --dump-conf` command returned the
  asserted `exit-idle-time = 4242` value. Both builds matched checksum
  `ecd67b81b6a6dfde48df082245e94b837d53624306b65501b8124d2d02506a7d`.
- Observation: An internal PulseAudio test must include `<stdbool.h>` before
  `conf-parser.h` when no other included header supplies `bool`.
  Evidence: the first package compile stopped at that type; adding the direct
  include made the focused target and both complete builds pass.
- Observation: PulseAudio's normal Meson test switch needs the Check framework,
  whose declared bundle was absent from the disposable store.
  Evidence: the first `-Dtests=true` build stopped while materializing
  `x86_64/pkg/dev/util/check/0.15.2/bundles/dev`. A strict Check build refreshed
  its stale checksum and profile in commit `492403f`, and a C smoke linked to
  the packaged `libcheck`. PulseAudio then found Check 0.15.2, ran the focused
  Meson test by name, and passed it twice.
- Observation: An assembly dependency supplies files to the assembly build
  root but does not put that output in the finished Nex-structured system.
  Evidence: the first rebuilt Edgebox checkout lacked `/usr/lib/nsswitch.conf`
  and `/usr/lib/rpc` even though `nex-systemd` and the installer named Glibc's
  library output as a dependency. Adding `outputs/conf` to `packages` in the
  five standalone base manifests made the files visible in every checked
  root.
- Observation: Building a broad assembly is also a strict check of every
  package ref it names, even when the package source did not change in the
  active plan.
  Evidence: the `desktop-dev` build found 19 refs whose recorded manifests no
  longer matched the disposable store. Each package was rebuilt and exercised
  before its metadata commit; the final desktop assembly then built twice
  with checksum `75e4957348ef7c2e7f410ee9a16b52dad18405a315967477018e9492eaae24f5`.

## Decision Log

- Decision: Treat BlueZ, PulseAudio, OpenSSH, and Glibc as the remaining first
  parser wave.
  Rationale: The initial audit named these four with Bash and e2fsprogs. The
  latter two already pass strict builds and tiered lookup tests. The later TODO
  categories need separate plans because they involve different file classes
  and specifications.
  Date/Author: 2026-08-14 / Codex.
- Decision: Use full-file selection for each main configuration file.
  Rationale: Each parser already accepts one main file, and combining complete
  files can change ordering, `Match` blocks, or executable startup commands.
  The first existing `/etc`, `/run`, or `/usr` file must win; an empty file is
  therefore a deliberate mask.
  Date/Author: 2026-08-14 / Codex.
- Decision: Preserve user-specific and explicit upstream overrides ahead of
  the three system paths.
  Rationale: UAPI system tiers do not replace a user's PulseAudio file,
  `PULSE_*` override, OpenSSH `-F` or `-f`, BlueZ service-provided
  `CONFIGURATION_DIRECTORY`, or another documented explicit path.
  Date/Author: 2026-08-14 / Codex.
- Decision: Make every source patch generic and keep product policy in
  assemblies and their tests.
  Rationale: Nex package manifests must work in unrelated assemblies. Package
  patches may name only normal Linux paths and upstream program concepts.
  Date/Author: 2026-08-14 / Codex.
- Decision: Include Glibc's packaged `rpc` database in this wave.
  Rationale: Moving only `nsswitch.conf` would leave Glibc's vendor database in
  `/etc` and leave the Glibc manifest incomplete in the exhaustive inventory.
  Date/Author: 2026-08-14 / Codex.
- Decision: Land one checked package commit at a time, then land assembly and
  audit updates after every package works.
  Rationale: BlueZ, PulseAudio, OpenSSH, and Glibc can each remain a useful
  `git bisect` point with its own strict build and behavior test.
  Date/Author: 2026-08-14 / Codex.
- Decision: Store the selected nsswitch tier as part of Glibc's reload cache
  and trace every candidate path in nscd.
  Rationale: Glibc considers empty and missing files equivalent for content
  caching, while UAPI priority also depends on which path exists. Watching and
  comparing every tier makes creation and removal visible to long-lived
  processes and nscd.
  Date/Author: 2026-08-14 / Codex.
- Decision: Add Glibc's `conf` output as a package in each standalone base
  assembly, then let extended assemblies inherit it.
  Rationale: a root filesystem must ship the vendor databases for Glibc's
  `/etc`, `/run`, `/usr` lookup to work. Assembly dependencies alone do not
  place files in a Nex-structured result.
  Date/Author: 2026-08-15 / Codex.

## Outcomes & Retrospective

BlueZ, PulseAudio, OpenSSH, and Glibc now use tested layered readers and put
their package defaults below `/usr`. Their strict two-build checks and focused
behavior tests pass. All 11 affected assemblies also build reproducibly. The
checked Edgebox root passes its complete smoke test, the desktop root contains
the BlueZ and Glibc vendor files without package-owned `/etc` copies, the
standalone base roots contain the Glibc vendor databases, and `nex-systemd`
boots to `ASSERT-BOOT-PASS`. The package inventory now has exactly 31
manifests with declared `/etc` outputs. The tracked runbook and both durable
knowledge themes contain the verified lessons, and the ignored local tracker
matches them.

## Context and Orientation

UAPI.6 is the Linux convention used by this project for layered system
configuration. Package-owned defaults live in `/usr`, temporary machine
overrides live in `/run`, and persistent administrator choices live in
`/etc`. For one main file, a program uses the first existing candidate in this
order:

1. `/etc`
2. `/run`
3. `/usr`

An existing empty file wins and masks lower files. A program that has a real
drop-in subsystem must collect filenames across the three trees, sort them as
its documented parser requires, and let a higher tree shadow the same basename
from a lower tree. An empty higher-tier drop-in or a link to `/dev/null` masks
the same lower-tier file. A program must not concatenate complete startup
scripts or ordered main files merely because it supports an explicit include
statement.

`PHILOSOPHY.md` defines why Nex keeps host-owned `/etc` outside read-only
deployments. `.agents/MANIFESTS_CODE_STYLE.md` requires reusable package
manifests and reviewable local patches. `.agents/TESTING.md` requires a test
that exercises the built behavior. `.agents/kb.md`, under `UAPI.6
configuration audit`, records the source findings already verified. The
ignored `tmp/UAPI_TODO.md` records the live inventory in this shared worktree
and must stay current here. A fresh clone does not contain that local file; the
ExecPlan and tracked knowledge files contain everything required to work
without it.

The four package manifests are:

- `pkg/net/bluetooth/bluez.yaml`, BlueZ 5.85. It installs
  `/etc/bluetooth/main.conf`, `input.conf`, and `network.conf`.
- `pkg/libs/audio/pulseaudio.yaml`, PulseAudio 17.0. It installs
  `/etc/pulse/client.conf`, `daemon.conf`, `default.pa`, and `system.pa`.
- `pkg/cli/net/openssh.yaml`, OpenSSH 9.9p1. It installs
  `/etc/ssh/ssh_config`, `sshd_config`, and `moduli`.
- `pkg/libs/system/glibc.yaml`, Glibc 2.39. It installs
  `/etc/nsswitch.conf` and `/etc/rpc`.

Nex carries an authored source patch beside the manifest that uses it. Each
patch header names a subject, source, upstream status, and rationale. The
manifest records the patch SHA-256 and applies it through `patch --batch
--fuzz=0`. Do not use a series of `sed` commands for these multi-file source
changes.

Use `/home/wegel/work/perso/zub/target/debug/zub` through `ZUB_BIN` for every
build and checkout. The older `/home/wegel/.local/bin/zub` does not support the
store behavior that current Nex tests require.

## Plan of Work

### Milestone 1: freeze the source-reader map

Unpack each source archive through its normal package build or inspect the
retained build root. For every installed configuration file, record all source
functions that open it, every command-line or environment override, every
reload or file-watch path, and every upstream test that exercises the reader.

Update this plan's `Surprises & Discoveries` before editing source. If a file
has more readers than the current audit lists, add all of them to the same
package patch. Do not ship a patch that fixes only the easiest reader.

### Milestone 2: give BlueZ one shared system-file selector

Add `pkg/net/bluetooth/bluez-uapi-config.patch` and list it as a local source
in `pkg/net/bluetooth/bluez.yaml`. The patch must give `main.conf`,
`input.conf`, and `network.conf` the same selector. A service-provided
`CONFIGURATION_DIRECTORY` remains an explicit override: examine its
colon-separated directories in their given order and select the first existing
named file. Without that override, select:

- `/etc/bluetooth/<name>`
- `/run/bluetooth/<name>`
- `/usr/lib/bluetooth/<name>`

Move all three packaged defaults to `/usr/lib/bluetooth`. Keep the D-Bus policy
under `/usr/share/dbus-1/system.d`; that file is already vendor data in the
correct tree.

Add a focused source or build-root test that calls the shared selector for all
three filenames. The test must cover vendor-only, transient override,
administrator override, empty administrator mask, colon-separated
`CONFIGURATION_DIRECTORY`, and a missing explicit directory. Run an installed
`bluetoothd --version` or equivalent command from the package root in addition
to the selector test.

### Milestone 3: cover every PulseAudio main-file and drop-in reader

Add `pkg/libs/audio/pulseaudio-uapi-config.patch` and list it in
`pkg/libs/audio/pulseaudio.yaml`. Preserve PulseAudio's user configuration and
the documented `PULSE_CLIENTCONFIG`, `PULSE_CONFIG`, and `PULSE_SCRIPT`
overrides. For system main files, select:

- `/etc/pulse/<name>`
- `/run/pulse/<name>`
- `/usr/lib/pulse/<name>`

Move `client.conf`, `daemon.conf`, `default.pa`, and `system.pa` to
`/usr/lib/pulse`. Add `conf` to every bundle that upstream callers expect to
contain the normal PulseAudio defaults.

`client.conf` and `daemon.conf` have structured drop-ins. Make their effective
result obey the UAPI filename rules across user, `/etc`, `/run`, and `/usr`
trees. The higher tree must shadow the same basename from a lower tree; an
empty file and a `/dev/null` link must mask the lower file. Feed the selected
files to PulseAudio in the order required for lexicographically later names and
higher tiers to produce the documented effective value.

Treat `default.pa` and `system.pa` as complete executable startup scripts.
Select one main script and preserve explicit `.include` behavior; do not
automatically concatenate scripts from several tiers. Audit match and restore
tables and ALSA mixer data, but leave existing `/usr/share/pulseaudio` vendor
data there unless source evidence shows that a listed file is actually a
host-editable main file.

Extend upstream tests or add a focused build-root harness for all four main
files and both structured drop-in families. Exercise user priority, each
system tier, same-name shadowing, empty and `/dev/null` masks, and all three
explicit environment overrides. Run `pulseaudio --dump-conf` from the built
package and assert a chosen value, not only a zero exit status.

### Milestone 4: separate OpenSSH vendor policy and administrator state

Add `pkg/cli/net/openssh-uapi-config.patch` and list it in
`pkg/cli/net/openssh.yaml`. Preserve `ssh -F` and `sshd -f` as exact explicit
paths. Without those flags, select one client or server main file from:

- `/etc/ssh/<name>`
- `/run/ssh/<name>`
- `/usr/lib/ssh/<name>`

Move the packaged `ssh_config` and `sshd_config` to `/usr/lib/ssh`. Keep host
keys and generated machine identity under `/etc/ssh`; the package manifest
must not create them. Configure or patch the immutable `moduli` database to
use `/usr/share/ssh/moduli`, and move the packaged file there.

OpenSSH templates and parsers support `Include` statements and `.d`
directories. Preserve arbitrary explicit includes. For the standard client
and server drop-in families, collect `/usr`, `/run`, and `/etc` files so the
effective result follows UAPI basename shadowing, empty and `/dev/null` masks,
and filename order despite OpenSSH's first-obtained-value parser. The patch
must feed files in the order that produces the required effective result; it
must not depend on reading duplicate lower-tier basenames and hoping that the
parser ignores them.

Extend OpenSSH's regress tests or add a private package-root harness. Exercise
client and server main-file priority, standard drop-in ordering and masks,
`ssh -F`, `sshd -f`, `sshd -T`, SIGHUP reload of the default server path, and
the relocated moduli path. Use generated throwaway host keys inside the test
root. Never use a host key from the workstation.

### Milestone 5: make Glibc follow live NSS and RPC path changes

Add `pkg/libs/system/glibc-uapi-config.patch` and list it in
`pkg/libs/system/glibc.yaml`. Move packaged defaults to
`/usr/lib/nsswitch.conf` and `/usr/lib/rpc`. For both databases, select the
first existing `/etc`, `/run`, or `/usr/lib` file. Keep Glibc's compiled NSS
defaults when no file exists. An empty higher-tier file must mask lower policy
and cause the same compiled-default behavior that an explicitly empty
`/etc/nsswitch.conf` has upstream.

Patch every NSS path consumer, including reload state, tracing, and nscd source
code. Nex still builds Glibc with `--disable-nscd`; do not add nscd to the
package output. The source must nevertheless remain internally consistent so
an upstream build that enables nscd does not keep watching only `/etc`.

Add focused Glibc tests for vendor-only, `/run`, `/etc`, empty masks, no-file
compiled defaults, and `rpc` lookup. A long-lived C test process must prove
that NSS notices a higher-priority file appearing, changing, and disappearing
without restarting the process. Use a private chroot with synthetic passwd,
group, hosts, and rpc data. Do not modify the workstation's `/etc`.

### Milestone 6: rebuild real systems and close the inventory rows

Run `nex format` and `nex check` after each strict package build updates its
manifest. Rebuild all assemblies whose package content or embedded manifest
snapshot changes. At minimum, cover:

- `asm/flat-minimal.yaml`
- `asm/flat-systemd.yaml`
- `asm/installer/installer.yaml`
- `asm/nex-minimal.yaml`
- `asm/nex-systemd.yaml`
- `asm/edgebox-rootfs.yaml`
- `asm/flat-podman.yaml`
- `asm/desktop-vwl/desktop-vwl.yaml`
- `asm/desktop-vwl/desktop-vwl-nvidia-current.yaml`
- `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml`
- `asm/desktop-dev.yaml`

If `desktop-dev` still lacks GN 0.2289 in the disposable store, build
`pkg/core/toolchain/gn.yaml` with its strict command, then resume the assembly.
A missing disposable store ref is a normal prerequisite, not a reason to skip
the final assembly.

Extend `scripts/test-edgebox-rootfs.sh` for the OpenSSH and Glibc vendor paths.
Add a focused checked-out desktop-root assertion for all three BlueZ files.
Run the direct `nex-systemd` QEMU assertion and require
`ASSERT-BOOT-PASS`. PulseAudio's package harness must prove its policy readers
even if no current assembly materializes its `conf` output.

Re-run the exhaustive package output scan. Starting from the 35-manifest count
recorded on 2026-08-14, finishing these four manifests should reduce the count
to 31 unless another concurrent checked change alters the inventory. Record
the exact names, not only the number. Mark only the completed rows in the local
`tmp/UAPI_TODO.md` when it exists, and add verified source and test facts to
`.agents/kb.md` and the appropriate `.agents/knowledge/` theme files.

## Concrete Steps

Run every command from the Nex repository root. Prefix shell commands with
`rtk` as `~/.codex/RTK.md` requires.

Start and resume safely:

    rtk git status --short --untracked-files=all
    rtk rg --files .agents/knowledge
    rtk semeja search "UAPI configuration lookup and package tests" .agents/knowledge .agents/kb.md
    rtk sed -n '1,240p' .agents/MANIFESTS_CODE_STYLE.md
    rtk sed -n '1,240p' .agents/TESTING.md

Find all current package paths and assembly consumers:

    rtk rg -n '^\s*- path: /etc/(bluetooth|pulse|ssh)|nsswitch\.conf|/etc/rpc' pkg
    rtk rg -n 'name: (bluez|pulseaudio|openssh)|glibc/2\.39' asm pkg --glob '*.yaml'

Hash each new local patch and place the exact value in its manifest:

    rtk sha256sum pkg/net/bluetooth/bluez-uapi-config.patch
    rtk sha256sum pkg/libs/audio/pulseaudio-uapi-config.patch
    rtk sha256sum pkg/cli/net/openssh-uapi-config.patch
    rtk sha256sum pkg/libs/system/glibc-uapi-config.patch

Format and statically check each changed manifest:

    rtk ./src/cli/target/debug/nex format <changed-manifest.yaml>
    rtk ./src/cli/target/debug/nex check <changed-manifest.yaml>

Run the strict package command separately for all four manifests:

    rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./nex build pkg/net/bluetooth/bluez.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./nex build pkg/libs/audio/pulseaudio.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./nex build pkg/cli/net/openssh.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
    rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./nex build pkg/libs/system/glibc.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

Run each affected assembly through the strict assembly command:

    rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub ./nex build <assembly.yaml> --single --check --update-checksum --verbose

Check out and exercise the Edgebox result:

    rtk mkdir -p .nex/tmp/uapi-edgebox-root
    rtk /home/wegel/work/perso/zub/target/debug/zub --repo .nex/repo checkout --copy --force systems/edgebox-rootfs/0.0.1 .nex/tmp/uapi-edgebox-root
    rtk scripts/test-edgebox-rootfs.sh .nex/tmp/uapi-edgebox-root

Boot the rebuilt Systemd system:

    rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub scripts/qemu-test-systemd.sh systems/nex-systemd/0.0.1 --timeout 120

Count and list remaining package-owned `/etc` outputs:

    rtk rg -l --glob '*.yaml' '^\s*- path: /etc(?:/|$)' pkg
    rtk rg -l --glob '*.yaml' '^\s*- path: /etc(?:/|$)' pkg | rtk wc -l

Run final text and generic-policy checks:

    rtk rg -n -i 'yocto|soniq|edgebox|buildroot|nex[_-]uapi' pkg/net/bluetooth/bluez.yaml pkg/net/bluetooth/bluez-uapi-config.patch pkg/libs/audio/pulseaudio.yaml pkg/libs/audio/pulseaudio-uapi-config.patch pkg/cli/net/openssh.yaml pkg/cli/net/openssh-uapi-config.patch pkg/libs/system/glibc.yaml pkg/libs/system/glibc-uapi-config.patch
    rtk proxy git diff --check -- . ':!*.patch'
    rtk git status --short --untracked-files=all

## Validation and Acceptance

The human can accept this plan only when all of these facts hold:

- Each package manifest passes `nex format --check` and `nex check`.
- Each package passes the strict two-build command and records the resulting
  checksum, build profile, outputs, and dependency map.
- Each package test exercises the built reader rather than only checking that
  the patch applied or the program started.
- BlueZ applies one tested selector to `main.conf`, `input.conf`, and
  `network.conf`, including `CONFIGURATION_DIRECTORY`.
- PulseAudio preserves user and `PULSE_*` overrides, selects all four main
  files correctly, and applies UAPI drop-in shadowing and masks.
- OpenSSH preserves `-F`, `-f`, arbitrary `Include`, host-key state, reload,
  drop-in semantics, and moduli use while package defaults live below `/usr`.
- Glibc observes NSS tier changes in one long-lived process, uses compiled
  defaults after an empty mask or no file, and reads the selected `rpc`
  database.
- The four reusable package manifests and patches contain no Nex, Edgebox,
  Soniq, Yocto, Buildroot, or product policy.
- Every affected assembly builds reproducibly. A checked-out Edgebox root and
  desktop root contain the intended vendor files and no package-owned copies
  at the replaced `/etc` paths.
- `scripts/test-edgebox-rootfs.sh` passes against the rebuilt Edgebox root.
- The direct QEMU test prints `ASSERT-BOOT-PASS`.
- The exhaustive output list falls from 35 manifests to the expected 31, or
  this plan records the exact concurrent manifest that explains a different
  count.
- `.agents/kb.md` and `.agents/knowledge/` contain the final verified facts and
  no stale version claims. The ignored `tmp/UAPI_TODO.md` matches them in this
  worktree when that local tracker exists.
- Every commit passes its matching integrated checks and leaves a usable
  `git bisect` point.

### Completion Check

Completed on 2026-08-15. The 25 implementation commits span
`9ef7038fa^..fc97eab12`. They contain four isolated parser changes, one Check
framework prerequisite, 19 isolated `desktop-dev` prerequisite refreshes, and
one final assembly change. Each commit passed its matching integrated check
and is a usable bisect point. The branch was pushed through `fc97eab12`.

All four package manifests passed `nex format --check` and `nex check` after
their strict builds. Each strict command built twice with matching output:

- BlueZ: `f4b4a8aeac62ad3283a2f61ee7d895964372f09f92c3d72f42f5df2a9e0d7016`
- PulseAudio: `ecd67b81b6a6dfde48df082245e94b837d53624306b65501b8124d2d02506a7d`
- OpenSSH: `b9b50e17f25c18b3bb3e19f4e4bc4f2ae9e9de278b53daa2bd493b85f3f91558`
- Glibc: `bf348eabcec257edace3e1e05458bf79ddad1a5164f25e706b7e50d93b25190d`

The BlueZ selector test covered all three filenames, all three system tiers,
an empty mask, and ordered `CONFIGURATION_DIRECTORY` values. PulseAudio's
production-linked test covered its four main files, user and `PULSE_*`
overrides, both structured drop-in families, basename shadowing, empty masks,
and `/dev/null` masks; installed `--dump-conf` returned the asserted value.
OpenSSH covered client and server priority, standard drop-ins, arbitrary
Includes, `-F`, `-f`, `sshd -T`, relocated moduli, and a live SIGHUP with
throwaway host keys. One long-lived Glibc process covered compiled NSS
defaults, all tiers, an empty mask, file appearance, changes, removal, and all
three RPC tiers in a private chroot.

The 19 package prerequisites also passed strict two-build checks and a real
installed command, library, upstream test, or parser action. Their commits
and checksums are:

- `5c1a80fa0` GN: `8426401058c56042e36872ab6e766ce0a598c4e5fe7fffc13338af4fda99eeaa`
- `2e2c82c7e` Tk: `d26dc75115e81d566de0d5e3bb78500b224a4cafd16c994d4fd1f97fc8dfc09b`
- `44c9ffee3` gopls: `166311a6aae3f44993b45e14987457c094ff0a17d0ab1e43b340c4e40d0c8f34`
- `2e90f6ec2` AWS CLI: `33333267ded16cb6ba0c3d740614b60fa7e6ab97b7e0df0c059678b2065338cf`
- `7b9f0af79` libxcrypt-compat: `d752801bf76bb5dcd0b9b1b06e9831359d8448b8709647edc1be78ea4588ca92`
- `d359007c9` efivar: `d0c040dd7624adbb6ab7aabcbd853e90fb5522e6838aff5f01772888f63fe110`
- `7d9f2ad9e` efibootmgr: `cde38545b11a4c6488ed78f5f15c98142abd280d3a4aed350af94f64d1fc6125`
- `8e6b7463b` fakeroot: `2f08d29d9c340351801ddaf75f5a75ae0ceb04ff7dde7f27ee1aaddbcc4500a4`
- `1dd37835f` GDB: `52a9ba4090dd1b8bb0e557f8a4034522b409de7a073850db642165ba0a012cb7`
- `40a8db8b2` LLDB: `6a94a142c8809637f21d836e64a51ada2ce30128b210c5f00eca128ebf17ebd5`
- `7939741de` Meld: `0b478f553947933d11d490617803ff3861a0ef14bb99185902bb54c039fd4bd8`
- `f2203774e` SQLite Browser: `27bcc18ee5833ecb9fe0ed490ad81e0d20b21323c962330a84cb8d06c983a702`
- `941dbbf7c` Archive::Cpio: `632cf46a88df4a0ba4c668d4392efe46b02fb46069633ab730d8b4ef81bc7f5c`
- `223190f45` Archive::Zip: `6960b79d008700be53838b225798c7a896923c3cf3727f457a750befecf5fae8`
- `ba67b08ac` strip-nondeterminism: `68251c9ab6ac557575fc0b17fcf0627c418f83a9e71b285fe5fc44b2923972a4`
- `867b55f3f` ast-grep: `46f94fd87a6124da897fe03d2388aee5d1a7aeec9b5b5edf68f38dcdf2e7c6af`
- `334f1be74` cloc: `3d9ed0d0a1d1e58120c2549010ca6d5a25e3fbeb23d6545cdd11d2c1e60de620`
- `c6f8ecebd` Taplo: `4347fe12e16620c438f1b3d42264bd55331895af20601d55690289f872e84b71`
- `fa00c12be` Tokei: `3edaa95fc7904d663cfe3bf5683eec62c387d843a21ce0d36d0923451c8f441e`

All 11 assembly manifests passed `nex format --check` and `nex check`. Each
strict assembly command built twice and matched:

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

`scripts/test-edgebox-rootfs.sh` passed every check against the rebuilt
Edgebox root, including OpenSSH vendor paths, both Glibc databases, and a real
`getent rpc portmapper` lookup. A rootless desktop chroot found all three
BlueZ files and both Glibc databases, and found none of the replaced
package-owned `/etc` files. Flat-minimal, nex-minimal, and installer checkouts
also contained both Glibc databases. The direct nex-systemd QEMU test booted
Systemd and printed `ASSERT-BOOT-PASS`.

The final exhaustive package scan returned these 31 manifests with declared
`/etc` outputs:

```text
pkg/apps/containers/netavark.yaml
pkg/apps/graphics/imagemagick.yaml
pkg/apps/misc/ca-certificates.yaml
pkg/apps/security/gnome-keyring.yaml
pkg/apps/terminal/foot.yaml
pkg/bootstrap/phase0/toolchain.yaml
pkg/bootstrap/phase1/glibc.yaml
pkg/cli/shells/bash-completion.yaml
pkg/core/ipc/dbus.yaml
pkg/core/userland/2nex-utilities.yaml
pkg/desktop/wayland/fuzzel.yaml
pkg/desktop/wayland/swaync.yaml
pkg/desktop/wayland/waybar.yaml
pkg/dev/libs/openssl3.yaml
pkg/dev/virt/libvirt.yaml
pkg/libs/audio/pipewire.yaml
pkg/libs/crypto/p11-kit.yaml
pkg/libs/graphics/at-spi2-core.yaml
pkg/libs/graphics/fontconfig.yaml
pkg/libs/graphics/gtk3.yaml
pkg/libs/graphics/nvidia-580.yaml
pkg/libs/graphics/nvidia-current.yaml
pkg/libs/net/libnl.yaml
pkg/libs/net/libtirpc.yaml
pkg/libs/security/linux-pam.yaml
pkg/libs/system/attr.yaml
pkg/libs/system/fuse3.yaml
pkg/libs/text/vte.yaml
pkg/libs/tui/slang.yaml
pkg/net/firewall/iptables.yaml
pkg/net/firewall/nftables.yaml
```

The generic-policy search found no `yocto`, `soniq`, `edgebox`, `buildroot`,
or `nex_uapi` text in the four manifests or patches. `git diff --check -- .
':!*.patch'` passed before the assembly commit and again before this completion
record.

No acceptance check was skipped. Four prerequisite checks had bounded host or
test-harness limits: GN's full unit runner could not find its fixture path in
the private chroot although the files were present and readable; fakeroot's
suite passed 12 tests and failed four UID-preservation cases because it ran as
root in a user namespace; GDB printed `/proc` warnings before it successfully
traced `/usr/bin/true`; and LLDB needed its packaged `lldb-server` path plus a
no-PTY launch before `/usr/bin/true` exited zero. The Nex-minimal build also
prints pre-existing `touch` warnings while following public absolute symlinks;
the second build still matched, and a checked-out chroot resolved both new
Glibc links. QEMU logged an EFI automount failure on its BIOS test disk, but
the requested system unit ran and printed the explicit pass marker.

## Idempotence and Recovery

The strict package and assembly commands are safe to rerun. They rebuild twice
and update manifest metadata only after reproducible output. Run `nex format`
and `nex check` again after a strict build changes YAML.

Every local patch applies with `--fuzz=0`, so stale source context fails rather
than guessing. If a build stops after `--generate-outputs` edits a manifest,
inspect the diff, fix the package, and rerun the same strict command. Do not
hand-edit large generated output lists unless the builder cannot represent a
required curated output.

Keep private chroots and checked-out systems below `.nex/tmp/`. Add reusable
disposable paths to `.agents/cleanup-workdirs.sh` instead of deleting broad
paths by hand. Never modify workstation files below `/etc` while testing
priority or reload behavior.

If an assembly lacks a package ref in the disposable zub store, build the
declared manifest that owns that ref, then resume the assembly. Do not change a
correct assembly ref merely to match the current cache.

If one package proves that the chosen full UAPI behavior cannot preserve an
upstream invariant, stop before committing that package, record the exact
source conflict and test in `Surprises & Discoveries`, and use the Ralph blocker
protocol only when the plan permits no safe compatible design.

## Artifacts and Notes

Expected authored patch files:

- `pkg/net/bluetooth/bluez-uapi-config.patch`
- `pkg/libs/audio/pulseaudio-uapi-config.patch`
- `pkg/cli/net/openssh-uapi-config.patch`
- `pkg/libs/system/glibc-uapi-config.patch`

Expected durable notes:

- `.agents/kb.md`
- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/system-assemblies.md`
- `tmp/UAPI_TODO.md`, which remains ignored local working state

The package-output baseline on 2026-08-14 is 35 manifests with at least one
declared `/etc` path. Bash and e2fsprogs already pass their tier tests with
checksums recorded in `.agents/kb.md`.

## Interfaces and Dependencies

The patches add no public Nex manifest fields, command-line flags, libraries,
or services. Each package keeps its upstream public interface. The source
patches add only internal path-selection and drop-in helpers plus focused
tests.

The final system paths are:

- BlueZ vendor files: `/usr/lib/bluetooth/{main,input,network}.conf`
- BlueZ transient files: `/run/bluetooth/{main,input,network}.conf`
- BlueZ administrator files: `/etc/bluetooth/{main,input,network}.conf`
- PulseAudio vendor files and drop-ins: `/usr/lib/pulse/`
- PulseAudio transient files and drop-ins: `/run/pulse/`
- PulseAudio administrator files and drop-ins: `/etc/pulse/`
- OpenSSH vendor files and drop-ins: `/usr/lib/ssh/`
- OpenSSH transient files and drop-ins: `/run/ssh/`
- OpenSSH administrator files, drop-ins, and host keys: `/etc/ssh/`
- OpenSSH immutable moduli database: `/usr/share/ssh/moduli`
- Glibc vendor databases: `/usr/lib/nsswitch.conf` and `/usr/lib/rpc`
- Glibc transient databases: `/run/nsswitch.conf` and `/run/rpc`
- Glibc administrator databases: `/etc/nsswitch.conf` and `/etc/rpc`

The plan uses the existing package toolchains and the authorized zub binary at
`/home/wegel/work/perso/zub/target/debug/zub`. Add no new runtime package unless
the patched upstream source genuinely requires it.
