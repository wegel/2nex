# Hard Or Policy-Heavy Packages

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Handle the remaining parity rows that need a kernel-module, proprietary,
multilib, AUR, local-device, or policy decision. This subplan should implement
the rows that Nex can prove today, and it should record concrete deferrals for
rows that need a human policy choice before package work would be correct.

## Progress

- [x] (2026-06-29 07:25Z) Started this sub-EP after completing
  `002g-large-apps-browsers-and-media`.
- [x] (2026-06-29 07:25Z) Ran the new-subplan worktree pre-task. Only local
  notes and settings were visible as untracked files, and no tracked commit
  candidate was present.
- [x] (2026-06-29 07:25Z) Listed knowledge files:
  `agent-workflow.md`, `cli-testing.md`, `kernel-and-boot.md`,
  `ostreefy-parity.md`, `package-manifests.md`, `reproducibility.md`, and
  `system-assemblies.md`.
- [x] (2026-06-29 07:25Z) Read relevant kernel/boot and parity knowledge.
- [x] (2026-06-29 07:39Z) Added `linux-headers` to `desktop-vwl`, checked the
  assembly, rebuilt it reproducibly, and checked out the resulting root.
- [x] (2026-06-29 07:45Z) Smoked Linux header paths from inside the checked-out
  root: `/usr/include/linux/eventpoll.h`, `/usr/include/asm/unistd_64.h`, and
  `/usr/include/drm/drm.h`.
- [x] (2026-06-29 07:45Z) Smoked the `ntp` replacement from inside the same
  root: `systemd-timesyncd --help` works, and
  `/usr/lib/systemd/ntp-units.d/80-systemd-timesync.list` names
  `systemd-timesyncd.service`.
- [x] (2026-06-29 08:03Z) Reclassified `wlopm` from policy-heavy to packageable
  after inspecting upstream. Added a package, built it reproducibly, included
  it in `desktop-vwl`, rebuilt the assembly reproducibly, and smoked the full
  root.

## Surprises & Discoveries

- Checked-out assembly roots use absolute symlinks into `/nex/pkg`. Host-side
  `test -e` follows those links against the host root and can report false
  negatives. Run smoke checks with `unshare --root <root>` so `/nex/pkg`
  resolves inside the checked-out root.
- A small package smoke root built from output refs may not include `/lib64`.
  If direct execution fails because the ELF interpreter path is absent, run the
  binary through `/usr/lib/ld-linux-x86-64.so.2` or smoke direct execution in a
  full assembly root.

## Decision Log

- `desktop-vwl` should carry `linux-headers` directly because ostreefy had
  `linux-headers` in the base set and Nex already has a built Linux headers
  package.
- Keep replacing `ntp` with systemd-timesyncd for this assembly. Nex already
  ships the service through `nex-systemd`, and the checked-out root proves the
  daemon and unit list are present.
- Package `wlopm` now. It is a small GPL Wayland client and does not require a
  proprietary, AUR, kernel-module, or hardware policy decision.

## Outcomes & Retrospective

002h completed the concrete rows and recorded the policy rows.

- Commit `7d98ea1 asm: include linux headers` added Linux headers to
  `desktop-vwl`. Checks passed:
  `nex check asm/desktop-vwl/desktop-vwl.yaml`, reproducible `desktop-vwl`
  build with checksum
  `0af9edce4583e94595ce6bed246c97ba47eff0e6be43502647160420e792357e`, and
  checked-out root smokes for `/usr/include/linux/eventpoll.h`,
  `/usr/include/asm/unistd_64.h`, `/usr/include/drm/drm.h`,
  `systemd-timesyncd --help`, and the systemd NTP unit list.
- Commit `7b09b08 pkg: add wlopm to desktop image` added
  `pkg/desktop/wayland/wlopm.yaml` and included it in `desktop-vwl`. Checks
  passed: `nex format pkg/desktop/wayland/wlopm.yaml`,
  `nex check pkg/desktop/wayland/wlopm.yaml`, strict reproducible package
  build with checksum
  `41c45e559265a423b1d191930d9bc1ba32de5018c9f75a5b313ccdb54db0d142`,
  package smoke through `/usr/lib/ld-linux-x86-64.so.2`, assembly check,
  reproducible `desktop-vwl` build with checksum
  `1c0690d052986aa96620e8a1028ce058bb4605d81b452ae19e8969fdb6000015`, and
  checked-out root smokes for `wlopm --version`, the man page, and bash
  completion file.
- `.agents/ostreefy-parity-matrix.md` now marks `linux-headers` and `wlopm`
  covered, keeps `ntp` replaced by systemd-timesyncd, and gives every
  remaining 002h deferral a concrete missing policy and future proof.
- Durable notes were promoted into `package-manifests.md`,
  `system-assemblies.md`, and `ostreefy-parity.md`.

No hard blocker remains in this subplan. Final boot and daily-driver runtime
validation stays with `002i`.

## Context and Orientation

Current 002h rows from `.agents/ostreefy-parity-matrix.md`:

```text
amd-ucode | deferred-policy
lib32-vulkan-radeon | deferred-policy
linux-headers | needs-assembly
looking-glass | deferred-policy
ntp | replaced
nvidia-580xx-dkms | deferred-policy
nvidia-container-toolkit | deferred-policy
openconnect-sso | deferred-policy
steam | deferred-policy
v4l2loopback-dkms | deferred-policy
v4l2loopback-utils | deferred-policy
wlopm | deferred-policy
```

## Plan of Work

1. Implement or prove the rows that do not need a new policy decision:
   `linux-headers` in the relevant desktop assembly, and the chosen time-sync
   replacement for `ntp`.
2. Inspect whether `wlopm` is small enough to package without adding a new AUR
   policy. If not, record the exact reason in this subplan and the matrix.
3. Record policy-heavy rows with concrete next decisions: CPU microcode boot
   integration, 32-bit graphics and Steam runtime policy, Nvidia driver and
   container runtime policy, Looking Glass host/client policy, OpenConnect SSO
   auth-helper policy, and v4l2loopback kernel-module policy.
4. Build changed packages or assemblies and smoke the resulting system paths.
5. Update `.agents/ostreefy-parity-matrix.md` so no 002h row remains vague.

## Concrete Steps

Run relevant checks from the repository root:

```bash
./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml
./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force
```

Use `.agents/cleanup-workdirs.sh` for disposable roots. Add new smoke paths to
that script before cleaning.

## Validation and Acceptance

- `linux-headers` is either included in the correct assembly and proved by a
  checked-out root, or the plan records why no assembly needs it.
- `ntp` has a checked replacement service or a recorded blocker.
- Each policy-heavy row names the exact missing policy decision and the future
  proof command.
- Any changed assembly builds reproducibly and the checked-out system smoke
  proves the concrete outcome.
