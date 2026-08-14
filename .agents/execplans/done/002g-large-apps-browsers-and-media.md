# Large Apps, Browsers, And Media

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Package or explicitly replace the remaining large browser and media
applications from the OSTreefy parity matrix. After this subplan,
`desktop-vwl` should expose a practical browser path and VLC or should record a
specific blocker with the command that failed.

## Progress

- [x] (2026-06-29 05:00Z) Started this sub-EP after completing
  `002f-container-and-vm-support`.
- [x] (2026-06-29 05:00Z) Ran the new-subplan worktree pre-task. Only local
  notes and settings were visible as untracked files, and no tracked commit
  candidate was present.
- [x] (2026-06-29 05:00Z) Listed knowledge files:
  `agent-workflow.md`, `cli-testing.md`, `kernel-and-boot.md`,
  `ostreefy-parity.md`, `package-manifests.md`, `reproducibility.md`, and
  `system-assemblies.md`.
- [x] (2026-06-29 05:00Z) Read relevant package-manifest, system-assembly,
  parity, and reproducibility knowledge. Key reminders: build-time
  dependencies are explicit, runtime ELF library dependencies are computed,
  script and plugin runtime files need manual `needs`, Python packages may need
  capsule-local wrapper paths, and disposable smoke roots go through
  `.agents/cleanup-workdirs.sh`.
- [x] (2026-06-29 05:00Z) Inspected the 002g matrix rows. `imagemagick`,
  `libvncserver`, and `tk` are already covered by earlier subplans.
  `firefox`, `vimb`, and `vlc` still need manifests or explicit replacements.
  The local ungoogled Chromium archive is deferred by human request.
- [x] (2026-06-29 06:20Z) Added
  `pkg/apps/multimedia/vlc.yaml` for VLC 3.0.23. `nex check` passed, and the
  strict package build passed with checksum
  `f0721b8de57797bcf6d7a04cbba5797d9fc034a24bc7f9744d4ccbd059c4a3c2`.
  Package smoke materialized a 12-ref runtime closure, verified that the
  package omits `plugins.dat`, ran `vlc-cache-gen` against 238 plugin shared
  objects, and ran `vlc`, `cvlc`, `nvlc`, and `rvlc` far enough to reach VLC's
  root-user guard.
- [x] (2026-06-29 06:35Z) Added VLC to `desktop-vwl`. `nex check` passed,
  the reproducible assembly build passed with checksum
  `11b65cb6b5a9c15453116c9b0d93313648ce430d3cb6bf520d0201e15f24d3b8`,
  and an assembled-root smoke verified `/usr/bin/vlc`, the wrapper scripts,
  `/usr/bin/sh`, `/lib64/ld-linux-x86-64.so.2`, no packaged
  `plugins.dat`, 238 VLC plugin shared objects, successful `vlc-cache-gen`,
  and the expected root-user guard for `vlc`, `cvlc`, `nvlc`, and `rvlc`.
- [x] (2026-06-29 07:00Z) Chose the existing built Chromium package as the
  supported browser path for the Firefox and Vimb rows. `pkg/apps/web/chromium.yaml`
  passed `nex check`, `nex resolve chromium --verbose` returned a 55-ref
  runtime closure, and a package smoke root ran `chromium --version` and
  `chromedriver --version`.
- [x] (2026-06-29 07:15Z) Added Chromium to `desktop-vwl`. `nex check`
  passed, the reproducible assembly build passed with checksum
  `1671d2d68b2c79fe7e19d456980e275bcef6318720b39715e1f4b4911ffb273b`,
  and an assembled-root smoke ran `chromium --version` and
  `chromedriver --version`.

## Surprises & Discoveries

- Observation: `pkg/apps/web/chromium.yaml` exists, but the frozen parity row
  is for the local ungoogled Chromium archive and is deferred by human request.
  Evidence: `.agents/ostreefy-parity-matrix.md` has
  `ungoogled-chromium-local` as `deferred-request`, while `rg --files pkg`
  finds `pkg/apps/web/chromium.yaml`.
- Observation: VLC 3.0.21 does not build against the repo FFmpeg 8 package
  with the tested feature set. Evidence: the build failed on removed
  `AVCodecContext` fields and old FFmpeg symbols before this plan switched to
  VLC 3.0.23.
- Observation: VLC's generated `plugins.dat` is not reproducible. Evidence:
  comparing the first raw store checkout against the second split output, with
  output-category prefixes removed, found 728 matching paths and exactly one
  changed file: `/usr/lib/vlc/plugins/plugins.dat`.
- Observation: VLC 3 uses a Qt 5 interface, while this repo currently has a
  Qt 6 stack. Evidence: the package disables Qt, X, and Wayland VLC interfaces
  and still builds command, ncurses, ALSA, FFmpeg, and plugin support.
- Observation: VLC wrapper scripts use `#!/bin/sh`, and the dependency scanner
  does not infer script interpreters. Evidence: the first smoke root omitted
  Bash until the manifest added manual `needs: /usr/bin/sh` and
  `resolution: /usr/bin/sh: bash`.
- Observation: Chromium is already packaged and built in the store. Evidence:
  `pkg/apps/web/chromium.yaml` passes `nex check`, `zub refs` shows
  `x86_64/pkg/apps/web/chromium/143.0.7499.169/bundles/full`, and
  `nex resolve chromium --verbose` returns a 55-ref runtime closure.
- Observation: Vimb needs a WebKitGTK stack that this repo does not currently
  package. Evidence: `rg --files pkg` finds GTK, Chromium, and libsoup3
  manifests, but no WebKitGTK, WPE, JavaScriptCore, or Vimb manifest.
- Observation: `zub union-checkout` can fail on duplicate headers for a large
  browser closure. Evidence: Chromium's 55-ref closure stopped on
  `/usr/include/at-spi2-atk/2.0/atk-bridge.h`; sequential
  `zub checkout --copy --force` into the same disposable root produced a usable
  smoke root.

## Decision Log

- Decision: Keep the custom local ungoogled Chromium archive out of this
  subplan unless the human changes the request.
  Rationale: The main EP and parity knowledge both record the human's request
  to skip it for now.
  Date/Author: 2026-06-29 / Ralph
- Decision: Cover the VLC row with a non-Qt VLC package for now.
  Rationale: VLC 3.0.23 builds reproducibly with the existing FFmpeg, ALSA,
  ncurses, and plugin stack. Adding the Qt GUI would require a Qt 5 stack or a
  later VLC track, which is larger than the row requires to package the used
  program.
  Date/Author: 2026-06-29 / Ralph
- Decision: Mark Firefox and Vimb replaced by the supported Chromium browser
  path in `desktop-vwl`.
  Rationale: Chromium is already source-packaged, built, resolves to a runtime
  closure, and runs version checks from both package and assembled roots. Vimb
  would require a new WebKitGTK stack before the small browser itself could be
  packaged.
  Date/Author: 2026-06-29 / Ralph

## Outcomes & Retrospective

VLC is packaged and included in `desktop-vwl`. Firefox and Vimb are replaced by
the existing Chromium browser package in `desktop-vwl`. The custom local
ungoogled Chromium archive remains deferred by human request.

## Context and Orientation

Current 002g rows from `.agents/ostreefy-parity-matrix.md`:

```text
firefox | needs-manifest
vimb | needs-manifest
vlc | needs-manifest
```

Rows already covered outside this subplan:

```text
imagemagick | covered by 002e
libvncserver | covered by 002e
tk | covered by 002d
```

Existing related manifests:

```text
pkg/apps/web/chromium.yaml
pkg/apps/graphics/imagemagick.yaml
pkg/libs/net/libvncserver.yaml
pkg/dev/lang/tk.yaml
```

## Plan of Work

1. Inspect existing browser, WebKit, GTK, multimedia, and codec manifests to
   choose the smallest correct path for Firefox, Vimb, and VLC.
2. Start with VLC if its dependency graph is mostly present, because it gives
   a concrete media command smoke and can use existing multimedia packages.
3. For Vimb, determine whether Nex already has a WebKitGTK stack that matches
   the current GTK generation. If not, record the missing WebKitGTK package
   stack and decide whether to create a focused sub-EP.
4. For Firefox, prefer a source-buildable or already-supported package path. If
   the source build requires a large new toolchain or policy decision, record
   the blocker and pick the supported browser replacement that best fits Nex.
5. Add successful packages to `asm/desktop-vwl/desktop-vwl.yaml`.
6. Build changed packages with the strict package command, build
   `desktop-vwl` reproducibly, check out `systems/desktop-vwl/0.0.1`, and
   smoke the public commands without network access.
7. Update `.agents/ostreefy-parity-matrix.md` after each row reaches covered,
   replaced, deferred, or blocked.

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
zub --repo .nex/repo checkout --copy --force systems/desktop-vwl/0.0.1 .nex/tmp/desktop-vwl-002g-smoke
```

## Validation and Acceptance

- `firefox`, `vimb`, and `vlc` rows are covered, replaced, deferred, or blocked
  with concrete evidence.
- Each new package passes `nex check`.
- Each new package builds reproducibly with the strict package command, unless
  the row records a hard blocker.
- `desktop-vwl` includes successful runtime packages and builds reproducibly.
- A checked-out `desktop-vwl` root runs non-network smokes such as
  `<command> --version`, `--help`, or plugin listing checks.
