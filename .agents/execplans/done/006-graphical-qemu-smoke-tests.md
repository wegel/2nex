# Add Graphical QEMU Smoke Tests

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Nex needs a repeatable way to test graphical desktop programs such as
Chromium. A package smoke that runs `chromium --version` proves that the binary
and dynamic libraries load, but it does not prove that a built desktop can
open a display, run the Wayland session, create a browser window, render page
pixels, or keep the browser alive long enough to use it.

After this plan, a coding agent can run one command from the repository root
and get proof that a selected Nex desktop boots in QEMU with an emulated video
device, starts a graphical session, launches Chromium through the normal
desktop graphics path, renders a local test page, and captures evidence from
inside the guest. The proof may be machine-readable logs, agent-visible
screenshots, or both. The same harness should be easy to extend for other
graphical packages such as VLC, sqlitebrowser, virt-manager, Looking Glass,
and Qt or GTK apps.

Every default check in this harness must be runnable and judgeable by a coding
agent. The harness must not require a human to look at a QEMU window, press
keys, move a mouse, or decide whether pixels look right. A visual check is
allowed when the harness saves an image or frame artifact that the agent can
open, inspect, and cite in the ExecPlan.

## Progress

- [x] (2026-06-29 21:42Z) Created this ExecPlan after the human asked for EP
  006 to organize correct testing for Chromium-like graphical packages with
  emulated video in QEMU or an equivalent environment.
- [x] (2026-06-29) Run the Ralph worktree pre-task and record what dirty or untracked files
  were committed, ignored, or left alone.
- [x] (2026-06-29) Read the required root docs and relevant knowledge notes.
- [x] (2026-06-29) Inventory existing QEMU scripts and desktop overlay startup behavior.
- [x] (2026-06-29) Design the graphical guest assertion contract.
- [x] (2026-06-29) Implement the QEMU graphical smoke harness.
- [x] (2026-06-29) Implement the guest-side graphical assertions for Chromium.
- [x] (2026-06-29) Add cleanup support for the new temporary images, roots, logs, sockets,
  and screenshots.
- [x] (2026-06-30) Build the target desktop assembly and run the graphical Chromium smoke.
- [x] (2026-06-30) Record all commands, logs, artifacts, and durable lessons.

## Surprises & Discoveries

- Observation: The EP006 pre-task found no checked commit candidates.
  Evidence: `git status --short --untracked-files=all` showed only known
  local notes and settings: `.claude/settings.local.json`, `CURRENT_TODO.md`,
  `PROMPT.md`, `SONIQ_YOCTO_PARITY_PLAN.md`,
  `chromium-manifests-todo.md`, and
  `src/bootloader/.claude/settings.local.json`. The completed EP005 and
  knowledge updates live under ignored `.agents/` paths.

- Observation: Durable knowledge now includes a builder/materializer theme
  from EP005 that can matter if this plan touches the materializer or assembly
  checks.
  Evidence: `ls .agents/knowledge` listed
  `builder-materializer.md`, and the EP006 orientation read
  `kernel-and-boot.md`, `system-assemblies.md`, `cli-testing.md`, and
  `builder-materializer.md`.

- Observation: The repo already has `scripts/qemu-desktop.sh`, but it is an
  interactive helper rather than an automated test.
  Evidence: the script starts QEMU with `-device virtio-vga-gl` and
  `-display gtk,gl=on`, prints SSH instructions, copies local SSH keys into
  the guest image, and has no success marker or timeout assertion.

- Observation: The current automated boot helper proves systemd boot, not
  graphical readiness.
  Evidence: `scripts/qemu-test-installer.sh --direct-initramfs --assert-boot`
  boots headlessly and waits for `ASSERT-BOOT-PASS`, but its QEMU command uses
  `-display none` and does not start or inspect a graphical app.

- Observation: This sandbox's initial QEMU install lacked automated graphical
  backends and virtio GPU devices.
  Evidence: before installing QEMU plugin packages,
  `qemu-system-x86_64 -display help` listed only `none`, and
  `qemu-system-x86_64 -device help | rg 'virtio.*(vga|gpu)|bochs|qxl'`
  found only `bochs-display`.

- Observation: Installing Arch QEMU UI and display-device packages makes the
  needed graphical modes available in this sandbox.
  Evidence: `pacman -S --noconfirm --needed qemu-ui-egl-headless
  qemu-ui-opengl qemu-ui-gtk qemu-hw-display-virtio-vga
  qemu-hw-display-virtio-vga-gl qemu-hw-display-virtio-gpu
  qemu-hw-display-virtio-gpu-gl qemu-hw-display-virtio-gpu-pci
  qemu-hw-display-virtio-gpu-pci-gl qemu-hw-display-qxl` completed. After
  that, `qemu-system-x86_64 -display help` listed `none`, `gtk`, and
  `egl-headless`, and device help listed `virtio-vga`, `virtio-vga-gl`,
  `virtio-gpu-pci`, and `virtio-gpu-gl-pci`.

- Observation: Chromium has only version-level smoke evidence today.
  Evidence: completed EP `002g-large-apps-browsers-and-media.md` records
  `chromium --version` and `chromedriver --version` package and assembled-root
  smokes, but no graphical launch, compositor, page render, screenshot, or
  remote debugging proof.

- Observation: QEMU `egl-headless` with `virtio-vga-gl` now boots the Nex
  desktop, gives wlroots a virtio DRM device, and lets the test prove browser
  pixels without a human looking at a window.
  Evidence: `scripts/qemu-test-graphical.sh --target-ref
  systems/desktop-vwl/0.0.1 --app chromium --timeout 300` passed. The serial
  log showed `Initialized virtio_gpu`, `fbcon: virtio_gpudrmfb (fb0) is
  primary device`, `/dev/dri/card0 /dev/dri/renderD128`, `dimensions=1280
  800`, `remote-debugging-title=NEX_GRAPHICAL_SMOKE_READY`, and
  `center-pixel=srgb(240,0,255)`.

- Observation: wlroots needed virtio GPU KMS before the graphical assertion
  could start.
  Evidence: the first smoke with a loadable virtio GPU driver logged
  `[drm] KMS disabled` and `vwl` found `0 GPUs`. After
  `CONFIG_DRM_VIRTIO_GPU=y` and `CONFIG_DRM_VIRTIO_GPU_KMS=y`, the smoke
  found `/dev/dri/card0` and `wlr-randr` reported `Virtual-1` at `1280x800`.

- Observation: Chromium's graphical launch exposed runtime files that version
  checks missed.
  Evidence: the first graphical Chromium launch failed on missing
  `/usr/lib/chromium/chrome_crashpad_handler` and missing NSS plugin library
  `libsoftokn3.so`. The manifest now installs crashpad when Chromium builds
  it and manually declares the NSS plugin libraries loaded through NSS.

## Decision Log

- Decision: Build a graphical VM assertion harness instead of accepting
  command-version smokes for desktop applications.
  Rationale: Desktop programs can pass version checks while still failing on
  display setup, Wayland protocol support, sandbox setup, GPU or software
  rendering, fonts, portals, profile directories, or runtime data files.
  Date/Author: 2026-06-29 / human and Carlos

- Decision: Prefer an automated QEMU path with emulated video as the first
  implementation.
  Rationale: QEMU is already used for Nex boot checks, and a virtual GPU gives
  this repo a reproducible graphics device without requiring the human's real
  desktop session or physical GPU.
  Date/Author: 2026-06-29 / Carlos

- Decision: Treat an interactive QEMU window as useful for debugging but
  insufficient for acceptance.
  Rationale: `.agents/TESTING.md` requires finite timeouts, captured logs, and
  explicit success conditions. A person seeing a window is not a commit gate.
  Date/Author: 2026-06-29 / Carlos

- Decision: Allow visual evidence only when the coding agent can inspect it.
  Rationale: Some browser failures are easiest to prove from pixels. The test
  may therefore save screenshots, but the agent must be able to view or parse
  those artifacts and record what it saw.
  Date/Author: 2026-06-29 / human and Carlos

- Decision: Use `egl-headless` with `virtio-vga-gl` as the first automated
  QEMU graphics mode, and keep `gtk` as a debug mode.
  Rationale: After installing QEMU plugin packages, this sandbox exposes both
  display backends. `egl-headless` gives a noninteractive QEMU display path,
  while GTK matches the existing manual desktop helper for local debugging.
  Date/Author: 2026-06-29 / Carlos

- Decision: Add a new focused script instead of folding graphical app checks
  into the existing installer boot helper.
  Rationale: `scripts/qemu-test-installer.sh` proves early boot and systemd
  assertion targets. `scripts/qemu-test-graphical.sh` needs app-specific
  guest scripts, screenshots, remote debugging, graphics mode selection, and
  artifact collection.
  Date/Author: 2026-06-29 / Carlos

- Decision: Build virtio GPU KMS into the kernel instead of relying on a
  module load before the compositor starts.
  Rationale: The direct-initramfs graphical assertion starts as a systemd
  target and needs `/dev/dri/card0` ready before `vwl` starts. Built-in KMS
  matched that boot path and removed the race.
  Date/Author: 2026-06-29 / Carlos

## Outcomes & Retrospective

EP006 added `scripts/qemu-test-graphical.sh`. The script builds a disposable
direct-initramfs disk under `.nex/tmp/graphical-smoke`, checks the desktop
manifest, checks out the target system ref, prepends AMD and Intel early
microcode cpios when present, injects a temporary SSH key and assertion
services, chooses `egl-headless-gl` when QEMU supports it, and saves logs and
guest artifacts under `.nex/tmp/graphical-smoke/artifacts`.

The first matrix entry is Chromium. The guest assertion starts `vwl`, waits
for a Wayland socket, checks `wlr-randr`, launches Chromium without headless
mode on a local HTML file, polls Chromium's local DevTools endpoint for the
title `NEX_GRAPHICAL_SMOKE_READY`, captures a screenshot with `grim`, and
checks the center pixel for `srgb(240,0,255)`. The script prints
`ASSERT-GRAPHICS-PASS` only after those checks pass.

The plan also fixed the concrete bugs that the graphical smoke found:

- The kernel now builds virtio GPU KMS in, so QEMU exposes `/dev/dri/card0`
  before wlroots starts.
- Chromium no longer mutates `/usr` during its package build; the manifest
  uses workdir overlays for clang, `libffi_pic.a`, and `nss.pc`.
- Chromium installs `chrome_crashpad_handler` when present and declares the
  NSS plugin libraries that NSS loads dynamically.

Checks run:

- `bash -n scripts/qemu-test-graphical.sh`
- `./src/cli/target/debug/nex check pkg/core/kernel/linux.yaml`
- `./src/cli/target/debug/nex check pkg/apps/web/chromium.yaml`
- `./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml`
- `./src/cli/target/debug/nex build pkg/core/kernel/linux.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs`
- `./src/cli/target/debug/nex build pkg/apps/web/chromium.yaml --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs`
- `./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force --record-profile`
- `scripts/qemu-test-graphical.sh --target-ref systems/desktop-vwl/0.0.1 --app chromium --timeout 300`

## Context and Orientation

Read these files first:

- `AGENTS.md`
- `PHILOSOPHY.md`
- `RUST_CODE_STYLE.md`
- `MANIFESTS_CODE_STYLE.md`
- `.agents/TESTING.md`
- `.agents/PLANS.md`

List `.agents/knowledge/` before work starts. Read at least:

- `.agents/knowledge/kernel-and-boot.md`
- `.agents/knowledge/system-assemblies.md`
- `.agents/knowledge/package-manifests.md`
- `.agents/knowledge/reproducibility.md`
- `.agents/knowledge/cli-testing.md`

Relevant existing files:

- `scripts/qemu-test-installer.sh`
- `scripts/qemu-desktop.sh`
- `.agents/cleanup-workdirs.sh`
- `asm/desktop-vwl/desktop-vwl.yaml`
- `asm/desktop-vwl/desktop-vwl-overlay.yaml`
- `pkg/apps/web/chromium.yaml`
- `pkg/desktop/wayland/*`
- `pkg/servers/xwayland.yaml`
- `pkg/libs/graphics/mesa.yaml`

Current desktop facts:

- `desktop-vwl` includes Chromium as
  `x86_64/pkg/apps/web/chromium/143.0.7499.169/bundles/full`.
- `desktop-vwl` includes Wayland desktop tools such as `grim`, `wlr-randr`,
  `wl-clipboard`, `Xwayland`, and `vwl` session glue.
- The current headless QEMU assertion can boot the deployment and prove
  systemd reaches the assertion service, but it does not provide a display.
- `scripts/qemu-desktop.sh` can create a full disk image and boot with
  `virtio-vga-gl`, but it depends on local SSH keys and manual observation.

## Plan of Work

Start by deciding whether to extend `scripts/qemu-test-installer.sh`, replace
`scripts/qemu-desktop.sh`, or add a new focused script such as
`scripts/qemu-test-graphical.sh`. Prefer a focused script if it keeps the
existing boot and installer helper simpler.

The harness must create a disposable VM disk or direct-initramfs boot target
from a chosen system ref. It should default to `systems/desktop-vwl/0.0.1` and
allow `TARGET_REF=systems/desktop-vwl-nvidia-580/0.0.1` or another ref. It
must not require human SSH keys. If the guest needs SSH for assertions, create
a temporary key under `.nex/tmp/`, inject only that key into the disposable
var image, and remove it through the cleanup script.

Implement one or more QEMU graphics modes:

- Preferred automated mode: a virtual GPU with a headless-capable display
  backend, such as `virtio-vga-gl` plus `egl-headless`, when the host supports
  it.
- Fallback automated mode: a virtual GPU without host GL, such as
  `virtio-vga` or `virtio-gpu-pci`, with software rendering inside the guest.
- Debug mode: GTK display with `virtio-vga-gl`, matching the current
  `scripts/qemu-desktop.sh` behavior.

The script must detect which mode is available and print the selected mode. If
no automated graphical mode is available, it must fail with a clear message
that names the missing QEMU feature or host dependency.

Define a guest-side assertion script that runs after boot. It must produce a
single success marker, such as `ASSERT-GRAPHICS-PASS`, only after all required
checks pass. It must print `ASSERT-FAIL:<reason>` before exiting on a known
failure.

Each assertion must produce evidence that a coding agent can judge without
human interaction. Logs and probes should return pass or fail through exit
status and explicit markers. Visual checks should save screenshots or frame
captures under `.nex/tmp/graphical-smoke-*` in a common format such as PNG, so
the agent can inspect the image directly or run pixel checks.

The Chromium assertion should prove the real graphical path:

- The guest sees a DRM device or virtual framebuffer from QEMU.
- The selected compositor or desktop session starts and exposes a Wayland
  socket under a valid `XDG_RUNTIME_DIR`.
- A tool such as `wlr-randr` can see at least one output with a real mode.
- Chromium starts with a fresh writable profile directory.
- Chromium uses the desktop display path, not `--headless`.
- Chromium opens a local `file://` or `data:` test page that includes a
  distinctive title, visible text, and simple canvas or CSS color blocks.
- The harness can prove that the page loaded, either through Chromium remote
  debugging on localhost inside the guest, a browser-produced artifact, or
  another deterministic in-guest probe.
- The harness captures a screenshot with `grim` or an equivalent tool.
- The screenshot is nonblank and contains expected pixel colors or another
  deterministic marker from the test page. This may be checked by script or by
  an agent opening the saved image and recording the visible evidence.

Do not use external network access for the page smoke. The page should live in
the guest assertion script, the var image, or a checked-in fixture under a
test-data directory.

The final harness should support a small test matrix. The first matrix entry
must be Chromium. Design the script so later entries can add commands such as
`vlc`, `sqlitebrowser`, `virt-manager`, or `looking-glass-client` without
rewriting the boot path.

The matrix format must tell the coding agent how to judge each app. Each entry
should name the launch command, the expected process or IPC endpoint, the log
or visual artifact, and the pass condition.

## Concrete Steps

Run commands from the repository root.

1. Orientation:

   ```bash
   sed -n '1,240p' AGENTS.md
   sed -n '1,260p' PHILOSOPHY.md
   sed -n '1,260p' RUST_CODE_STYLE.md
   sed -n '1,220p' MANIFESTS_CODE_STYLE.md
   sed -n '1,220p' .agents/TESTING.md
   sed -n '1,240p' .agents/PLANS.md
   find .agents/knowledge -maxdepth 1 -type f -printf '%f\n' | sort
   ```

2. Worktree pre-task:

   ```bash
   git status --short --untracked-files=all
   ```

   Commit coherent checked work when it already makes sense. Do not commit
   ignored files, local settings, scratch files, half-finished work, or
   unrelated human notes.

3. Inspect existing boot and desktop scripts:

   ```bash
   sed -n '1,280p' scripts/qemu-desktop.sh
   sed -n '1,760p' scripts/qemu-test-installer.sh
   sed -n '1,340p' asm/desktop-vwl/desktop-vwl-overlay.yaml
   ```

4. Inspect available QEMU display support:

   ```bash
   command -v qemu-system-x86_64
   qemu-system-x86_64 -display help
   qemu-system-x86_64 -device help | rg 'virtio.*(vga|gpu)|bochs|qxl'
   ```

5. Build or verify the target system:

   ```bash
   ./src/cli/target/debug/nex check asm/desktop-vwl/desktop-vwl.yaml
   ./src/cli/target/debug/nex build asm/desktop-vwl/desktop-vwl.yaml --verbose --check --update-checksum --force
   ```

6. Implement the harness. If adding a new script, prefer:

   ```text
   scripts/qemu-test-graphical.sh
   ```

   The script should support at least:

   ```bash
   scripts/qemu-test-graphical.sh --target-ref systems/desktop-vwl/0.0.1 --app chromium --timeout 300
   scripts/qemu-test-graphical.sh --target-ref systems/desktop-vwl/0.0.1 --app chromium --graphics-mode gtk-debug --timeout 300
   ```

   Environment variable equivalents are acceptable when they match existing
   script style, for example `TARGET_REF=... APP=chromium`.

7. Add cleanup coverage:

   ```bash
   sed -n '1,220p' .agents/cleanup-workdirs.sh
   ```

   Add every new `.nex/tmp/` path, image, socket, screenshot, and log prefix
   that the harness creates.

8. Run shell checks:

   ```bash
   sh -n scripts/qemu-test-graphical.sh
   sh -n .agents/cleanup-workdirs.sh
   ```

   If the final script uses Bash features, use a Bash shebang and run
   `bash -n` instead.

9. Run the graphical Chromium smoke:

   ```bash
   scripts/qemu-test-graphical.sh --target-ref systems/desktop-vwl/0.0.1 --app chromium --timeout 300
   ```

   The command must fail if Chromium exits early, the compositor never starts,
   the page does not load, or the screenshot does not contain the expected
   marker.

10. Run the normal boot assertion after adding the graphical harness:

    ```bash
    scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --timeout 180 --headless
    ```

    This proves the new graphical work did not break the existing boot check.

## Validation and Acceptance

This plan is complete only when all of these are true:

- A committed script or test command can boot a selected Nex desktop in QEMU
  with an emulated graphics device.
- The graphical harness runs without interactive input in its default test
  mode.
- A coding agent can judge pass or fail from process exit status, success
  markers, logs, saved screenshots, or deterministic artifact checks.
- The harness enforces a finite timeout and prints captured guest logs on
  failure.
- The harness does not require human SSH keys or host-specific private files.
- The harness creates only disposable files under approved temp paths and the
  cleanup script removes them.
- The guest assertion proves the display device exists.
- The guest assertion proves a Wayland session or compositor is running.
- The guest assertion proves an output mode exists.
- The Chromium assertion launches Chromium without `--headless`.
- The Chromium assertion uses a local page, not the external network.
- The Chromium assertion proves the page loaded through a deterministic probe.
- The Chromium assertion captures a screenshot or equivalent framebuffer
  artifact.
- The screenshot or framebuffer artifact is checked for a nonblank expected
  marker, not just file existence.
- If the screenshot is judged visually, the agent opens the saved image,
  records what visible marker proves success, and records the artifact path.
- The script prints `ASSERT-GRAPHICS-PASS` only after all graphical assertions
  pass.
- `scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --timeout
  180 --headless` still passes after the change.
- The active ExecPlan records every command, result, artifact path, and any
  host capability that affected the chosen graphics mode.

Do not complete this plan by saying Chromium was tested with `--version`,
`--headless`, by manual observation of a QEMU window, or by saving a screenshot
that no agent inspects. Those checks are useful diagnostics, but they do not
satisfy this plan.

## Idempotence and Recovery

The graphical harness must be safe to rerun. It should replace or recreate its
own images, logs, screenshots, profiles, sockets, and temporary keys. It must
not delete unrelated `.nex/tmp/` directories.

If QEMU leaves a process running after timeout, the script must kill it and
wait for it before exiting. If the guest assertion fails, the script must print
the serial log, guest assertion log, QEMU command line, selected graphics
mode, and paths to screenshots or framebuffer artifacts.

If host GL support is unavailable, try the software-rendering virtual GPU
path before blocking. If no automated graphical backend works in the sandbox,
record the exact `qemu-system-x86_64 -display help` and `-device help` output,
state the missing capability, print `@@BLOCKED@@`, and stop.

If the desktop session fails but the VM boots, leave the failed guest logs and
screenshot artifacts under `.nex/tmp/graphical-smoke-*` and add that prefix to
`.agents/cleanup-workdirs.sh`.

## Artifacts and Notes

Initial source facts:

- `scripts/qemu-desktop.sh` currently builds a disk and boots with
  `virtio-vga-gl` plus GTK GL display, but it is interactive and depends on
  local SSH keys.
- `scripts/qemu-test-installer.sh --direct-initramfs --assert-boot` already
  creates disposable root and var images and waits for `ASSERT-BOOT-PASS`.
- `desktop-vwl` includes Chromium and Wayland desktop packages.
- EP `002g` proved only version-level Chromium smokes, not a graphical browser
  launch.

Future agents must append:

- The selected QEMU graphics mode and why it was chosen.
- The guest assertion script path or generated script transcript.
- Serial log path.
- Guest assertion log path.
- Screenshot or framebuffer artifact path.
- The exact Chromium command line used in the guest.
- The proof that the page loaded.
- The proof that the screenshot contains the expected marker.
