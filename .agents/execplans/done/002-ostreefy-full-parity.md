# Achieve OSTreefy Personal System Parity

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Ralph will make Nex produce a desktop system that behaves like the human's
current OSTreefy-based personal machine, except where Nex deliberately uses a
different base technology.

The final user-visible result is this:

- `asm/desktop-vwl/desktop-vwl.yaml` builds a usable daily desktop with the old
  personal runtime package set.
- `asm/desktop-dev.yaml` builds a development desktop with the old personal
  developer tools.
- The built systems expose the expected commands, libraries, services, desktop
  helpers, container tools, VM tools, fonts, and browser or media apps.
- A checked-out or booted system passes a parity matrix that names each old
  OSTreefy package or behavior and says whether Nex matches it, replaces it
  with a Nex-native equivalent, or defers it for a recorded hard reason.

Parity here means practical behavior, not blind Arch package-name matching.
Nex should not add `ostree`, `grub`, or `mkinitcpio` only because OSTreefy used
them. Nex uses zub, Nex deployments, and its own boot path.

## Progress

- [x] (2026-06-28 04:30Z) Created the main parity ExecPlan and named the first
  planned sub-EPs.
- [x] (2026-06-28 04:36Z) Created sub-EP
  `002b-freeze-ostreefy-parity-matrix`.
- [x] (2026-06-28 05:24Z) Executed sub-EP
  `002b-freeze-ostreefy-parity-matrix`; the matrix consistency check passed
  with `expected=163 rows=163 unique_rows=163`.
- [x] (2026-06-28 12:40Z) Executed sub-EP
  `002c-small-daily-cli-tools`; daily CLI tools now land in `desktop-vwl` or
  `desktop-dev`, and Nushell is deferred by human request.
- [x] (2026-06-28 12:49Z) Created sub-EP
  `002d-developer-debug-tools`.
- [x] (2026-06-28 17:37Z) Executed sub-EP
  `002d-developer-debug-tools`; developer and debug rows assigned to 002d are
  covered in the matrix, with Nushell deferred by human request from earlier
  work.
- [x] (2026-06-29 07:45Z) Executed sub-EP
  `002e-desktop-session-glue`; `desktop-vwl` now covers the assigned desktop
  session rows with checked package builds, reproducible assembly builds,
  checked-out system smokes, or explicit replacements and skips.
- [x] (2026-06-29 04:57Z) Executed sub-EP
  `002f-container-and-vm-support`; `desktop-vwl` now includes the assigned
  container, filesystem, network capture, and VM support rows with package
  smokes, reproducible assembly builds, checked-out system smokes, or explicit
  Podman replacements.
- [x] (2026-06-29 07:20Z) Executed sub-EP
  `002g-large-apps-browsers-and-media`; VLC is packaged and included in
  `desktop-vwl`, and Chromium is the supported browser replacement for the
  Firefox and Vimb rows. The local ungoogled Chromium archive remains deferred
  by human request.
- [x] (2026-06-29 08:15Z) Executed sub-EP
  `002h-hard-or-policy-heavy-packages`; `desktop-vwl` now includes Linux
  headers and `wlopm`, `ntp` is verified as a systemd-timesyncd replacement,
  and the remaining hard rows have concrete policy deferrals.
- [x] (2026-06-29 05:57Z) Executed sub-EP
  `002i-final-boot-and-daily-driver-validation`; the matrix has no `needs-*`
  rows, `desktop-dev` passes its final dev-tool smoke, and `desktop-vwl`
  passed the direct initramfs boot assertion.
- [x] Move this ExecPlan to `.agents/execplans/done/` only after the parity
  matrix says Nex covers the old system or records the remaining near-parity
  gaps with concrete blockers.

## Surprises & Discoveries

- Observation: The old `tmp/ostreefy` tree is not present in this checkout at
  plan creation time.
  Evidence: `find tmp/ostreefy -maxdepth 3 -type f` failed with
  `No such file or directory` on 2026-06-28.
- Observation: The existing report already captured a useful package list and
  first-pass work order.
  Evidence: `OSTREEFY_REPLICATION_REPORT.md` lists package groups already in
  assemblies, package refs added in the first parity pass, missing manifests,
  and packages that need policy decisions.
- Observation: The first parity pass already landed.
  Evidence: Commit `a4670a4 asm: add ostreefy desktop packages` added
  Xwayland, libseat, wlroots, Wayland protocol data, Mesa Vulkan pieces,
  vulkan-loader, libsecret, libnotify, QEMU, libvirt, dosfstools, and
  `desktop-dev` with Node.js.
- Observation: The live OSTreefy source tree exists outside this checkout.
  Evidence: Sub-EP 002b used
  `/home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/Containerfile` and
  `/home/wegel/work/wegelcorp/ostreefy/flavours/archlinux/examples/Containerfile.wegel`
  as the primary sources.
- Observation: The frozen parity matrix has 163 unique rows.
  Evidence: The 002b matrix consistency script compared
  `.agents/ostreefy-parity-matrix.md` with the live base Containerfile,
  personal Containerfile, AUR loop list, and local package archive entry, then
  printed `expected=163 rows=163 unique_rows=163`.

## Decision Log

- Decision: Use this file as the only top-level active ExecPlan for the full
  parity push.
  Rationale: Ralph selects only files that match
  `.agents/execplans/[0-9][0-9][0-9]-*.md`. Sub-EP files named with suffixes
  such as `002b` will be plan artifacts controlled by this main EP, not
  top-level active plans.
  Date/Author: 2026-06-28 / Carlos

- Decision: Store sub-EP files under `.agents/execplans/subplans/` with names
  such as `002b-freeze-ostreefy-parity-matrix.md`.
  Rationale: That keeps Ralph's top-level selector focused on this main EP
  while still giving each work slice its own living plan.
  Date/Author: 2026-06-28 / Carlos

- Decision: Treat `ostree`, `grub`, and `mkinitcpio` as replaced base
  technology unless the parity matrix finds a user-facing behavior that still
  needs a Nex equivalent.
  Rationale: Nex uses zub, Nex deployments, and its own boot path.
  Date/Author: 2026-06-28 / Carlos

- Decision: Build package and assembly artifacts as the proof for each sub-EP.
  Rationale: `nex check` alone can miss missing runtime files, broken symlink
  paths, and packages that build but do not land in the system image.
  Date/Author: 2026-06-28 / Carlos

- Decision: Treat parity as program and behavior parity rather than exact Arch
  package-name parity.
  Rationale: The human clarified that package names do not need to match, but
  the programs being used must be packaged using Nex conventions and judgment.
  Date/Author: 2026-06-28 / Ralph

- Decision: Skip the custom local ungoogled Chromium archive for now.
  Rationale: The human explicitly allowed this exception during 002b.
  Date/Author: 2026-06-28 / Ralph

## Outcomes & Retrospective

Sub-EP 002b created `.agents/ostreefy-parity-matrix.md`. The matrix lists 163
old inputs, assigns one status to each row, and names the future sub-EP that
must handle every remaining gap. The custom local ungoogled Chromium archive is
deferred by human request.

Sub-EP 002c added the small daily CLI package set and assembled it into the
desktop systems. `desktop-vwl` now contains the runtime tools `age`, `bmon`,
`btop`, `chezmoi`, `dool`, `fish`, `jq`, `mosh`, `nc`, `screen`, and `wget`;
`desktop-dev` now contains `ast-grep`, `cloc`, `taplo`, and `tokei`. Nushell
remains out of scope by human request after reproducibility failures.

Sub-EP 002d added or verified the developer and debug tool rows assigned to
that slice.

Sub-EP 002e completed the desktop-session rows. It added or verified portals,
keyring and Polkit helpers, X11 helpers, fonts, terminal replacements, Qt
Wayland coverage, image and multimedia tools, remote desktop clients, JACK
tools, Loupe, and several documented replacements. It did not boot the full
graphical session; 002i keeps the final boot validation work.

Sub-EP 002f completed the container, filesystem, network capture, and VM rows.
It added or verified rootless Podman helpers, Distrobox, FUSE tools, SSHFS,
tcpdump, XFS tools, QEMU, libvirt, and virt-manager. Docker and Buildx remain
recorded as Podman replacements. The work also fixed the assembly capsule
flattener so Python package-directory needs copy all sibling modules into the
runtime capsule.

Sub-EP 002g completed the large browser and media rows. It added VLC 3.0.23
and Chromium to `desktop-vwl`, marked VLC covered, and marked Firefox and Vimb
replaced by the supported Chromium browser path. The local ungoogled Chromium
archive remains deferred by human request.

Sub-EP 002h completed the hard-or-policy-heavy rows that Nex can prove now.
It added Linux headers and `wlopm` to `desktop-vwl`, verified
systemd-timesyncd as the `ntp` replacement, and recorded concrete policy
deferrals for CPU microcode, 32-bit/Steam runtime support, Nvidia drivers and
containers, Looking Glass, OpenConnect SSO, and v4l2loopback.

Sub-EP 002i completed final validation. The parity matrix has no vague
`needs-*` rows. `desktop-dev` now uses full/dev bundles for build tools that
need support files, Meson has a package-local Python launcher wrapper, and the
rebuilt dev assembly passed its smoke suite. `desktop-vwl` booted through
`scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --timeout
180` and printed `ASSERT-BOOT-PASS`. The remaining gaps are the intentional
request and policy deferrals named by the matrix.

## Context and Orientation

Read these files before doing work from this plan:

- `AGENTS.md`
- `PHILOSOPHY.md`
- `RUST_CODE_STYLE.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/PLANS.md`
- `.agents/TESTING.md`
- `OSTREEFY_REPLICATION_REPORT.md`
- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/kernel-and-boot.md`
- `.agents/knowledge/cli-testing.md`

Important current facts:

- `tmp/ostreefy` is absent in this checkout as of 2026-06-28.
- `OSTREEFY_REPLICATION_REPORT.md` records an earlier comparison against the
  old personal image.
- The first assembly-only parity pass is already committed.
- The report counted 156 unique Arch package names from the old base and
  personal images. At that time, local assemblies covered 71 names, 12 more
  had Nex manifests but did not yet land in assemblies, and 73 lacked local
  manifests or close equivalents.
- The first pass added the 12 easy package refs except Chromium and
  `linux-headers` where the target use still needs a decision.

Key existing assemblies:

- `asm/nex-systemd.yaml`: base systemd system.
- `asm/desktop-vwl/desktop-vwl.yaml`: main desktop runtime.
- `asm/desktop-dev.yaml`: development desktop that extends `desktop-vwl`.

Key commands:

```bash
./src/cli/target/debug/nex check <manifest>
./src/cli/target/debug/nex build <manifest> --verbose --check --update-checksum --force
./src/cli/target/debug/nex build <package-manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
zub --repo .nex/repo checkout --copy systems/<slug>/<version> /tmp/<checkout-dir>
```

## Sub-EP Rules

Ralph must create and maintain sub-EP files as the work proceeds. Put them in:

```text
.agents/execplans/subplans/
```

Name them with this pattern:

```text
002b-freeze-ostreefy-parity-matrix.md
002c-small-daily-cli-tools.md
002d-developer-debug-tools.md
002e-desktop-session-glue.md
002f-container-and-vm-support.md
002g-large-apps-browsers-and-media.md
002h-hard-or-policy-heavy-packages.md
002i-final-boot-and-daily-driver-validation.md
```

Ralph may add more sub-EPs, such as `002j-*`, when new evidence shows that a
slice is too large or has a separate risk. Ralph must update this main EP when
adding, completing, merging, or dropping a sub-EP.

Each sub-EP must follow `.agents/PLANS.md`. Each sub-EP must name:

- package manifests or assembly manifests it will change
- commands it will run
- root filesystem paths, commands, services, or boot behavior it will inspect
- the commit or commits it produced
- any package that remains blocked, and why

This main EP is not complete while any referenced sub-EP remains incomplete.

## Plan of Work

Ralph should execute these sub-EPs in order unless new evidence requires a
different order.

### 002b: Freeze OSTreefy Parity Matrix

Create the source-of-truth parity matrix. If `tmp/ostreefy` is still missing,
use `OSTREEFY_REPLICATION_REPORT.md` as the starting source and search for a
restored copy before claiming final parity. The matrix must list each old
package or behavior, the Nex status, the target manifest or assembly, the proof
command, and the final decision.

Expected artifact:

```text
.agents/ostreefy-parity-matrix.md
```

Acceptance:

- Every package from the report lands in exactly one status: covered, replaced,
  blocked, deferred by policy, or needs manifest.
- The matrix names the next sub-EP for every `needs manifest` or `covered by
  assembly only` row.
- Ralph commits any tracked docs or helper scripts that the matrix needs. Do
  not commit ignored `.agents/` files.

### 002c: Small Daily CLI Tools

Add manifests and assembly refs for daily-use command-line tools that should
build quickly and unblock real use.

Initial target list from the report:

```text
jq
age
chezmoi
wget
cloc
mosh
taplo-cli
ast-grep
tokei
btop
bmon
screen
openbsd-netcat
```

Acceptance:

- Each new package passes `nex check`.
- Each new package builds with the strict package command or a stricter command
  recorded in the sub-EP.
- Each package has at least one smoke check from a checked-out output or built
  system, such as `<command> --version`, file existence, or an equivalent
  non-network test.
- `desktop-vwl` or `desktop-dev` includes the package where a daily driver
  would expect it.

### 002d: Developer Debug Tools

Add developer tools that are larger or have heavier dependencies.

Initial target list:

```text
gdb
lldb
gopls
sqlitebrowser
meld
```

Acceptance:

- CLI tools run version or help checks from the built output or built system.
- GUI tools at least have their binary and desktop metadata present in the
  built system, with library resolution checked where practical.
- `desktop-dev` includes development-only tools unless the matrix records a
  reason to put a tool in `desktop-vwl`.

### 002e: Desktop Session Glue

Add desktop services and helpers that make the Wayland desktop behave like a
daily driver.

Initial target list:

```text
xdg-desktop-portal
xdg-desktop-portal-wlr
gnome-keyring
polkit-gnome
pavucontrol
loupe
kitty
alacritty
swaync
xorg-xauth
xorg-xhost
xorg-xeyes
freerdp
remmina
gvfs-smb
gnome-themes-extra
qt5-wayland
qt6-5compat
```

Acceptance:

- The built `desktop-vwl` system contains the expected portal, keyring,
  authentication, notification, terminal, remote desktop, and X11 helper
  files.
- The sub-EP names which services should start automatically and proves their
  unit files or desktop/session config are present.
- If a full boot test is practical, run it. If not, check the built rootfs and
  record the boot-test blocker.

### 002f: Container And VM Support

Complete rootless container and VM support beyond the first QEMU/libvirt pass.

Initial target list:

```text
fuse-overlayfs
slirp4netns
distrobox
virt-manager
dmidecode
efibootmgr
tcpdump
sshfs
fuse3
linux-headers, if the system needs local kernel-module or DKMS-style builds
```

Acceptance:

- The built system has rootless Podman helper commands where applicable.
- VM user tools and libvirt-related GUI tools are present in the built system.
- The sub-EP runs the smallest practical command check for each CLI tool.
- If `linux-headers` enters the system, the sub-EP records the use case and
  proves the headers path exists.

### 002g: Large Apps, Browsers, And Media

Add large applications after the lower-level desktop pieces work.

Initial target list:

```text
firefox
chromium
vlc
vimb
imagemagick
libvncserver
tk
```

Acceptance:

- Browser packages build or the sub-EP records a hard build blocker with the
  failing command and log location.
- The built system contains at least one working browser path unless both
  browser attempts hit hard blockers.
- Media and image tools run non-network version or codec/list checks where
  possible.

### 002h: Hard Or Policy-Heavy Packages

Handle packages that require a policy, kernel-module, proprietary-driver, AUR,
or multilib decision.

Initial target list:

```text
steam
lib32-vulkan-radeon
nvidia-580xx-dkms
nvidia-container-toolkit
looking-glass
v4l2loopback-dkms
v4l2loopback-utils
openconnect-sso
wlopm
amd-ucode
jack-example-tools
docker-buildx
```

Acceptance:

- Each item gets one of these outcomes: implemented, replaced, blocked with a
  concrete missing design, or deferred with a policy reason.
- Kernel-module items do not enter the image until Nex has a recorded module
  build/install design.
- Proprietary or multilib items do not enter the image until the matrix records
  the chosen policy.

### 002i: Final Boot And Daily Driver Validation

Build the final assemblies and prove the result.

Acceptance:

- `desktop-vwl` builds with:

```bash
./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force
```

- `desktop-dev` builds with:

```bash
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
```

- Ralph checks out both systems with `zub` and runs the parity matrix probes.
- Ralph runs the smallest boot test that proves the desktop system reaches the
  expected target. If booting the graphical session is not practical in the
  sandbox, Ralph records exactly which boot path passed and which human test
  remains.
- The parity matrix has no untriaged rows.

## Concrete Steps

1. List `.agents/knowledge/` and read relevant notes.

```bash
find .agents/knowledge -maxdepth 1 -type f -printf '%f\n' | sort
```

2. Create `.agents/execplans/subplans/002b-freeze-ostreefy-parity-matrix.md`
   using the skeleton in `.agents/PLANS.md`.

3. Keep this main plan and the active sub-EP current after every meaningful
   discovery, check, or commit.

4. For each package manifest, run checks like this from the repo root:

```bash
./src/cli/target/debug/nex check pkg/path/to/package.yaml
./src/cli/target/debug/nex build pkg/path/to/package.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

5. For each changed assembly, run checks like this from the repo root:

```bash
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml asm/desktop-dev.yaml
./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force
./src/cli/target/debug/nex build asm/desktop-dev.yaml --verbose --check --update-checksum --force
```

6. Inspect built systems with `zub`:

```bash
tmp=$(mktemp -d /tmp/nex-desktop-vwl-check.XXXXXX)
zub --repo .nex/repo checkout --copy systems/desktop-vwl/0.0.1 "$tmp"
```

7. Commit frequently after the matching checks pass. Use exact paths with
   `git add`. Never use `git add -A`. Do not stage ignored `.agents/` files.

## Validation and Acceptance

This main EP is complete only when all of these conditions hold:

- Every referenced sub-EP is complete.
- `.agents/ostreefy-parity-matrix.md` has no untriaged rows.
- Every row in the parity matrix has evidence from a package build, assembly
  build, checked-out rootfs probe, command smoke test, service/unit file check,
  boot check, or recorded hard blocker.
- `asm/desktop-vwl/desktop-vwl.yaml` and `asm/desktop-dev.yaml` pass `nex
  check`.
- `desktop-vwl` and `desktop-dev` build with `--check`.
- Ralph commits all tracked package, assembly, script, and CLI changes needed
  for parity.
- Ralph leaves ignored `.agents/` files uncommitted.

Near-parity is acceptable only if the remaining rows belong to one of these
categories and the matrix explains each one:

- proprietary driver policy
- multilib policy
- kernel-module design
- missing upstream source or license decision
- sandbox limitation that blocks a test but not the repo change

## Idempotence and Recovery

All package and assembly builds are reproducible. Ralph may restart a failed
build from scratch instead of manually managing `.nex/tmp` directories. The
builder owns temporary build directories.

If a strict package build updates a checksum, rerun the same build command
until it passes the reproducibility check. If the second pass fails, compare
the first and second inputs or rootfs contents before changing package logic.

If a system assembly build updates its checksum and then fails the
reproducibility check, check for self-referential source snapshots and for
different package ref lists between passes.

If a package is blocked, update the active sub-EP, update the parity matrix,
and decide whether another package can proceed independently. Stop only when
AGENTS.md says the blocker is hard.

## Artifacts and Notes

Sub-EP files will live under:

```text
.agents/execplans/subplans/
```

The parity matrix will live at:

```text
.agents/ostreefy-parity-matrix.md
```

Current durable knowledge files:

```text
.agents/knowledge/cli-testing.md
.agents/knowledge/kernel-and-boot.md
.agents/knowledge/package-manifests.md
.agents/knowledge/reproducibility.md
.agents/knowledge/system-assemblies.md
```

Do not commit these ignored planning and knowledge files. They guide Ralph
locally.
