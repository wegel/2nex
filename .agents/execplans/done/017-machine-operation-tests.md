# Make an installed machine work, and test that it does

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

After this plan, one command passes, and it proves that a person sitting at an
installed Nex machine can do the four things Nex exists to let them do:

    scripts/test-machine-operations.sh

It passes because this plan both writes the tests and fixes what they find.
Writing tests that fail against a broken product proves nothing on its own, so
the fixes are in scope here rather than deferred. This plan absorbs what was
ExecPlan 018, now deleted: shipping manifests as a Git bundle, and renaming the
store.

It boots a real installed disk image in QEMU and makes the guest perform each
operation itself, over SSH, then checks the machine's own state afterwards.
Today no test does this. `scripts/qemu-test-live-upgrade.sh` proves the boot and
rollback machinery, but it stages the new deployment from the host by checking a
ref straight into `/nex/deployments`, which is the operation under test. So the
thing a user actually types has never been exercised.

The four operations, in the words a user would use:

1. Build a package on my machine.
2. Install a package temporarily, then throw it away.
3. Install a package and keep it across a reboot.
4. Put a new system version on the machine so it boots next time, and undo that.

Test 1 also settles a question this repository cannot currently answer from
source alone. Package manifests name their build environment by Git blob SHA,
and 476 of them name `27b6e5dc`, a historical revision of `env/standard.yaml`
(see `.agents/knowledge/environment-pinning.md`). `/nex/manifests` is created on
first boot by `git init` plus one commit of the shipped snapshot
(`base/nex-systemd.yaml:222`), and a repository with one commit cannot hold a
past revision of a file. Either the machine gets its manifests some other way,
or test 1 fails. Both outcomes are useful, and neither is known today.

## Progress

- [x] Read `scripts/qemu-test-live-upgrade.sh` end to end and list the parts
      worth reusing: disk assembly, `ensure_assert_key`, `ssh_probe`,
      `wait_for_ssh`, serial logging.
- [x] (2026-08-19) Established the fixture: `base/nex-systemd.yaml` plus SSH
      injected into the var tree by `scripts/prepare-qemu-test-identity.sh`. No
      new assembly needed; see Surprises & Discoveries.
- [x] (2026-08-19) Built the fixture image and proved an overlay boots from it
      with SSH reachable (9s to SSH, three runs in a row).
- [x] (2026-08-19) Implemented the harness verbs: `boot`, `run`, `reboot`,
      `expect_deployment`, in `scripts/test-machine-operations.sh`. `reboot`
      and `expect_deployment` are exercised by no test yet (test 1 needs
      neither); they are written and match the reference script's proven
      `assert_current_deployment`/reboot-by-killing-qemu pattern, but unproven
      end to end until tests 3/4 exist.
- [x] (2026-08-19) Wrote test 1 (`scripts/machine-tests/build-package.sh`)
      and recorded what it reports: FAIL, deterministically, three runs in a
      row. See Surprises & Discoveries.
- [x] (2026-08-19) Wrote `tests/nex-test-fixture.yaml` (extends
      `nex:base/nex-systemd.yaml`, adds `git`, ships
      `tests/nex-test-fixture.bundle` and overrides `nex-init-manifests` to
      clone it). `nex check` passes; `nex build --single --check
      --update-checksum` is strict two-build reproducible at checksum
      `a22e42ab383d0c108c3fb6f2ada6899c6f3a21ff31aed3c05e61b112aee6b507`
      (ref `systems/nex-test-fixture/0.0.1`). Harness repointed at it. Test
      1 reran, still FAILs, now past environment resolution's first gate; see
      Surprises & Discoveries.
- [x] (2026-08-19) Wrote tests 2, 3, 4
      (`scripts/machine-tests/temporary-install.sh`,
      `persistent-install.sh`, `deploy-and-rollback.sh`). Test 4 passes
      end to end, twice in a row. Tests 2 and 3 FAIL deterministically at
      `nex stage`, for a reason this plan had not named: the fixture's kernel
      bundle carries no `overlay.ko`. See Surprises & Discoveries.
- [x] (2026-08-19) Wired all four tests into
      `scripts/test-machine-operations.sh`. `--test <name>` still runs one
      alone; with no flag it runs all four in order and prints one summary.
      Tests 3 and 4 cross a reboot, handled by a per-test phase list
      (`TEST_PHASES`) the host steps through, calling the `reboot` verb
      between phases -- the first real exercise of `reboot` and
      `expect_deployment`-equivalent logic in this plan.
- [x] (2026-08-19) Ran the acceptance checks: full suite twice in a row
      (identical: 3 FAIL, 1 PASS, same reasons both times); a deliberately
      broken guest command (typo'd deploy ref) made exactly
      `deploy-and-rollback` fail, carrying `ref not found:
      systems/nex-systemd/0.0.1-typo-deliberately-broken`, while the other
      three tests kept their original, unrelated failures. Reverted after.
- [x] (2026-08-19) Promoted durable findings into `.agents/knowledge/`: the
      OverlayFS/kernel-bundle finding into `kernel-and-boot.md` ("Kernel
      bundle module dependencies") with a cross-reference from
      `machine-self-hosting.md`.

- [x] (2026-08-19) Rebuilt the fixture after the human added
      `kernel-fs-overlay`
      (`x86_64/pkg/core/kernel/linux/6.18.24/outputs/fs-overlay`) to
      `tests/nex-test-fixture.yaml`. `nex check` passes; strict two-build
      reproducible at checksum
      `69c012db66f612dc7074ab29d8b076876382a33e8cbda20396d410710d5fb487`
      (still `systems/nex-test-fixture/0.0.1`). Rebuilt the backing image and
      ran the full suite twice: `nex stage` now succeeds in both
      `temporary-install` and `persistent-install` (confirms the OverlayFS
      finding was accurate and the fix landed), but both now fail one step
      later, at `nex install`, on a different, newly-confirmed gap. See
      Surprises & Discoveries. Reported per instruction; did not attempt to
      route around it.

- [x] (2026-08-19) The human moved `kernel-fs-overlay` from the test fixture
      into `base/nex-systemd.yaml` itself ("nobody should be running a plain
      nex-systemd machine that cannot install a package"), so the fixture no
      longer carries its own copy. Rebuilt the cascade: `base/nex-systemd.yaml`
      (checksum `a271d6d1246076e032c99b3a8d2c060baff9004e428c6ae1f1fb9ddb258d31de`,
      strict two-build reproducible) and `tests/nex-test-fixture.yaml`
      (checksum unchanged, `69c012db66f612dc7074ab29d8b076876382a33e8cbda20396d410710d5fb487`
      -- the merged package list is identical either way, by name-based
      dedup in `merge_packages`). `examples/desktop-vwl/desktop-vwl.yaml`
      failed to build at this point: it takes `bundles/all-modules` for the
      kernel, which already carries `fs-overlay`, and now also inherited the
      base's new `kernel-fs-overlay` with nothing excluding it, so two
      packages delivered the same `overlay.ko`. I misdiagnosed this as a
      `zub` bug; the human corrected it (see Surprises & Discoveries) by
      testing the manifest change itself and fixed it with an `exclude` in
      `desktop-vwl.yaml`. `examples/desktop-dev.yaml` (which extends
      desktop-vwl) was not attempted by me either way.
      Rebuilt the backing image from the fixture and reran the full suite
      twice: identical to the previous entry (`nex stage` succeeds,
      `nex install` fails on the unresolved-runtime-dependency gap).

- [x] (2026-08-19) The human fixed environment resolution in commit
      `28218662`: `load_environment` now resolves blobs against the
      repository owning the manifest, not the store. Rebuilt the `nex`
      package itself (`pkg/core/nex/nex.yaml`, which builds from
      `dev: src/cli`, so it does not pick up source changes until rebuilt;
      checksum `4bdb768c35538eaa8116665a074c2f0677913d919ce83321a1574e3de7bd8f06`,
      strict two-build reproducible), then `base/nex-systemd.yaml`
      (checksum `27e6eea03bcbf309e1199b26ed8629a168943afa2b58bb2010f7efcbf1493482`)
      and `tests/nex-test-fixture.yaml`
      (checksum `508027e3b4f740de92c69df1690676a32ce6fe0a6e26da085211e33e2e67e223`)
      to pick up the fresh `nex` binary. Also fixed `seed_system_repo()` in
      `scripts/test-machine-operations.sh` to seed `tig`'s full runtime
      closure (resolved fresh each run via `nex resolve tig -v`, matching the
      exact algorithm `nex install` itself uses, then `zub pull` for each ref
      named), not just `tig`'s own ref -- this was the harness gap the
      earlier "4 unresolved runtime dependency requirement(s)" finding
      actually was. Ran the full suite twice: `build-package` gets past
      environment resolution but still fails, now on a *different*,
      newly-surfaced gap (building gzip needs its own build-sandbox
      dependency closure, which the harness does not seed, since seeding was
      scoped to the install tests' package). `temporary-install` and
      `persistent-install` get past `nex install`'s dependency resolution
      (`Runtime closure: 6 commit(s)`, matching what was seeded) but now fail
      during the actual checkout. `deploy-and-rollback`, previously passing
      every run, now FAILS too. All three failures are the identical error,
      reproduced deterministically twice: `uid 0 not mapped in namespace`.
      New finding, not decided or routed around; see Surprises & Discoveries.
- [x] (2026-08-19) The human root-caused and fixed the uid-mapping bug:
      `write_zub_config()` now writes an explicit identity range
      (`inside_start=0, outside_start=0, count=65536`) for `uid_map` and
      `gid_map` instead of an empty list, since the guest runs as real root.
      `scripts/qemu-test-live-upgrade.sh:168` carries the same latent bug,
      deliberately left alone per instruction. Reran the full suite twice
      (harness rebuilds its backing image on every invocation, so no stale
      cache to clear): `deploy-and-rollback` passes again, twice, as before
      the regression. `temporary-install`/`persistent-install` get past the
      checkout that was failing and reach `mount_nex_overlays()`, where they
      hit a new, different, confirmed error: `/nex/env` does not exist on
      this machine and can't be created, because `/nex` is on the read-only
      deployment root and no assembly ever creates that directory. Neither
      reaches `nex commit`. `build-package` still fails as expected, on
      gzip's own build-time dependency closure (glibc's `/files` and others)
      not being present in the guest store -- a real limitation (a machine
      that cannot fetch cannot build an arbitrary package), left as reported,
      nothing seeded around it. See Surprises & Discoveries and
      `.agents/knowledge/machine-self-hosting.md`.
- [x] (2026-08-19) The human fixed `/nex/env`: added `/target/nex/env` to
      `base/nex-systemd.yaml`'s mkdir line. Rebuilt `base/nex-systemd.yaml`
      (checksum `1ae2b42c1a0e5f24f6bf1473f0eb7089340151479c733dcf731ca6c7a2230860`)
      and `tests/nex-test-fixture.yaml`
      (checksum `7a902425d6315f6d66930b5a26fe4b41219baa3729a6e6cae9c0657cf691c034`),
      both strict two-build reproducible. Ran the full suite twice, identical
      both times. `nex install` now succeeds completely for the first time
      in this plan: checkout, flatten, symlink into `/usr/bin`, the installed
      `tig`/`git` binaries run. `temporary-install` then fails at `nex
      discard` (`umount: /nex/pkg: target is busy`, then removing
      `/nex/staging` hits `Read-only file system`) -- new, not chased to a
      root cause. `persistent-install` reaches `nex commit` for the first
      time and confirms the hypothesis flagged four units ago: `Error:
      Custom { kind: NotFound, error: "Could not determine current
      deployment" }` -- `get_current_deployment_ref()` finds neither a
      `nex/deployments/*` ref nor `nex/base` in the store, and nothing
      anywhere in this repository creates either on a fresh machine.
      `deploy-and-rollback` keeps passing. `build-package` still fails on
      gzip's build-time dependency closure, unchanged, as expected. See
      Surprises & Discoveries and `.agents/knowledge/machine-self-hosting.md`.
- [x] (2026-08-19) The human published deployment refs two ways: `nex deploy`
      now publishes `nex/deployments/<checksum>.<serial>` after checkout
      (`deploy.rs`, uncommitted, "the code builds and 202 tests pass"), and
      `seed_system_repo()` now publishes `nex/deployments/<checksum>.0` for
      the fixture's initial deployment (a correct installer would do this;
      `scripts/nex-install` does not today -- separate, known, left alone).
      Rebuilt `pkg/core/nex/nex.yaml`
      (checksum `3d766d5c90c456045045c54b4f0fc664d753a5b1c2451faed558571ecbdbceda`),
      `base/nex-systemd.yaml`
      (checksum `79ddf69671945f774c6955aa898b257b5c5ccaafdac97c28866046a7f200c645`),
      `tests/nex-test-fixture.yaml`
      (checksum `26f48b39b377a76f200d59a19e8ec5fefcb557ae9977291e8b9138449bf2eb53`),
      all strict two-build reproducible. Ran the full suite twice, identical
      both times -- unchanged from the previous entry in every observable
      way: `persistent-install` still fails at `nex commit` with the exact
      same `Could not determine current deployment`, even though a ref is
      now published in the right place. Traced further: `commit.rs:159`
      calls `store.refs(Some("nex/deployments/"))`, and the underlying
      `glob::Pattern` requires an exact match for a pattern with no wildcard
      -- `zub`'s own test for this exact function
      (`/home/wegel/work/perso/zub/src/refs.rs:381`) uses
      `"x86_64/*"`, with a trailing `*`, to get prefix-style matching.
      Reported as a probable second, independent cause, not asserted as
      certain and not fixed -- see Surprises & Discoveries.
- [x] (2026-08-19) The human confirmed the glob theory directly (a two-line
      repro) and fixed `commit.rs:159`, with a pinning test in
      `store/store_tests.rs`. Committed `d1e79485`. Rebuilt the cascade again
      (`pkg/core/nex/nex.yaml`
      `08cb193376fb4242149c554c2bb4d9d802343bd2b3c2c849af7eb59c0c8a3351`,
      `base/nex-systemd.yaml`
      `81b9557705e99d4214bd417fd9793226e9a4cad6bda642229ed57ca4ed810799`,
      `tests/nex-test-fixture.yaml`
      `2335f8ba7481717d770eb369fbfe96fcada5740e04e4c1d468f789fadd247be5`, all
      strict two-build reproducible) and reran: `persistent-install` still
      failed identically. Found and fixed a third cause myself, this time in
      the harness, not product code: `seed_system_repo()` tried to
      `rev-parse "$FROM_REF"` against the guest's own store to get the hash
      to publish, but `$FROM_REF` was never pulled into that store (only
      checked out via `zub checkout --copy` into the root filesystem
      content by an earlier step), so the resolve always failed silently
      and nothing ever got published -- visible in the log as `note: could
      not resolve systems/nex-test-fixture/0.0.1, initial deployment ref
      not published` on every run. Fixed by pulling `$FROM_REF` into the
      guest's store first. With all three fixes together, `nex commit`'s
      core logic now runs for real (finds the current deployment, checks it
      out, applies staged changes, creates a new commit ref) -- but the
      command still exits non-zero, because `create_deployment()` calls
      `cleanup_staging()` afterward, the same function `nex discard` calls,
      which hits the exact same `/nex/pkg` busy/EROFS failure. Root-caused
      by direct `/proc` inspection (no product code touched, diagnostics
      added to a copy of the guest script and reverted): PID 1 (`systemd`)
      holds an open file descriptor into `/nex/pkg`
      (`/usr/lib/systemd/systemd-executor`, a real symlink into a package
      capsule there), and roughly a dozen other processes have `/nex/pkg`
      paths in their memory maps, because `/usr/lib` and similar FHS paths
      are symlinked directly into `/nex/pkg` capsules on this kind of
      system. `/nex/pkg` may therefore be unable to unmount while anything
      is running, by construction. Full detail, observations kept separate
      from the proposed mechanism, in
      `.agents/knowledge/machine-self-hosting.md`.
- [x] (2026-08-19) The human verified both halves of the `/nex/pkg`
      diagnosis independently and fixed it in `stage.rs` (`f1a7dcfb`):
      `cleanup_staging` now unmounts each overlay with a fallback to `umount
      -l` (lazy) when busy, and only errors (naming the mount) if that also
      fails. Rebuilt the cascade again
      (`pkg/core/nex/nex.yaml` `325803c9b11b1910ab9401aa82fd6345cf00ef6bcd0c98e7f659ab650c151a02`,
      `base/nex-systemd.yaml` `bf3fddb017733ea2b04310e2cfd54b8040c202f1d14cbf59a69196973641e0f5`,
      `tests/nex-test-fixture.yaml` `b62f51326661982ee51ba674858de1652f30427247b9cedd223d7c2e1e36e790`,
      all strict two-build reproducible). Ran the full suite twice, identical
      both times: the mount-level problem is gone (confirmed: after a failed
      `nex discard`, no overlay remains in `/proc/mounts`, `/nex/staging` is
      confirmed completely empty, and `/usr/bin/tig` cleanly disappears, no
      stale view) but `nex discard`/`nex commit` both still fail with the
      same `Read-only file system` text, one layer deeper: `/nex/staging`
      itself is a separate mounted filesystem (its own bind mount from
      `/var/nex`), and removing that directory *entry* needs write access
      to its *parent*, `/nex`, which is on the read-only deployment root --
      so `rmdir("/nex/staging")` fails no matter how empty it is.
      `persistent-install` reaches `nex commit`'s real work again (finds
      current deployment, checks out, applies changes, creates the new
      commit) and fails at the same shared `cleanup_staging` step
      afterward, so it still never reaches reboot. `deploy-and-rollback`
      keeps passing; `build-package` unchanged. See Surprises &
      Discoveries and `.agents/knowledge/machine-self-hosting.md`.
- [x] (2026-08-19) The human fixed the mount-point removal in `stage.rs`
      (`eacb11f5`): `cleanup_staging` now clears `/nex/staging`'s contents
      via a new `clear_staging_state` instead of trying to `rmdir` the mount
      point itself. Also added a `current-deployment=` diagnostic to
      `persistent-install.sh`'s after-reboot phase (mine, harness-only, same
      `/proc/cmdline` convention used elsewhere). Rebuilt the cascade
      (`pkg/core/nex/nex.yaml`
      `3e6805c8a7250f84ee21d2f7818bd84a73e492cea1d11ba3d8f5d56671ef8ed4`,
      `base/nex-systemd.yaml`
      `f3b550a81361d56006da2026fed50c2505ad8adb102144ef1f524b469c07b25d`,
      `tests/nex-test-fixture.yaml`
      `319b995d56114e057ff8991e317dfd1aea2edb51629c45fd4c91502c158c570d`, all
      strict two-build reproducible). Ran the full suite twice, identical
      both times, 2 of 4 passing: `temporary-install` now passes completely
      -- the first fully green install-and-remove journey in this plan.
      `persistent-install` gets past `nex commit` (`commit-exit=0`,
      "Deployment created successfully.") and reboots cleanly for the first
      time, but its after-reboot assertion fails; checked which deployment
      actually booted per instruction, and it is the same one the machine
      started with. `nex commit` writes a new commit to the store under
      `nex/deployments/<timestamp>` but never touches `/sysroot` or the
      on-disk `/nex/deployments` directory the bootloader reads, so nothing
      new is ever bootable after a plain reboot. Real finding, not a test
      bug; not routed around. `deploy-and-rollback` and `build-package`
      unchanged. Only 2 of 4 passed, so the third confirmation run wasn't
      triggered. See Surprises & Discoveries and
      `.agents/knowledge/machine-self-hosting.md`.
- [x] (2026-08-19) The human made a committed install bootable (`a084cf66`):
      `create_deployment` now calls `deploy::run` on the new ref directly
      (reusing the activation path, `allow_commit_hash: true`). Rebuilt the
      cascade again
      (`pkg/core/nex/nex.yaml` `df49013f299e5196e73c19f2a2bbdda738c575506f7cf517d19cb292aaf03216`,
      `base/nex-systemd.yaml` `5cfeea15437583829a55724dfd05f866f675eb294d0637789665adb25b860471`,
      `tests/nex-test-fixture.yaml` `7591db572902f2a934cdd1a0f68b49eb7c2047baeae9c16b2fe34dbb8b0d2e5b`,
      all strict two-build reproducible). Ran the full suite twice, identical
      both times, still 2 of 4 passing (`temporary-install`,
      `deploy-and-rollback`). `persistent-install` now genuinely creates and
      activates a new on-disk deployment (`Deployed: <checksum>.1`,
      confirmed by listing `/sysroot/nex/deployments` directly: both the
      original and the new one present) -- but `nex commit` still exits
      non-zero right after, same `Read-only file system` text, one layer
      deeper again: `deploy::run`'s own remount-to-read-only for `/sysroot`
      also flips `/nex/deployments` and `/nex/staging` read-only, confirmed
      by direct before/after `/proc/mounts` comparison, because all three
      (plus `/`) are separate mounts of the same block device and a
      `remount` on any one changes the shared filesystem instance for all of
      them. The very next line, `commit.rs:151`'s
      `fs::remove_dir_all(&staging_dir)` (already checked once before and
      believed fine on its own terms), then fails because `/nex/staging` has
      just been made read-only as a side effect. Reboot never happens
      because the phase fails first (by the harness's own design). No
      cross-run serial accumulation was observable (fresh overlay per run,
      `Next serial: 1` every time), but the activation succeeding before the
      failing cleanup means a "failed" `nex commit` still leaves a real new
      deployment behind, worth flagging without fixing. `deploy-and-rollback`
      and `build-package` unchanged. See Surprises & Discoveries and
      `.agents/knowledge/machine-self-hosting.md`.
- [x] (2026-08-19) The human fixed the ordering (`9c94ef56`):
      `create_deployment` now only creates the ref and returns it;
      `cleanup_staging()` runs next; `activate_deployment(&new_ref)` runs
      last. Rebuilt the cascade again
      (`pkg/core/nex/nex.yaml` `225750ba4ddd31f366f6d4dca64666f06aedc666f248e31020549ebbeeaaee46`,
      `base/nex-systemd.yaml` `ab83690c7267a8ecf9f81dcf0b31061d7b73040f9c48939f045f81832d78fe77`,
      `tests/nex-test-fixture.yaml` `2a89a4ab2e540c4e4433d80a3069a54e4a33f1de311dfa2836ab7658da2bd949`,
      all strict two-build reproducible). Ran the full suite twice, identical
      both times, still 2 of 4 (`temporary-install`, `deploy-and-rollback`).
      `persistent-install`: activation again genuinely succeeds (confirmed:
      both deployments present on disk after the failure) but `nex commit`
      now fails on a new, different, downstream error:
      `mount: /sysroot: mount point is busy.` (twice, from `RemountGuard`'s
      explicit call plus its `Drop` retry) then `Error: Custom { kind:
      Other, error: "failed to remount /sysroot ro" }`. Confirmed directly:
      `/sysroot` is left mounted `rw` afterward -- the remount-to-read-only
      never happened. Reported as a new, precisely-located failure with a
      separately-flagged, not-yet-verified hypothesis (the same
      shared-block-device tension as before, now interacting with
      `cleanup_staging`'s lazy unmount happening immediately before
      activation). A stray deployment was again left on disk after this
      failure, as asked to watch for. Only 2 of 4 passed, so no third
      stability run. `deploy-and-rollback` and `build-package` unchanged.
      See Surprises & Discoveries and
      `.agents/knowledge/machine-self-hosting.md`.
- [x] (2026-08-19) The human fixed the remount retry (`49df0574`):
      `RemountGuard::remount_ro` now retries five times with a 200ms gap and,
      if `/sysroot` is still busy after that, prints a warning and returns
      success instead of an error. Rebuilt the cascade again
      (`pkg/core/nex/nex.yaml` `8e7419c35736574027fa79b908c18a90d0781fc9f68cea3030721d0b45c7498c`,
      `base/nex-systemd.yaml` `fd1951a09958ecf0d8d231fac3445f93412b0f9bb441d2024e85e8ad2ee7e81b`,
      `tests/nex-test-fixture.yaml` `6fc6aa8a93a1aadcff76545096c643167337dcdd69023b18962e6fc906e54a3a`,
      all strict two-build reproducible). Ran the full suite twice, identical
      both times, still 2 of 4 (`temporary-install`, `deploy-and-rollback`).
      `persistent-install` reaches `PASS: persistent-install (before-reboot)`
      for the first time in this plan: `commit-exit=0`, "Deployment created
      successfully.", the warning text present as predicted (twice, both
      runs -- the explicit call and the `Drop` retry). Reboot succeeds and
      `current-deployment=` genuinely differs from the pre-commit value
      (`746563fd97dc42f12e41a3312c39e72eb5d71cb5f04e0215905df02846a3ab4f.1`
      vs. the starting `6fc6aa8a93a1aadcff76545096c643167337dcdd69023b18962e6fc906e54a3a.0`),
      confirming activation itself now works end to end. But `tig` is absent
      after reboot: `post-reboot-binary-present=false`. Root-caused with a
      disposable diagnostic copy of the guest script (reverted after) plus a
      source read of `commit.rs`, not just a guess: `create_deployment`'s
      `copy_dir_contents(&upper_nex, &nex_target)` call, where
      `upper_nex = "{STAGING_STATE_DIR}/upper/nex"` and
      `nex_target = "{staging_dir}/nex/pkg"`, uses `cp -a "$src/." "$dst"`,
      which copies `upper_nex`'s *contents* into `nex_target`. `upper_nex`'s
      actual children are `pkg/` and `env/` (the two separate overlay
      upperdirs, confirmed earlier via `/proc/mounts`), so the copy lands
      them as `nex_target/pkg/...` and `nex_target/env/...` -- one extra
      `pkg/` nesting level, and `env/` misplaced under `nex/pkg/env` instead
      of `nex/env`. Confirmed directly on the booted new deployment:
      `/nex/pkg` (the FHS-visible top level of the current deployment) has an
      unexpected `pkg/` subdirectory alongside the legitimate namespace dirs
      (`apps`, `cli`, `core`, `dev`, `env`, `libs`), and `dev/vcs/tig/...`
      lives one level too deep at `/nex/pkg/pkg/dev/vcs/tig/2.6.0/0cbb5716`.
      `/usr/bin/tig`'s symlink itself is correct
      (`../../nex/pkg/dev/vcs/tig/2.6.0/0cbb5716/usr/bin/tig`, since the
      `usr/bin` copy is a separate, correctly-structured call in the same
      function) but dangling: `stat -L /usr/bin/tig` reports "No such file or
      directory" because nothing exists at the path the symlink names.
      `command -v` (and thus `post-reboot-binary-present`) correctly reports
      the dangling symlink as absent. This is confirmed, not a hypothesis:
      both the `cp -a`-into-contents mechanism and the resulting on-disk tree
      were inspected directly. Not fixed (product code). `deploy-and-rollback`
      and `build-package` unchanged. Only 2 of 4 passed, so no third
      stability run. See Surprises & Discoveries and
      `.agents/knowledge/machine-self-hosting.md`.
- [x] (2026-08-19) The human fixed the copy-nesting bug in both places
      (`dc561b55`): `create_deployment` now copies `upper/nex`'s contents to
      `{staging_dir}/nex` instead of `{staging_dir}/nex/pkg`, and the same bug
      existed a second time in `merge_overlay_changes` (the non-store,
      discard-adjacent path, `commit.rs:87`), copying to `/nex` instead of
      `/nex/pkg` there too. `merge_overlay_changes` also switched its two raw
      `let _ = Command::new("umount")...` calls to `stage.rs`'s
      `unmount_overlay` helper (now `pub(super)`), and added a missing
      `/nex/env` unmount alongside `/usr/bin` and `/nex/pkg`, for the same
      reason `cleanup_staging` needed it before: a swallowed unmount failure
      would copy into a still-live overlay instead of the real location.
      Rebuilt the cascade
      (`pkg/core/nex/nex.yaml` `25512d668f797e420416c7af8a1edebce7278f99bae778626c731ede217c5b94`,
      `base/nex-systemd.yaml` `fc83635fe2f5c6121df4bed654658fc493fa445d9aaaf8dd6ecbd7353fd526b3`,
      `tests/nex-test-fixture.yaml` `7a8781f3fe195bb626230c38a82cb773c4bc343a4a95c4d28c8d7eb8d266b0ff`,
      all strict two-build reproducible). Ran the full suite three times
      (three, since the first two runs both landed 3 of 4): identical every
      time. `persistent-install` passes completely for the first time in this
      plan, including the after-reboot run (`post-reboot-binary-present=true`,
      `tig version 2.6.0` executes). Checked the tree shape directly, not just
      the presence check, per instruction, with a disposable diagnostic on a
      copy of the guest script (reverted after; no product code touched):
      `/nex/pkg/pkg` absent, `/nex/env` present as a sibling of `/nex/pkg`,
      `/nex/pkg/env` absent, and `/usr/bin/tig`'s symlink resolves
      (`stat -L` succeeds) rather than dangling. The `/sysroot is still
      writable` warning still appeared every run, twice per `nex commit` call
      as before -- expected, not a failure, per instruction.
      `temporary-install` and `deploy-and-rollback` kept passing.
      `build-package` still fails, same root cause across all three runs
      (glibc's `/usr/lib/gconv/*` split outputs missing from the guest
      store -- the build-time dependency closure gzip needs was never seeded,
      a real, expected, unrouted-around limitation). This closes out
      `persistent-install`; three of four tests now pass deterministically,
      leaving `build-package` as the one understood, expected failure. See
      Surprises & Discoveries and `.agents/knowledge/machine-self-hosting.md`.
- [x] (2026-08-19) The human corrected an earlier call: seeding gzip's
      build-time dependency closure is legitimate host-side setup, the same
      kind `SEED_CLOSURE_PACKAGE` already does for tig's runtime closure --
      not something that weakens the test. Added `seed_gzip_build_closure()`
      to `scripts/test-machine-operations.sh`, called from
      `seed_system_repo()`. It extracts gzip's 14 (actually 15 --
      miscounted at first, corrected by an automated `grep`, not a hand
      count) `dependencies:` commits straight from
      `pkg/cli/archive/gzip.yaml` at seed time, pulls each exactly as
      declared (`nex build` uses that literal string as a resolver root,
      matching `src/cli/src/build/dependencies.rs:resolve_dependency_commits`),
      then discovers each dependency's own `/files` closure -- the same
      `resolve_runtime_deps_precomputed` resolver `nex build`'s chroot
      hydration calls
      (`src/cli/src/build/rootfs.rs:materialize_build_dependencies`) --
      via `nex resolve`, and pulls those too. `nex resolve` cannot be
      pointed at an exact ref (confirmed directly, not assumed: passing a
      full ref string as the query matches nothing, since
      `find_package_ref`'s matching is always name-based), and this
      repository has real collisions: `binutils`, `gcc`, `bash`,
      `coreutils`, `findutils`, `gawk`, `grep`, `make`, `sed`, `tar`, and
      `xz` each exist twice -- once under `bootstrap/phase1/*` (what
      gzip's manifest names) and once as the self-hosted `core/*`/`cli/*`
      build of the same name and version -- and a single-word query picked
      the wrong one for every single one of those in testing, silently,
      no error. A full `namespace/slug` query (no version) disambiguates
      those correctly but not `glibc`, which collides the other way:
      `libs/system/glibc` is a literal prefix of the unrelated
      `libs/system/glibc-locale-en-gb`, which the substring-matching query
      form picks instead; a single-word query (exact-equality on slug)
      resolves `glibc` correctly. The function tries both forms per
      dependency and verifies the resolved ref's namespace/slug/version
      against the manifest's own declared ref before trusting it,
      failing loudly (not silently seeding a wrong or partial set) if
      neither form matches. Verified compact, not "impractically large":
      one standalone run seeded 15 dep roots plus exactly one `/files`
      closure ref (glibc's own, `~815MB`, `~30k` objects, `zub fsck`
      healthy) in about 70 seconds.
      Rebuilt the cascade
      (`pkg/core/nex/nex.yaml` `25512d668f797e420416c7af8a1edebce7278f99bae778626c731ede217c5b94`,
      `base/nex-systemd.yaml` `fc83635fe2f5c6121df4bed654658fc493fa445d9aaaf8dd6ecbd7353fd526b3`,
      `tests/nex-test-fixture.yaml` `7a8781f3fe195bb626230c38a82cb773c4bc343a4a95c4d28c8d7eb8d266b0ff`
      -- unchanged from the previous unit; nothing in `src/cli` changed
      this unit) and ran the full suite three times: identical every
      time, still 3 of 4 (`build-package` still fails, same text, same
      checksum named, all three runs). Root-caused why the seeding did
      not close the gap, confirmed by direct reproduction rather than
      guessed: `nex build` without `--system`, run as root, uses
      `detect_context`'s *user* context
      (`src/cli/src/repo.rs:163-180`), whose primary repo is
      `/nex/users/root/repo` (empty on a fresh machine), not `/nex/repo`
      (the system store everything gets seeded into). `detect_context`
      does add `/nex/repo` to that context's `fallback_repos` when it
      exists, but `BuildOpts.fallback_repos`
      (`src/cli/src/commands/build.rs`) is populated only from the
      explicit `--fallback-repo` CLI flag, never from
      `NexContext.fallback_repos` -- so the auto-detected fallback the
      context computes is never threaded into
      `materialize_build_dependencies`'s `MaterializeConfig`, and the
      resolver's `self.store.resolve_ref` for glibc's self-referencing
      `/files` need (`src/cli/src/materializer/resolver.rs:308-318`,
      `queue_self_file_dependency`) only ever checks the empty per-user
      repo. Proved this precisely with a disposable diagnostic in a copy
      of `build-package.sh` (reverted after; no product code touched):
      `nex build pkg/cli/archive/gzip.yaml --single --repo /nex/repo`
      (explicit repo, bypassing user-context entirely) resolved the full
      15-commit closure, found every `/files` ref including glibc's,
      downloaded gzip's source, and reached the actual build sandbox --
      proving the seeding is complete and correct. It then hit a
      different, unrelated failure: `unshare: unshare failed: Invalid
      argument`, from the build script's
      `unshare --user --pid --mount --uts --fork --ipc --net
      --map-root-user` sandbox setup
      (`src/cli/src/build/script.rs:264`) -- not investigated further,
      flagged as a second, separate blocker. This is the stop condition
      the human named in advance ("resolving it needs a design
      decision"): no amount of host-side seeding closes this gap, because
      the guest's plain `nex build pkg/cli/archive/gzip.yaml --single`
      (no flags, exactly what a person would type, matching this test's
      own stated purpose) never consults `/nex/repo` for build-dependency
      resolution at all in the user-context path. Fixing it means either
      threading `NexContext.fallback_repos` into `BuildOpts.fallback_repos`
      in product code, or deciding the test should invoke `nex build`
      with `--system` or `--repo /nex/repo` instead of the bare command --
      a product or test-scope decision, not a seeding gap. Stopped per
      instruction rather than routing around it. See Surprises &
      Discoveries and `.agents/knowledge/machine-self-hosting.md`.
- [x] Add the `git_bundle` source kind with a recorded sha256, generated with
      `pack.threads=1` from an explicit commit. `fetch_git_bundle` in
      `src/cli/src/outputs/sources.rs`. Three requirements are not obvious and
      are documented at the call site: `pack.threads=1` for byte-reproducibility,
      a detached worktree (a bare SHA gives "empty bundle"), and an explicit
      commit (a branch name gives a clone with an empty tree).
- [x] Move `examples/desktop-vwl/desktop-vwl.yaml` from its six `dev:` sources
      to one bundle source. **The second half of this step was wrong and was
      not done**: `src/cli/.nex-dev-prepare` cannot be deleted, because
      `pkg/core/nex/nex.yaml:12` still builds the CLI from `dev: src/cli` and
      that script is what makes the source tarball. Deleting it would have
      broken the `nex` package on every machine. It was kept, and the actual
      defect in it fixed instead: `cp -a "$ZUB_PATH" vendor/zub-store` nests
      when the destination already exists, producing
      `vendor/zub-store/zub`, whose `target/` the cleanup then missed. That
      is how 3.8 GB of another repository's build output reached the image.
      It now removes any carried-in copy first.
- [x] Rewrite `nex-init-manifests` in `base/nex-systemd.yaml` to clone the
      bundle, then drop the fixture's override of it. A booted machine reports
      804 commits in `/nex/manifests`, and `git cat-file -t` on the pinned
      environment blob answers `blob`.
- [x] Rename `/nex/repo` to `/nex/store` and `--repo` to `--store`, with a
      migration for machines carrying the old path. Done in two passes,
      because the first missed the half that matters. The CLI rename landed
      first (`SYSTEM_STORE`, `LEGACY_SYSTEM_STORE` and `is_store()` in
      `src/cli/src/repo.rs`), but `pkg/core/kernel/initramfs-init.sh` still
      bound the store at `/nex/repo`, so `/nex/store` stayed an empty
      directory on every machine and all five tests were green through the
      legacy fallback alone: the new name had never carried a store.
      `store_bind_source()` now searches both names on both the root
      partition and `/var`, always preferring a directory that holds an
      actual store over one that is merely present, and binds what it finds
      onto `/nex/store`. `nex-systemd` stopped creating `/nex/repo`, so a
      deployment has one store path. Test `store-upgrade` is the migration's
      proof.
- [x] Rebuild every assembly, record checksums, and run the full suite green.
      Every assembly rebuilt with `--single --check --update-checksum`, both
      builds agreeing in each case: `flat-minimal` (unchanged),
      `flat-systemd` `9a6ceacf`, `flat-podman` `a510f95a`, `nex-minimal`
      `9f068c97`, `nex-systemd` `a93baa49`, `edgebox-rootfs` `f2ae7c90`,
      `desktop-vwl` `76db6f83`, `desktop-dev` `c0730028`, `installer`
      `98f0803d`, `nex-test-fixture` `83aec127`. `installer` was rebuilt a
      second time and produced the same checksum, so its earlier change was a
      caught-up record rather than a moving target.

## Surprises & Discoveries

**2026-08-19: no fixture assembly is needed.** `base/nex-systemd.yaml` ships
openssh and the `sshd` account but never enables the service, and root is
locked (`root:!*:`), so it is not reachable as built. That is deliberate:
`scripts/check-generic-assembly-policy.sh:36` rejects
`multi-user.target.wants/sshd.service` in a reusable assembly as "enabled
remote access".

The access belongs in machine state instead, and a helper already puts it
there. `scripts/prepare-qemu-test-identity.sh:74` writes the `sshd.service` and
`sshd-keygen.service` enablement symlinks into the var tree's
`/etc/systemd/system/multi-user.target.wants/`, alongside `authorized_keys` for
root or a `nex-test` user and an `/etc/ssh/sshd_config`. `qemu-test-live-upgrade.sh:302`
calls it that way.

So SSH access is injected at disk-assembly time rather than carried by an
assembly.

**2026-08-19, correcting the above: `base/nex-systemd.yaml` still cannot serve
as the fixture.** Test 1 ran against it and failed one step earlier than
this plan predicted. It reported:

    manifests-system-exists=true
    manifests-system-is-git=false
    manifests-worktree-exists=false
    pkg-manifest-exists=false
    build-exit=1
    error-line=Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }

`nex-systemd` names `/usr/share/nex/manifests` only as the source path inside
`nex-init-manifests` (line 202); it never populates it, and it ships no `git`
package. Only `examples/desktop-vwl/desktop-vwl.yaml` does either. So
`nex-init-manifests` finds no snapshot at boot, exits 0 after a warning, and
`/nex/manifests` stays an empty directory. `setup_user_manifests_worktree`
(`repo.rs:261`) then cannot create the per-user worktree, because it needs
`/nex/manifests` to already be a Git repository.

The build never got far enough to resolve an environment. That is not a gap in
the finding, because there was never a question there to answer. The design
requirement, restated by the human on 2026-08-19, is that a machine running Nex
always carries the full Git history of the manifests repository, so that every
pinned document is reachable. `nex-init-manifests` doing `git init` plus one
commit over a shipped file tree is therefore wrong code, not a condition to
measure: a one-commit repository cannot hold the historical
`env/standard.yaml` revisions that 476 manifests name.

`tests/nex-test-fixture.yaml` must be written after all. It extends
`nex:base/nex-systemd.yaml` and adds what makes a machine self-hosting: a `git`
package, a Git bundle carrying full history, and an override of
`nex-init-manifests` that clones from that bundle rather than running
`git init`. It must not copy desktop-vwl's `dev:` snapshot, which would
reproduce the historyless repository described above.

**2026-08-19: test 1 fails, and the cause is one step earlier than the
blob-SHA question this plan opened with.** `/nex/manifests` never becomes a
Git repository at all on this fixture, so the question of which revision of
`env/standard.yaml` a historical `manifest_ref` resolves to never comes up.

`base/nex-systemd.yaml` never populates `/usr/share/nex/manifests` (grepped
the whole manifest: no such path). That directory is only ever created by
`examples/desktop-vwl/desktop-vwl.yaml:933-965`, which is out of scope by this
plan's own Decision Log. `nex-init-manifests.service`
(`base/nex-systemd.yaml:195-226`) runs at boot, finds
`/usr/share/nex/manifests` missing, logs "missing source snapshot" to stderr,
and exits 0 without touching `/nex/manifests`. `git` is also not in the
package list, so even if a snapshot existed, `git init` would fail (`command
-v git` guards that path and takes the same silent-exit branch).

The practical effect, confirmed by SSH into a booted overlay: `/nex/manifests`
exists (it's a var-tree mount point) but is an empty directory, not a Git
repository. `nex build pkg/cli/archive/gzip.yaml --single`, run from
`/nex/manifests` (the only manifests-shaped path that exists; the
per-user worktree at `/nex/users/root/manifests` is never created, since
`repo.rs:261`'s `setup_user_manifests_worktree` needs `/nex/manifests` to
already be a Git repo with a HEAD before it can worktree-add from it), fails at
the very first step in `run_build` (`src/cli/src/main.rs:159`,
`Path::new(&args.manifest).canonicalize()?`) because
`/nex/manifests/pkg/cli/archive/gzip.yaml` does not exist. The guest's exact
output:

    Error: Os { code: 2, kind: NotFound, message: "No such file or directory" }

This reproduced identically across three separate fresh-overlay boots. It is a
legitimate FAIL per this plan's Expected Outcome section, just for a more
fundamental reason than the one named there: the machine never gets a
manifests source at all on this fixture, so the question of *which revision*
it would resolve `27b6e5dc` against never arises. Nothing here was changed to
make it pass, per instruction.

**2026-08-19: `tests/nex-test-fixture.yaml` ships a Git bundle, not a
`dev:` tree.** Writing the fixture surfaced two mechanical problems, both
resolved without touching the design:

1. `git bundle create <file> <commit>` refuses with "Refusing to create empty
   bundle" for a bare 40-hex commit SHA; it needs a *named* ref to advertise.
   Bundling a branch name (tested: `reorg-assemblies`, same commit) creates
   the bundle fine but the clone warns `remote HEAD refers to nonexistent
   ref, unable to checkout` and leaves an empty working tree — confirmed by
   direct test, not assumed. The combination that works: `git worktree add
   --detach <dir> <commit>` to pin an exact commit without touching the main
   checkout, then `git -c pack.threads=1 bundle create <out> HEAD` from
   inside that detached worktree. `pack.threads=1` was confirmed to make the
   bundle byte-identical across three regenerations (same sha256); without it
   the plan's own warning about non-determinism would apply.
2. A `sources:` entry of kind `file:` must be tracked by Git
   (`check.rs:377`, `git ls-files --error-unmatch`) or `nex check` fails with
   "path is not tracked by Git". Staging the bundle would satisfy this, but
   the task rule for this unit is to leave everything unstaged. Fix: don't
   use a `sources:` `file:` entry at all. The assembly's own `files:` entries
   support a `source:` field (`system/files.rs:90`, `copy_file_source`) that
   reads directly from a path relative to the repository root at build time,
   with no Git-tracking check anywhere in `check.rs`. The bundle is placed at
   `/usr/share/nex/nex.bundle` via a `files:` entry, same as the
   `nex-init-manifests` override. Confirmed `fs::copy`'s per-copy mtime
   (which is not fixed the way `SOURCE_DATE_EPOCH` fixes build-script output)
   does not break `--check` reproducibility: the two-build checksum matched
   both times this was tried.

Fixture: extends `nex:base/nex-systemd.yaml`, adds `git` (same
`commit`/`manifest_ref` desktop-vwl uses), overrides
`/usr/local/bin/nex-init-manifests` to `git clone /usr/share/nex/nex.bundle
/nex/manifests` (keeping the original's `.git`-already-exists guard) instead
of `git init`, and ships `tests/nex-test-fixture.bundle` (7.8 MB, full history
from commit `f2f342f9008549aec28d10faa7758ec40afd733e`, the tip of
`reorg-assemblies` when this was built). `nex check tests/nex-test-fixture.yaml`
passes. `nex build tests/nex-test-fixture.yaml --single --check
--update-checksum` is strict two-build reproducible; checksum
`a22e42ab383d0c108c3fb6f2ada6899c6f3a21ff31aed3c05e61b112aee6b507`, stored at
`systems/nex-test-fixture/0.0.1`.

**2026-08-19: test 1 against the new fixture gets past the finding above,
then fails one level deeper, in `load_environment` itself.** Rerun (twice,
identical both times) with the harness pointed at
`systems/nex-test-fixture/0.0.1`:

    manifests-system-exists=true
    manifests-system-is-git=true
    manifests-commit-count=784
    manifests-pinned-env-blob-reachable=true
    manifests-worktree-exists=false
    build-cwd=/nex/manifests
    pkg-manifest-exists=true
    Created manifests worktree at /nex/users/root/manifests
    Created user environment at /nex/users/root
    Building package: cli/archive/gzip
    Error: Custom { kind: NotFound, error: "Failed to load environment blob 27b6e5dc7ad152c9a17c2cabfcc5ee93daa9bbf0: fatal: not a git repository (or any parent up to mount point /nex)\nStopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).\n" }
    build-exit=1

Full history is confirmed present and reachable (`manifests-commit-count=784`
matches `git rev-list f2f342f9... | wc -l` on the host that built the bundle;
the pinned blob itself is confirmed reachable by `git cat-file -e`, not just
inferred from commit count). The worktree machinery this plan flagged as
untested now runs: `/nex/manifests` clones with a resolvable `HEAD`,
`setup_user_manifests_worktree` (`repo.rs:261`) successfully runs `git
worktree add --detach /nex/users/root/manifests HEAD`, and `nex build` reaches
`load_environment` and starts building the package.

It still fails there, for a reason this plan had not identified: `run_build`
passes `opts.repo_path` to `load_environment` (`src/cli/src/build/package.rs:64`),
and `opts.repo_path` is the *zub store* (`/nex/users/root/repo` in user
context, `detect_context`, `repo.rs`), not the manifests Git repository.
`load_environment_blob` (`src/cli/src/build/env.rs:47`) runs `git -C
<repo_path> cat-file blob <sha>`. `git -C` changes directory then lets normal
upward discovery find the nearest `.git`; on a dev checkout this works by
coincidence, because `.nex/repo` happens to sit inside the Nex source
checkout's own `.git`. On an installed machine `/nex/users/root/repo` is a
sibling of `/nex/manifests` under `/nex`, not a descendant of it, so no amount
of history in `/nex/manifests` makes this resolve: the walk never reaches a
`.git` at all, on any machine, regardless of what `/nex/manifests` contains.
This is a different, more specific bug than "one-commit repository missing a
blob" — it is "blob resolution is wired to the wrong directory" — and it is
squarely phase B of this plan (which fixes where environment blobs resolve;
see `.agents/knowledge/environment-pinning.md` and the `store` term in Context
and Orientation below). Per instruction, nothing was changed to route around
it: no repinning, no environment edits, no extra history.

**2026-08-19: tests 2 and 3 (`nex stage`) fail on this fixture because the
kernel bundle carries no `overlay.ko`, not because of anything in `nex`
itself.** `base/nex-systemd.yaml` names
`x86_64/pkg/core/kernel/linux/6.18.24/bundles/vm` for its kernel. That
bundle's component list (`pkg/core/kernel/linux.yaml:516-524`) is `boot,
drv-net-misc, drv-net-virt, drv-net-virtio, drv-virtio, lib, modules-meta,
net-misc` — no `fs-overlay`. The single output that carries the module,
`fs-overlay` (`pkg/core/kernel/linux.yaml:6141-6143`,
`/usr/lib/modules/6.18.24/kernel/fs/overlayfs/overlay.ko`), is a member of
exactly one bundle, `all-modules`, a much larger one. Confirmed on the guest:
`kernel/fs/overlayfs/` does not exist under `/usr/lib/modules/6.18.24` at
all; `modprobe overlay` fails ("Unknown symbol in module, or unknown
parameter"), `insmod` on the literal path fails ("No such file or
directory"), and `/proc/filesystems` has no `overlay` line. `nex stage`
(`src/cli/src/commands/stage.rs`) unconditionally does `mount -t overlay` on
`/usr/bin`, `/nex/pkg`, `/nex/env`, so it fails outright, the same way, every
time:

    mount: /usr/bin: unknown filesystem type 'overlay'.
    Error: Custom { kind: Other, error: "Failed to mount overlay on /usr/bin" }

This blocks `nex install --system` too, since it requires staging first.
Test 4 (deploy/rollback) needs neither staging nor overlayfs and is
unaffected — it passed cleanly, twice in a row.

Nothing was changed to route around this: swapping in `bundles/all-modules`
(or hand-adding the `fs-overlay` output) would make tests 2 and 3 pass, but
that is a choice about what this test fixture's kernel should carry, not a
mechanical follow-on from anything asked for in this unit, so it was left
alone and is reported here instead of decided.

**2026-08-19: the human added `kernel-fs-overlay` to the fixture; `nex stage`
now succeeds, and both install tests fail one step later, at `nex install`,
on a real dependency-resolution gap.** New checksum
`69c012db66f612dc7074ab29d8b076876382a33e8cbda20396d410710d5fb487`, strict
two-build reproducible, `nex check` passes. Rebuilt the backing image and ran
the full suite twice; identical both times.

`temporary-install` and `persistent-install` both now get past `stage-exit=0`
and fail at `install-exit=1`, with:

    Installing x86_64/pkg/dev/vcs/tig/2.6.0/outputs/bin...
    Materializing 1 request(s)...
      Resolving runtime dependencies (precomputed)...
      Loaded manifest index: 533 manifests, 162938 files
      Runtime closure: 1 commit(s)
    Error: Custom { kind: NotFound, error: "4 unresolved runtime dependency requirement(s):
      pkg/dev/vcs/git (run: nex compute-deps pkg/pkg/dev/vcs/git.yaml)
        needed by: /usr/bin/tig needs git - missing 51b15102f212/files commit
      pkg/libs/system/glibc (run: nex compute-deps pkg/pkg/libs/system/glibc.yaml)
        needed by: /usr/bin/tig needs glibc - missing bf348eabcec2/files commit
      pkg/libs/system/ncurses (run: nex compute-deps pkg/pkg/libs/system/ncurses.yaml)
        needed by: /usr/bin/tig needs ncurses - missing 05d611ad0722/files commit
      pkg/libs/system/readline (run: nex compute-deps pkg/pkg/libs/system/readline.yaml)
        needed by: /usr/bin/tig needs readline - missing 6c7351f248a8/files commit" }

Traced to `src/cli/src/materializer/mod.rs`: `nex install`'s
`resolve_runtime_deps_precomputed` reads the target package's precomputed
`needs` metadata and requires a `/files` commit for *every* runtime
dependency to already be resolvable in the store (`config.repo_path` plus
fallbacks) — not just the requested package's own ref. The harness had only
seeded `tig`'s own `outputs/bin` ref, not its dependency closure's `/files`
refs, so this fails deterministically. This is a different mechanism from the
"flatten deps into the package's own capsule" step used during *system
assembly* builds, so a package being self-contained there does not carry over
to `nex install`. Full writeup: `.agents/knowledge/machine-self-hosting.md`
("Installing an already-built package still needs its dependencies' own
store refs"). Per instruction, this was reported and nothing was seeded to
route around it.

**2026-08-19: `examples/desktop-vwl/desktop-vwl.yaml` failed to build after
the `base/nex-systemd.yaml` cascade — correctly diagnosed by the human as two
packages delivering the same file, not the zub bug I first reported.**
`desktop-vwl.yaml:67` takes the kernel's `bundles/all-modules`, which already
contains the `fs-overlay` output, and now also inherits the human's new
`kernel-fs-overlay` package from `base/nex-systemd.yaml`. Both deliver
`/usr/lib/modules/6.18.24/kernel/fs/overlayfs/overlay.ko`, and the "direct
layer" kernel-module installer fails placing the second one where the first
already wrote:

    Installing kernel modules: core/kernel/linux/6.18.24 (direct layer)
    Error: Custom { kind: Other, error: "io error at .../target/usr/lib/modules/6.18.24/kernel/fs/overlayfs/overlay.ko: No such file or directory (os error 2)" }

I had reported this as a probable `zub` hardlink-dedup bug, reasoning from
two facts that were true but tested the wrong scenario (a standalone `zub
checkout --copy` of `bundles/all-modules` alone, and the same checkout
`--force`-repeated over itself — neither reproduces two *different* refs
writing the same path in sequence, which is what a real build does). The
human corrected this by testing the actual first suspect, the manifest
change, and found the real cause immediately: `desktop-vwl.yaml` never
excluded the base's newly-added package. Fix (the human's, not mine): add
`exclude: packages: [kernel-fs-overlay]` in `desktop-vwl.yaml`. Builds
reproducibly at
`f07249fc4b6a0c5ba5cd620b4ecba941f484ccb79180e1e37f4be5cfa8e1972b`. Full
corrected writeup in `.agents/knowledge/kernel-and-boot.md`.

**2026-08-19: after fixing dependency-closure seeding, three of four tests
fail on the same new error, `uid 0 not mapped in namespace`, including
`deploy-and-rollback`, which had passed every run before this unit.**
Reproduced twice, identically. Full command sequence: rebuilt
`pkg/core/nex/nex.yaml`, `base/nex-systemd.yaml`,
`tests/nex-test-fixture.yaml` (checksums above), fixed
`seed_system_repo()` to seed `tig`'s full closure, rebuilt the backing image,
ran `./scripts/test-machine-operations.sh` twice.

`temporary-install`/`persistent-install`, at `nex install`:

    Runtime closure: 6 commit(s)
      Checking out to /...
      Checking out dev/vcs/tig/2.6.0 (1 output) -> /nex/pkg/dev/vcs/tig/2.6.0/0cbb5716
    Error: Custom { kind: Other, error: "uid 0 not mapped in namespace" }

Confirms the closure-seeding fix worked: 6 commits resolved (tig + git +
glibc + ncurses + readline + zlib, exactly what `nex resolve tig -v` names),
and the checkout actually starts. It fails one step further in, during the
real file materialization.

`deploy-and-rollback`, at `nex deploy`:

    Target:      /sysroot/nex/deployments/27e6eea03bcbf309e1199b26ed8629a168943afa2b58bb2010f7efcbf1493482.1
    Error: Custom { kind: Other, error: "uid 0 not mapped in namespace" }

Traced the error's origin (not fully to a root cause): `uid {0} not mapped in
namespace` is `zub::Error::UnmappedUid`
(`/home/wegel/work/perso/zub/src/error.rs:45`), raised by
`inside_to_outside(uid, &ns.uid_map)` returning `None`
(`/home/wegel/work/perso/zub/src/object/blob.rs:37,132`) inside `write_blob`
-- a *commit-time* function, called from a checkout by the hardlinked-checkout
repair path (the comment there: "A hardlinked checkout may have modified an
older store object... The atomic rename below repairs a mismatching
object"). `seed_system_repo`'s guest `nex/repo` config
(`write_zub_config()` in `scripts/test-machine-operations.sh`) sets
`uid_map = []`, `gid_map = []` -- copied verbatim from
`scripts/qemu-test-live-upgrade.sh`'s own `write_zub_config`, an established
pattern, not something changed this unit. An empty map has no entry
matching uid 0, so any code path that calls `inside_to_outside` on it fails
for any real (root-owned) file, by construction; `NsConfig::identity()`
(`zub/src/namespace/mapping.rs:49`) is what an unrestricted mapping actually
looks like (`[MapEntry(0, 0, u32::MAX)]`), not an empty vec.

What is not established: *why this started failing now* rather than always.
The config did not change. Two candidate explanations, neither confirmed:
either this write-blob repair path was never reached before (every prior
`nex install` run failed earlier, on the dependency-closure gap, before
reaching a real checkout; only `deploy-and-rollback` reached a real checkout
before, and always against a nex-systemd build nobody had rebuilt this
session), or something about content freshly built this session (the new
`nex` binary, or the rebuilt `nex-systemd`) differs from what was checked out
in every prior successful `deploy-and-rollback` run in a way that now
triggers the repair path. Not chased further, and not fixed: whether the fix
belongs in the harness's zub config (identity mapping instead of empty), in
zub itself, or somewhere else is exactly a design question, reported rather
than decided.

**2026-08-19, same day: the human fixed the uid-mapping bug; `deploy-and-rollback`
passes again, and the install tests reach a new, real, precisely-located
blocker before `nex commit`.** `write_zub_config()` now writes an explicit
identity range for `uid_map`/`gid_map` (`inside_start=0, outside_start=0,
count=65536`) instead of an empty list — the guest runs as real root, so
identity is the correct mapping, unlike the host's own rootless
`.nex/repo/config.toml` (inside 0 -> outside 1000). Confirmed:
`scripts/qemu-test-live-upgrade.sh:168` has the identical empty-map pattern
and the identical latent bug, simply never reached; left alone, recorded
only, per instruction.

No rebuild was needed for the fix itself: the config is written into the
guest's var tree at disk-assembly time (`stage_root_and_var` ->
`write_zub_config`), and the harness's `build_backing_image()` already does
`rm -rf "$WORK_DIR"` and rebuilds fully on every invocation of
`scripts/test-machine-operations.sh`, so there was no stale cached backing
image to clear.

Ran the full suite twice, identical both times:

    FAIL: build-package (guest exit 1)
    FAIL: temporary-install (guest exit 1)
    FAIL: persistent-install (guest exit 1)
    PASS: deploy-and-rollback (verify-deployed)
    PASS: deploy-and-rollback
    PASS: deploy-and-rollback
    tests run: 4
    failures: 3

`deploy-and-rollback`: back to passing, as it did every run before the
regression, across two real reboots each time.

`build-package`: unchanged from before, and expected: `Runtime closure: 15
commit(s)`, then 12 unresolved requirements rooted in
`x86_64/pkg/libs/system/glibc/2.39/bf348eabcec257edace3e1e05458bf79ddad1a5164f25e706b7e50d93b25190d/files
is missing` — gzip's own build-sandbox dependency closure is not in the
guest's store, and was never in scope to seed (that would mean seeding the
transitive build closure of an arbitrary package, which defeats the point of
proving whether a machine can build one from nothing). Left exactly as
reported; nothing seeded to route around it.

`temporary-install`/`persistent-install`: both now get past the checkout that
was failing (`Checking out dev/vcs/tig/2.6.0 (1 output) ->
/nex/pkg/dev/vcs/tig/2.6.0/0cbb5716`, `Flattened 6 libs...`,
`Materialization complete.`) — the uid-mapping fix and the closure-seeding
fix both hold — then fail immediately after with `Error: Os { code: 30, kind:
ReadOnlyFilesystem, message: "Read-only file system" }`, reproduced
deterministically twice, before ever reaching `nex commit`. Confirmed with a
temporary diagnostic added to the guest script (reverted after): `/nex/pkg`
exists and is read-only (part of the assembled, read-only deployment root);
`/nex/env` does not exist at all. `mount_nex_overlays()`
(`src/cli/src/commands/stage.rs`) creates `/nex/pkg`'s overlay mountpoint
fine (it already exists) but then tries `fs::create_dir_all("/nex/env")`,
which fails because `/nex` itself is on the read-only deployment root and
nothing in `base/nex-systemd.yaml`'s build script ever creates `/nex/env`
(it creates `/nex/repo`, `/nex/deployments`, `/nex/users`, `/nex/staging`,
`/nex/manifests` — not `/nex/env`). This is every first `nex install
--system` on a machine assembled this way, not specific to `tig` or to this
fixture. Full writeup: `.agents/knowledge/machine-self-hosting.md` ("`nex
install --system` can't create `/nex/env` on a machine that never had one").
Not fixed, not routed around: whether `/nex/env` belongs in the assembly's
directory list or `mount_nex_overlays()` needs a different target is a
product design question. `nex commit`'s own "may want a store ref nothing
creates" question therefore remains unanswered — install still can't
complete far enough to reach it.

**2026-08-19, later still: `/nex/env` fixed by the human; `nex install`
completes for the first time; two new findings, one per install test, and
the `nex commit` question is finally answered.** `base/nex-systemd.yaml`
checksum `1ae2b42c1a0e5f24f6bf1473f0eb7089340151479c733dcf731ca6c7a2230860`;
`tests/nex-test-fixture.yaml` checksum
`7a902425d6315f6d66930b5a26fe4b41219baa3729a6e6cae9c0657cf691c034`; both
strict two-build reproducible. Full suite twice, identical both times:

    FAIL: build-package (guest exit 1)
    FAIL: temporary-install (guest exit 1)
    FAIL: persistent-install (guest exit 1)
    PASS: deploy-and-rollback (verify-deployed)
    PASS: deploy-and-rollback
    PASS: deploy-and-rollback
    tests run: 4
    failures: 3

`temporary-install` and `persistent-install`, identically, up through
install:

    stage-exit=0
    Installing x86_64/pkg/dev/vcs/tig/2.6.0/outputs/bin...
    Materializing 1 request(s)...
      Resolving runtime dependencies (precomputed)...
      Loaded manifest index: 533 manifests, 162938 files
      Runtime closure: 6 commit(s)
      Checking out to /...
      Checking out dev/vcs/tig/2.6.0 (1 output) -> /nex/pkg/dev/vcs/tig/2.6.0/0cbb5716
      Flattened 6 libs into dev/vcs/tig/2.6.0/0cbb5716
    Materialization complete.
      Linked tig -> ../../nex/pkg/dev/vcs/tig/2.6.0/0cbb5716/usr/bin/tig
      Linked git -> ../../nex/pkg/dev/vcs/tig/2.6.0/0cbb5716/usr/bin/git
    Installed dev/vcs/tig 2.6.0 (set as current)
    install-exit=0
    tig version 2.6.0
    ncurses version 6.4.20230520
    readline version 8.2
    run-exit=0

Every fix from this plan holds at once: full manifest history, the seeded
dependency closure, the uid mapping, and `/nex/env` all needed for this to
work, and now do. `nex install` completes and the installed binary runs.
This is the first fully successful install anywhere in this plan.

`temporary-install` then calls `nex discard --force` and fails:

    Discarding staging changes...
    umount: /nex/pkg: target is busy.
    Error: Os { code: 30, kind: ReadOnlyFilesystem, message: "Read-only file system" }
    discard-exit=1

New finding, reproduced identically twice. Not chased to a root cause this
time (reporting only what was directly observed, after getting the earlier
`zub` diagnosis wrong from too little evidence): `/usr/bin`'s unmount
produced no message; only `/nex/pkg`'s reported busy. `cleanup_staging()`
then fails removing `/nex/staging` itself with `Read-only file system`. Full
detail: `.agents/knowledge/machine-self-hosting.md` ("`nex discard` can
fail...").

`persistent-install` calls `nex commit` and fails:

    Committing changes: install tig for persistent-install test
    Error: Custom { kind: NotFound, error: "Could not determine current deployment" }
    commit-exit=1

This is the answer to the question this plan has been carrying since the
`nex stage`/`nex install` blockers first went away: `get_current_deployment_ref()`
(`src/cli/src/commands/commit.rs:156`) looks for a `nex/deployments/*` ref,
then falls back to a literal `nex/base` ref. Neither exists on a machine
built by this fixture, and grepping `src/cli`, `scripts`, and `installer`
finds nothing anywhere in this repository that creates either, on any
machine. `nex commit`'s store-ref path (taken whenever `/nex/repo` exists,
which it does here) cannot get past a first install on any machine today.
Reported exactly, per instruction; not routed around.

`build-package` and `deploy-and-rollback` are unchanged from the previous
entry.

**2026-08-19, later still: a deployment ref is now published in the right
place, and `persistent-install` fails at `nex commit` with the exact same
error anyway -- a second, independent, likely cause found by reading the
matching code, not yet confirmed by a standalone test.** Rebuild cascade in
dependency order (`pkg/core/nex/nex.yaml` builds from `dev: src/cli`, so the
human's `deploy.rs`/`store/mod.rs` changes reach the guest only once this
package is rebuilt):

    pkg/core/nex/nex.yaml        3d766d5c90c456045045c54b4f0fc664d753a5b1c2451faed558571ecbdbceda
    base/nex-systemd.yaml        79ddf69671945f774c6955aa898b257b5c5ccaafdac97c28866046a7f200c645
    tests/nex-test-fixture.yaml  26f48b39b377a76f200d59a19e8ec5fefcb557ae9977291e8b9138449bf2eb53

All three strict two-build reproducible. Full suite twice, identical both
times:

    FAIL: build-package (guest exit 1)
    FAIL: temporary-install (guest exit 1)
    FAIL: persistent-install (guest exit 1)
    PASS: deploy-and-rollback (verify-deployed)
    PASS: deploy-and-rollback
    PASS: deploy-and-rollback

`persistent-install` reaches `nex commit` and fails with byte-identical
output to before the ref-publishing fix:

    Committing changes: install tig for persistent-install test
    Error: Custom { kind: NotFound, error: "Could not determine current deployment" }
    commit-exit=1

`temporary-install` also unchanged: `nex install` still succeeds completely
(checkout, symlinks, the binary runs), `nex discard` still fails the same
way (`/nex/pkg` busy, then `Read-only file system` removing
`/nex/staging`). `deploy-and-rollback` still passes, now with the new
publish-a-ref code path exercised silently (no stdout evidence either way,
since `deploy.rs` doesn't print anything about it).

`persistent-install`'s unchanged failure, despite a ref now genuinely
existing at `refs/heads/nex/deployments/<checksum>.0` (confirmed by
inspecting `zub`'s own ref layout: `Repo::refs_path()` in
`/home/wegel/work/perso/zub/src/repo.rs:112` is `<repo>/refs/heads`, so a ref
named `nex/deployments/X` lives on disk at exactly the path
`seed_system_repo()` writes), pointed at a real second suspect:
`get_current_deployment_ref()` calls `store.refs(Some("nex/deployments/"))`
(`src/cli/src/commands/commit.rs:159`) to find it. That resolves to
`zub::list_refs_matching(repo, "nex/deployments/")`
(`/home/wegel/work/perso/zub/src/refs.rs:114`), which builds a
`glob::Pattern` from the given string and keeps only refs where
`pattern.matches(ref_name)`. A pattern with no wildcard character matches
only that exact string -- standard, documented behavior for the `glob`
crate, and confirmed against this codebase's own usage: `zub`'s own test of
this exact function, `test_list_refs_matching`
(`/home/wegel/work/perso/zub/src/refs.rs:381`), passes `"x86_64/*"` -- with a
trailing `*` -- to get prefix-style matching, and a second call there
(`"*/pkg/foo/*"`) also always carries a wildcard. `"nex/deployments/"`, as
written in `commit.rs`, has none, so (if this reading holds) the call would
return an empty list regardless of what refs exist under that prefix, making
`nex/deployments/*` refs permanently invisible to `get_current_deployment_ref()`
by construction, independent of whether anything ever publishes one.

This is reported as a strong, evidence-based hypothesis, not a confirmed
fact: I did not write and run a standalone test to watch
`store.refs(Some("nex/deployments/"))` return empty against a repo holding
exactly that ref, and I got an earlier diagnosis in this same file wrong
before by reasoning from adjacent-but-not-identical evidence (the `zub`
hardlink note, since corrected). No product code was changed to check or
route around this.

**2026-08-19, later still: the glob theory was right, and a third,
independent cause -- this time a harness bug of my own -- was blocking
`nex commit` even after the human's fix landed.** The human confirmed the
glob theory directly (a two-line repro), fixed `commit.rs:159`, added a
pinning test, and committed as `d1e79485`. Rebuild cascade in the same
order:

    pkg/core/nex/nex.yaml        08cb193376fb4242149c554c2bb4d9d802343bd2b3c2c849af7eb59c0c8a3351
    base/nex-systemd.yaml        81b9557705e99d4214bd417fd9793226e9a4cad6bda642229ed57ca4ed810799
    tests/nex-test-fixture.yaml  2335f8ba7481717d770eb369fbfe96fcada5740e04e4c1d468f789fadd247be5

All three strict two-build reproducible. `persistent-install` still failed
with the byte-identical `Could not determine current deployment` error.
Before assuming the fix hadn't worked, checked the log for the harness's own
diagnostic line first, and found it: `note: could not resolve
systems/nex-test-fixture/0.0.1, initial deployment ref not published`, on
every run. `seed_system_repo()` (`scripts/test-machine-operations.sh`)
publishes `nex/deployments/<checksum>.0` by `rev-parse`-ing `$FROM_REF`
*against the guest's own store*, but `$FROM_REF` is never pulled into that
store anywhere -- the fixture's content only ever gets *checked out* (via
`zub checkout --copy`, from the host's own repo) directly into the root
filesystem content, a completely separate mechanism from the object-pull
`seed_system_repo()` uses for everything else it seeds. So the `rev-parse`
always failed, and the failure was swallowed (`2>/dev/null || true`),
landing silently in the "could not resolve" branch. This is harness code,
not product code, and unambiguously a bug in implementing what was already
the stated intent, not a design question, so I fixed it directly: pull
`$FROM_REF` into the guest's store before resolving it, same as every other
seeded ref.

With that fixed, ran the full suite twice more, identical both times.
`nex commit` now runs its real logic and succeeds at it:

    Committing changes: install tig for persistent-install test
      Current deployment: nex/deployments/<fixture-checksum>.0
      Checking out current deployment...
      Applying staged changes...
      Creating new commit...
      Created deployment: nex/deployments/<unix-timestamp>

This is the first time in this plan that `nex commit` has done real work.
But the command still exits non-zero and `persistent-install` still fails,
because `create_deployment()` (`commit.rs`) calls `cleanup_staging()`
afterward -- the exact same function `nex discard` calls -- and it hits the
exact same failure `temporary-install` already reported: `/nex/pkg` reports
busy on `umount`, then `fs::remove_dir_all("/nex/staging")` hits `Read-only
file system`. So this was never two separate bugs, one per test; it is one
bug in `cleanup_staging()`, reached from both commands.

Given persistent-install did not go green, I spent the remaining effort
diagnosing that shared failure, per instruction: report observations first,
separately from any proposed mechanism, and change no product code (all
diagnostics below were added to a disposable copy of the guest script and
reverted immediately after each check; `git diff` on it is clean).

**Observations:**

- `fuser` and `lsof` are not installed on this fixture.
- Immediately before cleanup runs, `/proc/mounts` shows three active overlay
  mounts: `/usr/bin`, `/nex/pkg`, `/nex/env` -- all three get mounted
  unconditionally by `mount_nex_overlays()`, even on a run where nothing was
  ever installed under `/nex/env`.
- A manual `umount /nex/pkg`, run directly in the guest shell before `nex
  discard`/`nex commit` is invoked at all, fails the same way: `umount:
  /nex/pkg: target is busy` (exit 32). Not specific to how `nex` itself
  calls `umount`, and not a timing artifact of running right after install.
- After the failed cleanup, `/proc/mounts` shows `/usr/bin` and `/nex/env`
  successfully unmounted; only `/nex/pkg` remains mounted.
- `/nex/staging` itself is writable both before and after the failed
  cleanup (`touch` succeeds both times). Writing into `/nex/pkg` afterward
  fails with "No such file or directory", not "Read-only file system".
  `/nex/staging/upper/nex/pkg` -- the still-mounted overlay's own upperdir
  -- no longer exists at all after the failed cleanup, even though
  `/proc/mounts` still lists that exact path as the mounted overlay's
  `upperdir=`.
- Scanning `/proc/*/cwd`, `/proc/*/fd/*`, and `/proc/*/maps` for `/nex/pkg`
  references while `/nex/pkg` was confirmed busy: PID 1 (`systemd`, the
  init process) has file descriptor 9 open on
  `/nex/pkg/core/init/systemd/257.5/1223026e/usr/lib/systemd/systemd-executor`,
  and roughly a dozen other running PIDs have `/nex/pkg` paths present in
  their memory maps. Separately confirmed `/usr/lib/systemd/systemd-executor`
  is a real symlink to exactly that path, and that `/usr/lib` in general is
  full of symlinks into `/nex/pkg/<namespace>/<slug>/<version>/<checksum>/...`.

**Proposed mechanism** (kept separate from the observations above, and not
acted on): on a `nex_structure: true` system, FHS paths like `/usr/lib` and
`/usr/bin` are symlinked directly into package capsules under `/nex/pkg`, so
every running process that has loaded a shared library or executed a helper
binary holds it open or mapped from there -- starting with PID 1, which
cannot be stopped to release it. Mounting an overlay on `/nex/pkg`
(`mount_nex_overlays()`) succeeds because mounting only needs the
mountpoint directory to exist. Unmounting it does not: the kernel refuses a
busy `umount`, `cleanup_staging()`'s plain `Command::new("umount")` (no
`-l`/`-f`) does not force it and silently discards the failure either way,
and the subsequent `fs::remove_dir_all` then tries to recurse into the
still-mounted overlay's own live upperdir, which is where the visible
`Read-only file system` error actually surfaces. On this reading, `/nex/pkg`
may be structurally unable to fully unstage while any process is running --
not about `tig`, not about timing, not fixable by retrying.

`temporary-install` (`nex discard`) and `deploy-and-rollback` are unchanged
from before this unit; `build-package` is unchanged, still failing on
gzip's build-time dependency closure.

**2026-08-19, later still: the lazy-unmount fix landed and independently
verified both halves of the diagnosis; the mount-level failure is gone, and
a third, deeper layer of the same shared `cleanup_staging` problem is now
the blocker.** The human verified `/usr/lib/systemd/systemd-executor`'s
symlink target and that 199 of the first 200 `/usr/lib` symlinks point into
`/nex/pkg` directly, confirming PID 1 genuinely pins the mount. Fixed in
`stage.rs`, committed `f1a7dcfb`: `cleanup_staging` now calls a new
`unmount_overlay` per target that skips an already-unmounted target, tries
a plain `umount`, falls back to `umount -l` (lazy) when busy, and only
returns an error (naming the mount) if both fail -- failures are no longer
silently swallowed.

Rebuild cascade, same order:

    pkg/core/nex/nex.yaml        325803c9b11b1910ab9401aa82fd6345cf00ef6bcd0c98e7f659ab650c151a02
    base/nex-systemd.yaml        bf3fddb017733ea2b04310e2cfd54b8040c202f1d14cbf59a69196973641e0f5
    tests/nex-test-fixture.yaml  b62f51326661982ee51ba674858de1652f30427247b9cedd223d7c2e1e36e790

All three strict two-build reproducible. Full suite twice, identical both
times:

    FAIL: build-package (guest exit 1)
    FAIL: temporary-install (guest exit 1)
    FAIL: persistent-install (guest exit 1)
    PASS: deploy-and-rollback (verify-deployed)
    PASS: deploy-and-rollback
    PASS: deploy-and-rollback

Not three of four green. `temporary-install` and `persistent-install` still
fail, with byte-identical error text to before the fix:

    Error: Os { code: 30, kind: ReadOnlyFilesystem, message: "Read-only file system" }

Confirmed via disposable diagnostics on a copy of the guest script (reverted
after; no product code touched) that the mount-level problem this fix
targeted is genuinely gone: after a failed `nex discard`, `/proc/mounts` no
longer lists any of the three overlays at all (`/usr/bin`, `/nex/pkg`,
`/nex/env` all detached, lazily where the plain unmount was refused), and
`/nex/staging` is confirmed completely empty afterward (`ls -la
/nex/staging` shows only `.` and `..` -- `upper/`, `work/`, and the `active`
marker are all gone). The coordinator's flagged risk did not materialize
either: `/usr/bin/tig` cleanly disappears, `command -v tig` finds nothing,
`tig --version` reports "command not found" -- no stale post-discard view.

One layer deeper: `cleanup_staging`'s `fs::remove_dir_all("/nex/staging")`
now successfully empties the directory but still fails to remove
`/nex/staging` itself. `/proc/mounts` shows `/nex/staging` is its own
mounted filesystem, `/dev/sda2 /nex/staging ext4 rw,relatime`, separate from
the deployment root -- matching the "boot mountpoints... bind-mounts
sysroot/var/nex into the deployment" comment in `base/nex-systemd.yaml`'s
build script, the same mechanism as `/nex/repo`, `/nex/users`,
`/nex/manifests`, `/nex/env`. `rmdir` on a directory needs write permission
on that directory's *parent* to remove the entry, independent of whether
the target itself is a separate writable mount; `/nex/staging`'s parent is
`/nex`, which is on the read-only deployment root. So `rmdir("/nex/staging")`
fails with the same EROFS regardless of how empty `/nex/staging` is. This
reads as structural, not content- or timing-dependent: nothing can ever
remove the `/nex/staging` mountpoint directory itself while the deployment
root stays read-only, so `cleanup_staging` clearing everything *inside* it
and then trying to `rmdir` it cannot succeed as written. Not fixed, not
routed around; observation and proposed explanation both recorded, kept
separate, in `.agents/knowledge/machine-self-hosting.md`.

`persistent-install` reaches `nex commit`'s real work again -- `Current
deployment: nex/deployments/<checksum>.0`, checks it out, applies staged
changes, `Created deployment: nex/deployments/<timestamp>` -- and fails at
the same shared `cleanup_staging` step immediately after, so it still never
reaches reboot; that ground remains untested. `deploy-and-rollback` and
`build-package` are unchanged.

**2026-08-19, later still: the mount-point fix landed; `temporary-install`
passes completely for the first time; `persistent-install` reaches reboot
for the first time and its after-reboot assertion reveals a real product
gap, not a test bug.** Fixed in `stage.rs`, commit `eacb11f5`:
`cleanup_staging` now calls `clear_staging_state`, which removes only
`/nex/staging`'s *contents*, leaving the mount point directory alone (it is
one of `base/nex-systemd.yaml`'s own boot mountpoints, created on purpose).
Also added a `current-deployment=` line to `persistent-install.sh`'s
after-reboot phase (harness-only, same `/proc/cmdline` `zub=` convention
already used elsewhere in this plan), so a failed assertion can be
diagnosed without a follow-up unit.

Rebuild cascade, same order:

    pkg/core/nex/nex.yaml        3e6805c8a7250f84ee21d2f7818bd84a73e492cea1d11ba3d8f5d56671ef8ed4
    base/nex-systemd.yaml        f3b550a81361d56006da2026fed50c2505ad8adb102144ef1f524b469c07b25d
    tests/nex-test-fixture.yaml  319b995d56114e057ff8991e317dfd1aea2edb51629c45fd4c91502c158c570d

All three strict two-build reproducible. Full suite twice, identical both
times:

    FAIL: build-package (guest exit 1)
    PASS: temporary-install
    FAIL: persistent-install (guest exit 1)
    PASS: deploy-and-rollback (verify-deployed)
    tests run: 4
    failures: 2

`temporary-install`, full output, both runs identical: stage, install
(`Runtime closure: 6 commit(s)`, checkout, symlinks), the installed binary
runs (`tig version 2.6.0`), discard (`umount: /nex/pkg: target is busy.`
still prints -- informational now, from the plain-unmount attempt before the
lazy fallback -- `discard-exit=0`), and the binary is confirmed gone
(`post-discard-binary-present=false`, `post-discard-staging-active=false`).
This is the first install-and-remove journey in this plan to go fully
green.

`persistent-install`: `nex commit` now succeeds outright --

    Committing changes: install tig for persistent-install test
      Current deployment: nex/deployments/319b995d56114e057ff8991e317dfd1aea2edb51629c45fd4c91502c158c570d.0
      Checking out current deployment...
      Applying staged changes...
      Creating new commit...
      Created deployment: nex/deployments/1787180328
    umount: /nex/pkg: target is busy.
    Deployment created successfully.
    commit-exit=0

-- and the host reboots the guest cleanly for the first time on this path.
The after-reboot assertion then fails:

    phase=after-reboot
    current-deployment=319b995d56114e057ff8991e317dfd1aea2edb51629c45fd4c91502c158c570d.0
    post-reboot-binary-present=false
    error-line=tig not present after reboot

Per instruction, checked which deployment actually booted before concluding
the install was lost: `current-deployment` after reboot is the exact same
`<fixture-checksum>.0` the machine was running before `nex commit` ran --
not a different, unexpected deployment, and not absent (which would suggest
a boot-selection problem); the machine simply never left the deployment it
started in. Traced why: `create_deployment()`
(`src/cli/src/commands/commit.rs:108`) calls only `store::commit_tree(NEX_REPO,
&new_ref, ...)` -- it writes the new content into the *store* as a ref named
`nex/deployments/<unix-timestamp>`, and never touches `/sysroot`, the
on-disk `/nex/deployments/<checksum>.<serial>` directories `nex
deploy`/`nex rollback` create, or anything boot-time deployment selection
looks at. The commit itself is correct and complete (a real ref exists, with
`nex.deployment.parent` naming the prior deployment and the commit message
attached), it is just never promoted to something bootable. So a plain
reboot after `nex commit`, with no other command, can never boot the new
content -- this is not about `tig`, not a race, and not fixable by waiting
or rebooting again. Not fixed, not routed around: the test was not changed
to also call `nex deploy` on the new ref, since that would test a different
operation than "install a package and keep it across a reboot" as the plan
specifies it. Full detail in
`.agents/knowledge/machine-self-hosting.md` ("`nex commit` records a new
deployment in the store, but never activates it").

`deploy-and-rollback` and `build-package` are unchanged. Only 2 of 4 tests
passed, so the third stability-confirmation run was not triggered (that was
conditioned on 3 of 4 passing).

**2026-08-19, later still: a committed install now activates a real on-disk
deployment; `nex commit` still fails, one layer deeper, because activation's
own cleanup remounts more than it means to.** The human made the commit
bootable (`a084cf66`): `create_deployment` now calls `deploy::run` on the
new ref (`allow_commit_hash: true`), reusing the same activation path `nex
deploy` uses instead of growing a second one.

Rebuild cascade, same order:

    pkg/core/nex/nex.yaml        df49013f299e5196e73c19f2a2bbdda738c575506f7cf517d19cb292aaf03216
    base/nex-systemd.yaml        5cfeea15437583829a55724dfd05f866f675eb294d0637789665adb25b860471
    tests/nex-test-fixture.yaml  7591db572902f2a934cdd1a0f68b49eb7c2047baeae9c16b2fe34dbb8b0d2e5b

All three strict two-build reproducible. Full suite twice, identical both
times, still 2 of 4:

    FAIL: build-package (guest exit 1)
    PASS: temporary-install
    FAIL: persistent-install (guest exit 1)
    PASS: deploy-and-rollback (verify-deployed)
    tests run: 4
    failures: 2

`persistent-install`'s `nex commit` output, both runs, structurally
identical (the timestamp-derived ref name and content hash naturally differ
run to run):

    Committing changes: install tig for persistent-install test
      Current deployment: nex/deployments/<fixture-checksum>.0
      Checking out current deployment...
      Applying staged changes...
      Creating new commit...
      Created deployment: nex/deployments/<unix-timestamp>
      Activating for next boot...
    System ref:   nex/deployments/<unix-timestamp>
    Checksum:    <new-checksum>
    Next serial: 1
    Target:      /sysroot/nex/deployments/<new-checksum>.1
    Staging:     /sysroot/nex/deployments/.<new-checksum>.1.tmp

    Deployed: <new-checksum>.1
    Reboot to activate (bootloader picks highest serial).
    Error: Os { code: 30, kind: ReadOnlyFilesystem, message: "Read-only file system" }
    commit-exit=1

The activation genuinely works this time -- confirmed directly, not
inferred: a disposable diagnostic added to a copy of the guest script
(reverted after; no product code touched) ran `ls -la
/sysroot/nex/deployments` right after the failure and found both the
original `<fixture-checksum>.0` and the new `<new-checksum>.1`, intact, on
disk. The failure is in what runs immediately after activation.

Root-caused by a direct, unfiltered `/proc/mounts` before/after comparison
around the `nex commit` call (same disposable-diagnostic method). Before:

    /dev/sda2 / ext4 ro,relatime
    /dev/sda2 /sysroot ext4 ro,relatime
    /dev/sda2 /nex/deployments ext4 rw,relatime
    /dev/sda2 /nex/staging ext4 rw,relatime

After (captured right after `deploy::run` printed "Deployed:..." /
"Reboot to activate...", i.e. after its own `RemountGuard` remounted
`/sysroot` back to read-only on the way out):

    /dev/sda2 / ext4 ro,relatime
    /dev/sda2 /sysroot ext4 ro,relatime
    /dev/sda2 /nex/deployments ext4 ro,relatime
    /dev/sda2 /nex/staging ext4 ro,relatime

`/nex/deployments` and `/nex/staging` flip from `rw` to `ro`, even though
`RemountGuard::remount_ro` (`deploy.rs`) names only `/sysroot`
(`mount -o remount,ro /sysroot`) and touches nothing else by name. This
matches exactly what was flagged as worth watching for: `deploy::run`
remounts the sysroot read-write and back, and inside a running system --
where `/sysroot`, `/nex/deployments`, and `/nex/staging` are separate mount
entries for the *same block device*, `/dev/sda2` -- a remount targeted at
one of them changes the underlying filesystem instance's read-only state
for all of them at once. `create_deployment`'s very next line,
`fs::remove_dir_all(&staging_dir)` (`commit.rs:151`, removing
`/nex/staging/commit_staging`), which the human had already confirmed
correct on its own terms (its parent is normally writable), then fails,
because by that point its parent has just been made read-only as an
unintended side effect of cleanup meant only for `/sysroot`.

Because the failure happens after the phase already reported non-zero, the
host-side harness -- by design, stopping at the first failed phase --
does not attempt the reboot. Whether the newly activated deployment would
actually have booted was therefore not tested; forcing a reboot past a
phase the harness is designed to stop at was judged out of scope for an
observation, not a decision to make unilaterally. Not fixed, not routed
around.

On the accumulation question: every run in this harness starts from a
fresh overlay, so cross-run serial accumulation could not be observed --
`Next serial: 1` every time. Within one run, only one `nex commit` call
happens, so no within-run accumulation was observed either. Worth flagging
regardless: because activation completes and commits its result *before*
the failing cleanup, a `nex commit` that ultimately reports failure (exit
1) still leaves a new, real, on-disk deployment behind every time it's
tried. Not fixed, per instruction.

Full detail in `.agents/knowledge/machine-self-hosting.md` ("`deploy::run`'s
remount to read-only affects more than `/sysroot` when it shares a block
device"). `deploy-and-rollback` and `build-package` are unchanged. Still 2
of 4, so the third stability run was not triggered.

**2026-08-19, later still: the ordering fix landed (`9c94ef56`); activation
keeps succeeding, and `nex commit` now fails on a different, downstream
mount error.** `create_deployment` now only writes the ref and returns it;
`cleanup_staging()` runs next; `activate_deployment(&new_ref)` runs last,
with a comment in the source explaining why.

Rebuild cascade, same order:

    pkg/core/nex/nex.yaml        225750ba4ddd31f366f6d4dca64666f06aedc666f248e31020549ebbeeaaee46
    base/nex-systemd.yaml        ab83690c7267a8ecf9f81dcf0b31061d7b73040f9c48939f045f81832d78fe77
    tests/nex-test-fixture.yaml  2a89a4ab2e540c4e4433d80a3069a54e4a33f1de311dfa2836ab7658da2bd949

All three strict two-build reproducible. Full suite twice, identical both
times, still 2 of 4:

    FAIL: build-package (guest exit 1)
    PASS: temporary-install
    FAIL: persistent-install (guest exit 1)
    PASS: deploy-and-rollback (verify-deployed)
    tests run: 4
    failures: 2

`persistent-install`'s `nex commit`, both runs, structurally identical:

    Committing changes: install tig for persistent-install test
      Current deployment: nex/deployments/<fixture-checksum>.0
      Checking out current deployment...
      Applying staged changes...
      Creating new commit...
      Created deployment: nex/deployments/<unix-timestamp>
    umount: /nex/pkg: target is busy.
      Activating for next boot...
    System ref:   nex/deployments/<unix-timestamp>
    Checksum:    <new-checksum>
    Next serial: 1
    Target:      /sysroot/nex/deployments/<new-checksum>.1
    Staging:     /sysroot/nex/deployments/.<new-checksum>.1.tmp

    mount: /sysroot: mount point is busy.
           dmesg(1) may have more information after failed mount system call.
    mount: /sysroot: mount point is busy.
           dmesg(1) may have more information after failed mount system call.
    Error: Custom { kind: Other, error: "failed to remount /sysroot ro" }
    commit-exit=1

The reordering is visible and real: `cleanup_staging`'s unmount attempt
(`umount: /nex/pkg: target is busy.`) now runs *before* "Activating for next
boot...", where it used to run after. Confirmed with a disposable diagnostic
on a copy of the guest script (reverted after; no product code touched)
that activation again genuinely succeeds: `ls -la /sysroot/nex/deployments`
after the failure shows both the original deployment and the new one,
intact. The doubled "mount point is busy" message is `RemountGuard`'s own
shape -- an explicit `remount_ro()` call fails and its error propagates via
`?`, then its `Drop` impl retries on the way out of scope and fails again,
silently. Confirmed directly that the remount genuinely never completed:
`/proc/mounts` right after shows `/dev/sda2 /sysroot ext4 rw,relatime`, and
`touch /sysroot/.diagtest` succeeds -- `/sysroot` is stuck read-write, not a
misleading error message. `/nex/pkg` and `/nex/env`'s overlays are gone from
`/proc/mounts` (the lazy unmount completed); `/nex/deployments` and
`/nex/staging` show `rw`, unaffected this time (unlike the previous
manifestation, where they flipped to `ro`).

Proposed mechanism, kept separate from the observations above and flagged
as unconfirmed by a standalone reproduction (unlike the `glob` and
`rmdir`-parent findings, which were independently verified before being
acted on): the same block-device sharing as before (`/`, `/sysroot`,
`/nex/deployments`, `/nex/staging` are all `/dev/sda2`), now interacting
with timing rather than flag state -- `cleanup_staging`'s lazy unmount of
`/nex/pkg` detaches immediately but the kernel may still be releasing the
underlying mount in the background; if that is still in flight when
`deploy::run`'s `mount -o remount,ro /sysroot` runs moments later, a
remount of the shared device could plausibly be refused as busy until it
settles. Not verified further.

On the accumulation question raised last time: this run's failure mode
(a busy remount, not a cleanup failure) still happens *after* activation
completes, so the same pattern recurred -- a stray deployment left on disk
after a `nex commit` that ultimately failed, confirmed the same way.
Watched for as asked; not fixed.

Full detail in `.agents/knowledge/machine-self-hosting.md`. `deploy-and-rollback`
and `build-package` are unchanged. Still 2 of 4, so no third stability run.

**2026-08-19: the retry landed, activation is proven, and `nex commit` reaches
`PASS` for the first time -- but the new deployment is missing the package it
was told to install.** With the retry-then-warn fix in place, `persistent-install`
reaches `PASS: persistent-install (before-reboot)` for the first time in this
plan. Full `before-reboot` output, both runs identical:

    Committing changes: install tig for persistent-install test
      Current deployment: nex/deployments/6fc6aa8a93a1aadcff76545096c643167337dcdd69023b18962e6fc906e54a3a.0
      Checking out current deployment...
      Applying staged changes...
      Creating new commit...
      Created deployment: nex/deployments/<unix-timestamp>
    umount: /nex/pkg: target is busy.
      Activating for next boot...
    System ref:   nex/deployments/<unix-timestamp>
    Checksum:    <new-checksum>
    Next serial: 1
    Target:      /sysroot/nex/deployments/<new-checksum>.1
    Staging:     /sysroot/nex/deployments/.<new-checksum>.1.tmp

    mount: /sysroot: mount point is busy.
           dmesg(1) may have more information after failed mount system call.
    [x5]
    Warning: /sysroot is still writable; could not restore it to read-only. The deployment is complete. A reboot restores the read-only mount.
    Deployed: <new-checksum>.1
    Reboot to activate (bootloader picks highest serial).
    mount: /sysroot: mount point is busy.
           dmesg(1) may have more information after failed mount system call.
    [x5]
    Warning: /sysroot is still writable; could not restore it to read-only. The deployment is complete. A reboot restores the read-only mount.
    Deployment created successfully.
    commit-exit=0
    PASS: persistent-install (before-reboot)

The warning text appears exactly twice per run (the explicit `remount_ro()`
call, then the `Drop` retry), both runs of the suite, as predicted. The
retry loop does not fully solve the busy remount here -- it still exhausts
all five attempts and falls back to the warning -- but the machine is left
writable rather than the command failing, which is the documented trade-off.

Reboot succeeds and the after-reboot phase confirms activation genuinely
worked: `current-deployment=746563fd97dc42f12e41a3312c39e72eb5d71cb5f04e0215905df02846a3ab4f.1`,
different from the pre-commit `6fc6aa8a93a1aadcff76545096c643167337dcdd69023b18962e6fc906e54a3a.0`
in both runs. But `post-reboot-binary-present=false`, and the guest reports
`error-line=tig not present after reboot`.

Root-caused, not guessed -- confirmed by both a source read and a direct
on-disk check (disposable diagnostic added to a copy of the guest script,
reverted after; no product code touched). `commit.rs`'s `create_deployment`
builds the new deployment's tree by checking out the current deployment into
a scratch dir, then layering the two staged overlays on top:

    let upper_nex = format!("{}/upper/nex", STAGING_STATE_DIR);       // /nex/staging/upper/nex
    let upper_usr_bin = format!("{}/upper/usr_bin", STAGING_STATE_DIR);
    if Path::new(&upper_nex).exists() {
        let nex_target = format!("{}/nex/pkg", staging_dir);
        fs::create_dir_all(&nex_target)?;
        copy_dir_contents(&upper_nex, &nex_target)?;
    }
    if Path::new(&upper_usr_bin).exists() {
        let bin_target = format!("{}/usr/bin", staging_dir);
        fs::create_dir_all(&bin_target)?;
        copy_dir_contents(&upper_usr_bin, &bin_target)?;
    }

`copy_dir_contents(src, dst)` runs `cp -a "$src/." "$dst"` -- it copies
`src`'s *contents* into `dst`, not `src` itself. `upper_nex`
(`/nex/staging/upper/nex`) is the parent of two separate overlay upperdirs,
confirmed earlier via `/proc/mounts` (`upperdir=/nex/staging/upper/nex/pkg`
for the `/nex/pkg` overlay, `upperdir=/nex/staging/upper/nex/env` for the
`/nex/env` overlay), so its direct children are `pkg/` and `env/`. Passing
`upper_nex` itself (rather than `upper_nex/pkg`) as `src` into a `dst` of
`{staging_dir}/nex/pkg` copies those two children *into* `nex/pkg`, landing
tig's files at `nex/pkg/pkg/dev/vcs/tig/...` (one extra `pkg/` level) and
also misplacing `env/`'s content at `nex/pkg/env/...` instead of `nex/env/...`.

Confirmed directly on the booted new deployment, both matching the
prediction:

    diag-nex-pkg-listing=... apps cli core dev env libs pkg ...
    diag-nex-pkg-pkg-listing=... dev ...
    diag-usrbin-tig-readlink=../../nex/pkg/dev/vcs/tig/2.6.0/0cbb5716/usr/bin/tig
    diag-usrbin-tig-stat=stat: cannot statx '/usr/bin/tig': No such file or directory

`/nex/pkg` (the real, booted, top-level FHS path) carries an unexpected
`pkg/` entry alongside the legitimate namespace directories
(`apps`, `cli`, `core`, `dev`, `env`, `libs`), and `dev/vcs/tig/2.6.0/0cbb5716`
lives one level too deep, under that extra `pkg/`. The `/usr/bin/tig` symlink
itself is correctly placed and correctly named (its copy, via `upper_usr_bin`,
is a separate call with matching source/target shapes and has no such bug),
but it points at a path (`/nex/pkg/dev/vcs/tig/...`) where nothing exists, so
it is dangling: `stat -L` reports "No such file or directory" and
`command -v` correctly reports it absent. This fully explains the failure;
no further hypothesis needed. Not fixed (product code, `commit.rs`, not
touched).

`deploy-and-rollback` and `build-package` unchanged. Still 2 of 4, so no
third stability run.

**2026-08-19: `persistent-install` passes end to end, tree shape confirmed
correct, three runs in a row.** With the copy-nesting fix landed in two
places (`dc561b55`: `create_deployment` and `merge_overlay_changes`), ran the
full suite three times (the third triggered because the first two both
landed 3 of 4). Identical every time:

    FAIL: build-package (guest exit 1)
    PASS: temporary-install
    PASS: persistent-install (before-reboot)
    PASS: persistent-install
    PASS: deploy-and-rollback
    tests run: 4
    failures: 1

`persistent-install`'s after-reboot phase now reports
`post-reboot-binary-present=true` and runs `tig --version` successfully
(`tig version 2.6.0`, `post-reboot-run-exit=0`).

Per instruction, checked the tree shape directly rather than trusting the
presence check alone, using a disposable diagnostic on a copy of the guest
script (reverted after; no product code touched):

    diag-nex-pkg-has-stray-pkg=false
    diag-nex-env-is-sibling=true
    diag-nex-pkg-env-should-not-exist=false
    diag-usrbin-tig-readlink=../../nex/pkg/dev/vcs/tig/2.6.0/0cbb5716/usr/bin/tig
    diag-usrbin-tig-resolves=true

All three of the requested checks hold: no `/nex/pkg/pkg`, `/nex/env` exists
as a sibling of `/nex/pkg` rather than nested under it, and `/usr/bin/tig`'s
symlink resolves rather than dangling. The fix is structurally correct, not
just accidentally passing the presence test.

The `/sysroot is still writable` warning still appeared every run, twice per
`nex commit` call, matching the previous unit exactly -- the retry still
does not settle the busy remount here, but per the coordinator this is now
expected and not a failure.

`build-package` unchanged across all three runs: same error text each time,
`glibc`'s split `/usr/lib/gconv/*` outputs (e.g. `libCNS.so`, `libGB.so`,
`libISOIR165.so`, `libJIS.so`) missing from the guest store, because the
harness only seeds `tig`'s runtime closure, not the build-time dependency
closure gzip's own build needs. Real, expected, understood limitation, left
as reported, not routed around.

This closes the `persistent-install` line of investigation: three of four
tests now pass deterministically. `build-package` is the one remaining,
understood failure -- a machine that cannot fetch cannot build an arbitrary
package from a closure the harness never seeded for it.

**2026-08-19: seeding gzip's build closure is correct and sufficient, but
does not make `build-package` pass -- the guest's plain `nex build` never
looks at the store the harness seeds.** The human corrected the previous
call that gzip's build closure was an inherent limitation not worth
seeding: seeding it is legitimate host-side setup, no different from
seeding tig's runtime closure, and does not weaken what the test proves.

Extended `seed_system_repo()` with `seed_gzip_build_closure()`, which reads
gzip's manifest's own `dependencies:` list (15 commits, not 14 -- an
early hand-count was off by one, corrected once the extraction was
scripted rather than eyeballed) and pulls each exactly as declared, then
discovers and pulls each dependency's own `/files` closure using the same
resolver `nex build`'s chroot hydration path uses
(`resolve_runtime_deps_precomputed`, via `nex resolve`).

Getting there required ruling out `nex resolve <name>` as a safe lookup
first, and this was checked by hand before trusting it, not assumed: it
matches by name, and this repository has real, confirmed collisions.
`binutils`, `gcc`, `bash`, `coreutils`, `findutils`, `gawk`, `grep`,
`make`, `sed`, `tar`, and `xz` all exist twice -- once under
`bootstrap/phase1/*` (what gzip's manifest actually names) and once as
the self-hosted `core/*`/`cli/*` package of the same name and version --
and a plain single-word query silently picked the *wrong* one for every
single one of those, confirmed by running all 15 and comparing each
"Resolving:" line against the manifest's own declared ref. A full
`namespace/slug` query (no version) disambiguates those correctly, but
not `glibc`, which collides the opposite way: `libs/system/glibc` is a
literal prefix of the unrelated `libs/system/glibc-locale-en-gb`, and the
substring-matching query form picks that instead; a single-word query
(exact equality on slug there) resolves `glibc` correctly. The function
tries both forms per dependency and verifies the resolved ref's
namespace/slug/version against the manifest's own declared ref before
trusting it, refusing to seed (loud failure, not a silent wrong or
partial closure) if neither form matches -- self-verifying at every run,
not something that could quietly drift.

Also checked, not assumed: for the two dependencies where both `bundles/dev`
(what gzip wants) and `bundles/full` exist (`glibc`, `texinfo`), reading
each manifest's `bundles:` section directly confirmed `full` is either
identical in output categories to `dev` (`glibc`) or a strict superset
(`texinfo`, which only adds an `info` category), so resolving via `full`
(which `nex resolve`'s priority order always prefers when both exist)
cannot miss anything `dev`'s own closure would have required.

Verified the seeding was compact, not "impractically large" -- the other
condition to stop and ask about: a standalone run seeded the 15 dependency
roots plus exactly one discovered `/files` closure ref (glibc's own,
matching the checksum this exact investigation started from four units
ago), about 815MB and 30202 objects, `zub fsck`-healthy, in about 70
seconds.

Rebuilt the cascade (unchanged from the previous unit -- nothing in
`src/cli` changed this unit) and ran the full suite three times: identical
every time, still 3 of 4. `build-package` failed with the *exact same*
error text and the *exact same* checksum
(`x86_64/pkg/libs/system/glibc/2.39/bf348eabcec257edace3e1e05458bf79ddad1a5164f25e706b7e50d93b25190d/files`)
as before the seeding was added, all three runs.

Root-caused why, confirmed by direct reproduction rather than left as a
guess: `nex build`, run as root without `--system`, uses `detect_context`'s
*user* context (`src/cli/src/repo.rs`), whose primary repo is
`/nex/users/root/repo` -- empty on a fresh machine -- not `/nex/repo`,
the system store everything gets seeded into. `detect_context` does add
`/nex/repo` to that context's `fallback_repos` when it exists (confirmed
present on the guest: `diag-nex-repo-exists=true`, and the exact glibc
`/files` ref confirmed sitting at
`/nex/repo/refs/heads/x86_64/pkg/libs/system/glibc/2.39/.../files` via a
disposable diagnostic, both checked directly on the booted guest, not
assumed) -- but `BuildOpts.fallback_repos`
(`src/cli/src/commands/build.rs`) is populated only from the explicit
`--fallback-repo` CLI flag, never from `NexContext.fallback_repos`. The
auto-detected fallback the context computes is therefore never threaded
into `materialize_build_dependencies`'s `MaterializeConfig`, and the
resolver's self-referencing-need check
(`src/cli/src/materializer/resolver.rs`, `queue_self_file_dependency`,
`self.store.resolve_ref`) only ever looks in the empty per-user repo.

Proved this precisely, not just reasoned about it: added a disposable
diagnostic to a copy of `build-package.sh` (reverted after; no product
code touched) that ran `nex build pkg/cli/archive/gzip.yaml --single
--repo /nex/repo` explicitly. With the repo forced, dependency resolution
found the full 15-commit closure including glibc's `/files` ref,
"Materialization complete", fetched gzip's source over the network,
started the build sandbox -- proof the seeding is complete and correct.
It then hit a second, different, unrelated failure: `unshare: unshare
failed: Invalid argument`, from the build script's `unshare --user --pid
--mount --uts --fork --ipc --net --map-root-user` sandbox setup
(`src/cli/src/build/script.rs`). Not investigated further this unit --
flagged as a separate, second blocker, observed but not chased.

This is the condition the human named in advance: resolving it needs a
design decision, not more seeding. No amount of host-side seeding can
close this gap, because the guest's plain `nex build
pkg/cli/archive/gzip.yaml --single` -- no flags, exactly what a person
sitting at the machine would type, matching this test's own stated
purpose -- never consults `/nex/repo` for build-dependency resolution at
all on the path this harness exercises. Closing it means either product
code threading `NexContext.fallback_repos` into
`BuildOpts.fallback_repos` so a build's dependency resolution sees the
same fallback its own context already computes, or a decision that this
test should invoke `nex build` with `--system` or `--repo /nex/repo`
rather than the bare command -- and if the latter, that changes what the
test is proving. Stopped per instruction rather than seeding around it or
changing the guest script's invocation unilaterally.

## Decision Log

- Decision: The harness pre-populates the guest's system store (`/nex/repo`)
  with refs pulled from the host's own build store before boot: `tig`
  (`outputs/bin`, small, already built, not part of this fixture's own
  package list) for tests 2-3, and `systems/nex-systemd/0.0.1` (a
  different already-built system) for test 4's deploy target.
  Rationale: Every test here must avoid the environment bug, which only
  triggers on a build. The guest still runs every `stage`/`install`/
  `discard`/`commit`/`deploy`/`rollback` command itself; only the ingredient
  each command needs already built is placed there first, the same way the
  fixture's own installed packages are host-built before boot. `zub pull`
  hardlinks content-addressed objects, so seeding both refs costs about 425 MB
  and well under two seconds.
  Date/Author: 2026-08-19 / Claude

- Decision: Tests that cross a reboot (3 and 4) are one guest script file
  taking a phase argument, driven by a per-test phase list
  (`TEST_PHASES` in `scripts/test-machine-operations.sh`) that the host
  steps through, calling the `reboot` verb between phases and stopping the
  test at the first failed phase without attempting the phases or reboots
  after it.
  Rationale: Keeps "one script per test" from the Plan of Work while
  giving the host what it needs to own the reboot, per the harness design.
  State a later phase needs (e.g. "is the installed binary still there")
  comes from the guest's own persistent files (or, for test 4, its
  `/proc/cmdline`), never from a value the host computed and handed back in.
  Date/Author: 2026-08-19 / Claude

- Decision: Drive every operation from inside the guest over SSH, never from the
  host.
  Rationale: The host staging a deployment is the operation under test. A test
  that performs it on the guest's behalf proves only that the harness works.
  Date/Author: 2026-08-19 / Claude

- Decision: Give each test its own qcow2 overlay over one shared backing
  image.
  Rationale: Every test then starts from an identical known deployment, a
  failure names one operation rather than a sequence, and discarding state is
  deleting a file. The existing script builds guest state inline, which couples
  tests together.
  Date/Author: 2026-08-19 / Claude

- Decision: Tests emit `key=value` lines on stdout and exit non-zero on
  failure. The host parses no prose.
  Rationale: This is what made the graphical smoke test legible.
  `center-pixel=srgb(240,0,255)` is checkable; "looks right" is not.
  Date/Author: 2026-08-19 / Claude

- Decision: The fixture must be a `nex_structure: true` assembly.
  Rationale: A flat assembly has no `/nex` at all. Checked on 2026-08-19: a
  built `edgebox-rootfs` root contains no `nex` directory and no `nex` binary,
  so it cannot build, install, deploy, or roll back anything.
  Date/Author: 2026-08-19 / Claude

- Decision: The fixture ships a Git bundle with full history and clones from it
  on first boot, rather than shipping a file tree.
  Rationale: A machine always needs full history so every pinned document is
  reachable; this is a design requirement, not a measurement. Shipping a tree
  and running `git init` over it cannot satisfy it. This anticipates phase C in
  one test assembly, which also proves the mechanism before phase C generalises
  it.
  Date/Author: 2026-08-19 / Human

- Superseded: Write `tests/nex-test-fixture.yaml`, extending
  `nex:base/nex-systemd.yaml` and adding a `git` package plus a populated
  `/usr/share/nex/manifests`.
  Rationale: Measured, not assumed. `nex-systemd` alone boots with an empty
  `/nex/manifests` and no git, so no test that touches manifests can run.
  Only desktop-vwl ships those today, and it costs roughly 7 GB per build.
  Date/Author: 2026-08-19 / Claude

- Superseded: Use `base/nex-systemd.yaml` as the fixture and inject SSH access
  into the var tree, rather than writing a fixture assembly.
  Rationale: The access an assembly may not carry is exactly the access that
  belongs in machine state, and `scripts/prepare-qemu-test-identity.sh` already
  writes it there. A fixture assembly would duplicate that helper and would
  have to be exempted from a policy it should never have tripped.
  Date/Author: 2026-08-19 / Claude

- Superseded: If `base/nex-systemd.yaml` will not serve, write a purpose-built
  `tests/nex-test-fixture.yaml` rather than using `examples/desktop-vwl`.
  Rationale: desktop-vwl builds a roughly 7 GB root carrying a whole desktop
  stack that none of these tests exercise, and every fixture rebuild would
  pay for it. A purpose-built fixture is small, fast, and its contents are
  chosen by what the tests need. It lives in `tests/` rather than `base/` or
  `examples/`, because a fixture is run directly, which `base/` is not for, and
  because it needs a known account and a fixed key, which
  `scripts/check-generic-assembly-policy.sh:29` rejects in anything it scans.
  Date/Author: 2026-08-19 / Human

- Decision: Assert on state the machine reports about itself, never on a value
  the test supplied.
  Rationale: Checking that a deployment directory holds the checksum the test
  just wrote there proves nothing about booting.
  Date/Author: 2026-08-19 / Claude

- Decision: Generate bundles with `git -c pack.threads=1 bundle create`, from
  an explicit commit reached through a detached worktree.
  Rationale: Measured 2026-08-19. Three default runs produced three different
  sha256 values; three with `pack.threads=1` produced one, at the same size, so
  the nondeterminism is parallel work-splitting rather than content. Adding
  `pack.window=0 pack.depth=0` is also deterministic but grows 7.4 MB to
  24.9 MB. `git bundle create <file> <commit-sha>` refuses with "empty bundle"
  because it needs a named ref, and bundling a branch yields a clone with an
  empty tree, so a detached worktree supplies a HEAD to bundle. Carried from
  the deleted ExecPlan 018 and since confirmed in `scripts/make-test-bundle.sh`.
  Date/Author: 2026-08-19 / Claude

- Decision: Record each bundle's sha256 in the manifest, as `cargo_lock` does.
  Rationale: It makes the snapshot a verified input rather than whatever the
  build host produced, and it restores pinning, which `dev:` blocks
  (`link.rs:284`). Carried from the deleted ExecPlan 018.
  Date/Author: 2026-08-19 / Claude

- Decision: Rename `/nex/repo` to `/nex/store` and `--repo` to `--store`.
  Rationale: Two different things were called "repo", a zub object store and a
  Git repository, and the confusion cost real time on 2026-08-19. The code
  already prefers the other word: `main.rs:18` declares `pub mod store` and the
  dependency at `Cargo.toml:27` is the `zub-store` crate. Carried from the
  deleted ExecPlan 018.
  Date/Author: 2026-08-19 / Human

- Decision: Fix the product defects here rather than deferring them, and merge
  ExecPlan 018 into this one.
  Rationale: This plan's acceptance is that all four tests pass, and three
  cannot pass while the defects stand. A plan that forbids itself from reaching
  its own acceptance is malformed. Merging also removes an ordering trap: the
  bundle alone does not make a machine able to build, because environment
  resolution reads the wrong directory.
  Date/Author: 2026-08-19 / Human

## Outcomes & Retrospective

Entries are in the order they happened, and each was true when written. The
suite grew from four tests to five and most of what the early entries report
as broken was later fixed, so read the last entry, dated 2026-08-20, for the
finished state.

`scripts/test-machine-operations.sh` exists and runs all four tests, `--test
<name>` runs one alone, and every test starts from a fresh qcow2 overlay
over one shared backing image built from `tests/nex-test-fixture.yaml`
(`systems/nex-test-fixture/0.0.1`, checksum
`69c012db66f612dc7074ab29d8b076876382a33e8cbda20396d410710d5fb487`, strict
two-build reproducible, unchanged since it extends `base/nex-systemd.yaml`
which now carries the module directly). Verified twice in a row, at three
points in this plan's history as the OverlayFS finding moved from unfixed, to
fixture-only, to fixed in the base assembly: `build-package`,
`temporary-install`, and `persistent-install` FAIL, identically each time;
`deploy-and-rollback` PASSes, identically each time, including two reboots.
The deliberately-broken-command check (a typo'd deploy ref) made exactly one
test fail, carrying the guest's own error text, while the other three kept
their unrelated pre-existing failures.

Three of four tests FAIL, and every one of those three is a legitimate,
reproducible finding this plan set out to get, not a harness defect:

1. `build-package`: a machine cannot build a package. Full history in
   `/nex/manifests` (shipped via Git bundle, not a `git init` snapshot) makes
   the manifests worktree and historical blob lookups work, but
   `load_environment` resolves environment blobs against the zub store, which
   is never inside a Git repository on an installed machine. Fixed in phase B.
2. `temporary-install` and `persistent-install`: a machine could not stage a
   package for install. `nex stage` unconditionally mounts OverlayFS, and
   `base/nex-systemd.yaml`'s kernel bundle did not carry `overlay.ko`.
   `deploy-and-rollback` proved this was specific to staging, not to the
   fixture generally: it needs no OverlayFS and passed cleanly throughout.
   The human first added `kernel-fs-overlay` to the test fixture only, then
   decided it belonged in `base/nex-systemd.yaml` itself ("nobody should be
   running a plain nex-systemd machine that cannot install a package") and
   moved it there; `nex-systemd` checksum
   `a271d6d1246076e032c99b3a8d2c060baff9004e428c6ae1f1fb9ddb258d31de`. `nex
   stage` now succeeds either way. Both install tests now fail one step
   later instead: `nex install` requires every runtime dependency of the
   package being installed to have its own `/files` commit already
   resolvable in the store, which the harness had not seeded (only the
   requested package's own ref was seeded). A different, now-precisely-located
   gap; see the 2026-08-19 Surprises & Discoveries entries after the
   OverlayFS one.

Rebuilding the cascade after the `base/nex-systemd.yaml` change also broke
`examples/desktop-vwl/desktop-vwl.yaml` (and `examples/desktop-dev.yaml`,
which extends it), briefly: `desktop-vwl.yaml` selects the kernel's
`bundles/all-modules`, which already carries `fs-overlay`, and now also
inherited the base's new `kernel-fs-overlay` package with nothing excluding
it, so two packages tried to deliver the same `overlay.ko`. I misdiagnosed
this as a `zub` bug before the human corrected it by testing the actual
manifest change first; the human's fix (an `exclude` in `desktop-vwl.yaml`)
resolved it. See the corrected Surprises & Discoveries entry and
`.agents/knowledge/kernel-and-boot.md`.

So the plan's four-test premise holds up: a person sitting at an installed
Nex machine can put a new system version on it and undo that (test 4,
proven, content-based, across two real reboots), but cannot build a package or
finish staging an install today, for three distinct, now-precisely-located
reasons (one for building, two in sequence for staging) none of which was
known before this plan.

What was not done: none of the three underlying issues were fixed by this
plan's tests, per instruction. The store/environment lookup bug is phase B's
job. Whether `base/nex-systemd.yaml` itself should carry `overlay.ko` was an
open design question, reported not decided, and the human answered it by
adding the module to the test fixture only, not to `base/nex-systemd.yaml`.
The `nex install` dependency-closure gap surfaced last and was reported, not
routed around: no dependency refs were seeded to make it pass. `reboot` and
`expect_deployment`-equivalent logic are now exercised (test 4), but
`expect_deployment` itself as a literal harness function is unused by any
test; tests use `/proc/cmdline` parsing inline instead since the
assertions needed were about content markers, not a single deployment-name
equality check. No stray QEMU processes or Git worktrees were left behind by
any run.

**Update, 2026-08-19, later the same day: the dependency-closure gap was a
harness bug and is fixed; a fourth issue (`uid 0 not mapped in namespace`)
replaced it and also broke the one test that had passed every prior run.**
The human fixed environment resolution (commit `28218662`) and I fixed
`seed_system_repo()` to seed `tig`'s full runtime closure via `nex resolve
tig -v`, not just its own ref. After rebuilding `pkg/core/nex/nex.yaml`,
`base/nex-systemd.yaml`, and `tests/nex-test-fixture.yaml` so the guest
carries the fixed CLI: `build-package` gets past environment resolution
(progress) but still fails, now on gzip's own build-sandbox dependency
closure, which was never in scope to seed. `temporary-install` and
`persistent-install` get past `nex install`'s dependency resolution (also
progress: `Runtime closure: 6 commit(s)`) but fail during the real checkout.
`deploy-and-rollback`, which had passed every single run before this unit,
now fails too. All three failures are the identical, deterministic error:
`uid 0 not mapped in namespace`, traced as far as a zub commit-time
uid-mapping check reachable from a hardlinked checkout's repair path, against
the harness's `uid_map = []` config (unchanged this unit, copied from
`qemu-test-live-upgrade.sh`). Why it fires now and did not before is not
established. Reported, not fixed or routed around; see the corresponding
Surprises & Discoveries entry and `.agents/knowledge/machine-self-hosting.md`.

**Update, 2026-08-19, later still: the human root-caused and fixed the
uid-mapping bug; `deploy-and-rollback` is back to passing, and the install
tests reached one blocker further before hitting a new, real, and clearly
located one.** `write_zub_config()` now writes an explicit identity uid/gid
map instead of an empty one (the guest is real root, so identity is
correct). Reran the suite twice, identical both times:
`build-package`/`temporary-install`/`persistent-install` FAIL,
`deploy-and-rollback` PASSes (including two real reboots, both runs).
`build-package` fails exactly as expected on gzip's own build-time
dependency closure, not seeded, a genuine limitation. The two install tests
now get through the checkout that used to fail and reach
`mount_nex_overlays()`, which fails creating `/nex/env` — nothing in
`base/nex-systemd.yaml` ever creates that directory, and `/nex` is on the
read-only deployment root, so it can never be created after the fact either.
Neither test reaches `nex commit`; that question is still open. Full detail
in the corresponding Surprises & Discoveries entry and
`.agents/knowledge/machine-self-hosting.md`.

**Update, 2026-08-19, later still: `/nex/env` fixed; `nex install` completes
end to end for the first time in this plan; the `nex commit` question is
finally answered.** `base/nex-systemd.yaml` and `tests/nex-test-fixture.yaml`
rebuilt (checksums `1ae2b42c1a0e5f24f6bf1473f0eb7089340151479c733dcf731ca6c7a2230860`
and `7a902425d6315f6d66930b5a26fe4b41219baa3729a6e6cae9c0657cf691c034`). Full
suite twice, identical both times: still 3 FAIL / 1 PASS, but not the same
three reasons as before. `temporary-install` and `persistent-install` both
now get all the way through `nex stage` and `nex install` -- checkout,
symlink into `/usr/bin`, the installed `tig`/`git` binaries actually run.
That is a real milestone: every fix landed across this plan (full manifest
history, seeded dependency closure, uid mapping, `/nex/env`) was necessary
and together sufficient to make a real `nex install` work. `temporary-install`
then fails at `nex discard` (`/nex/pkg` unmount reports busy, then removing
`/nex/staging` hits `Read-only file system`) -- new, not yet root-caused.
`persistent-install` reaches `nex commit` and confirms, at last, what this
plan flagged as a hypothesis units ago: no machine built by anything in this
repository today has a `nex/deployments/*` or `nex/base` ref in its store,
so `nex commit`'s store-ref path can never find a "current deployment" to
commit on top of. `deploy-and-rollback` keeps passing; `build-package`
fails exactly as before, on gzip's build-time dependency closure, a genuine
limitation left alone. Full detail in the corresponding Surprises &
Discoveries entry and `.agents/knowledge/machine-self-hosting.md`.

**What this plan got wrong, twice.** Two fixture decisions were recorded and
then overturned by measurement. First that `examples/edgebox-rootfs.yaml` could
serve, when a built edgebox root has no `/nex` at all. Then that
`base/nex-systemd.yaml` could serve unaided, when it ships neither a manifest
tree nor `git`. Both were caught by running something rather than by reasoning
about it, which is the right order, but writing the plan around an unverified
assumption cost two cycles. A plan that names a fixture should name the
property the fixture must have, and check it first.

**The fixture's bundle is generated, never committed.** `scripts/make-test-bundle.sh`
writes `tests/nex-test-fixture.bundle`, and `.gitignore` excludes
`/tests/*.bundle`. Committing it would put a 7.7 MB copy of this repository
inside this repository, adding its whole size to history on every
regeneration, which is the mistake the librsvg vendor tarball made. Anyone
building the fixture runs that script first. It is reproducible: two runs
produced sha256 `fc268d2210446e95…` both times.

**Update, 2026-08-20: the plan's own acceptance is met. Five tests, green
twice in a row, and every product defect they found is fixed.** The suite is
`build-package`, `temporary-install`, `persistent-install`,
`deploy-and-rollback` and `store-upgrade`, all passing from a fresh overlay,
twice consecutively, `failures: 0` both times. Each also passes run alone
with `--test`, in reverse order, which is what "in any order" comes to here:
every test discards its overlay and creates a new one from the fixture, so no
test can leave state for another. `build-package` reports
`MAKEFLAGS: -j2` (the guest's own CPU count, so the build really ran there),
804 commits in `/nex/manifests`, and `git cat-file -t` answering `blob` for
the pinned environment object -- that last check now fails the test rather
than merely printing a fact, since 476 manifests name their environment by
that blob and a shallow clone would strand all of them.

Twelve defects were found and fixed over this plan, and not one was visible
from reading the code. Every one needed a real machine to boot: environment
blobs resolved against the store rather than the manifests repository;
`overlay.ko` missing from the kernel bundle a machine ships; `/nex/env` never
created, so `nex stage` failed on a read-only root; `nex deploy` never
publishing the store ref `nex commit` looks for; a bare glob prefix matching
only the literal string; a swallowed unmount failure that left cleanup
deleting through a live overlay; `remove_dir_all` on a mount point whose
parent is read-only; `nex commit` writing a ref that was never made bootable;
activation ordered before a cleanup that needs a writable filesystem; staged
overlay contents copied one level too deep; `BuildOpts` never receiving the
context's fallback stores; and `CONFIG_IPC_NS` silently dropped for want of
`CONFIG_SYSVIPC`.

The last of the twelve arrived after the tests were already green, and it is
the one worth remembering. Renaming `/nex/repo` to `/nex/store` looked done:
the CLI used the new name, every test passed, and nothing complained. It was
not done. `pkg/core/kernel/initramfs-init.sh` still bound the store at the
old path, so `/nex/store` was an empty directory on every machine and all
five tests were passing through the legacy fallback -- the code path meant
for old machines was the only one anyone had ever exercised. A green suite
said nothing about it, because the tests had been built on machines the
harness itself set up under the old name. The check that caught it was
reading the acceptance list literally: `/nex/store` was supposed to exist on
an upgraded machine, and it did not exist anywhere. `store-upgrade` now
proves both directions, and it proves them by content: it renames
`/var/nex/store` back to `/var/nex/repo`, reboots, and checks that
`/nex/store` is served from the legacy directory (read from
`/proc/self/mountinfo`, field 4) with 34311 objects, 24 refs and the seeded
ref's commit id all unchanged, then has `nex deploy` resolve a ref through
it.

The deliberately-broken-command check was rerun against the finished suite,
which is the only run where it proves anything: with four tests failing for
their own reasons, "exactly one test fails" is not a claim about isolation.
A typo'd deploy ref made `deploy-and-rollback` fail alone, reporting
`deploy-exit=1` and the guest's own line `ref not found:
systems/nex-systemd/0.0.1-typo-deliberately-broken`, while the other four
stayed green -- including `store-upgrade`, which resolves the same system ref
for its own probe and was unaffected.

`nex link examples/desktop-vwl/desktop-vwl.yaml` succeeds, linking 163
references, where the six `dev:` sources made it refuse before. The built
image carries `/usr/share/nex/nex.bundle` at 7.5 MB and nothing else under
`/usr/share/nex`, with zero `vendor/zub-store` paths anywhere in the tree.
`cargo test` in `src/cli` passes 205 tests.

## Context and Orientation

Terms used here, defined once:

- **store**: the content-addressed object store at `/nex/repo` on a machine, a
  zub repository. Note that the CLI flag is `--repo`, which collides with the
  Git sense of the word. Phase D renames it.
- **deployment**: a system root at `/nex/deployments/<checksum>.<n>`, with
  `/nex/current` symlinked at the active one.
- **manifests repository**: `/nex/manifests`, a Git repository the machine can
  edit, plus a per-user `git worktree` of it at `/nex/users/<user>/manifests`
  created by `repo.rs:261`.

Files this plan touches:

- `scripts/qemu-test-live-upgrade.sh`, read for reusable parts, not extended.
- `scripts/test-machine-operations.sh`, new, the entry point.
- `scripts/machine-tests/*.sh`, new, one per test, run on the guest.

The CLI verbs a user has: `build`, `stage`, `install`, `remove`, `discard`,
`commit`, `switch`, `deploy`, `upgrade`, `deployments`, `status`, `rollback`,
`gc`.

## Plan of Work

Five phases, in this order. The order matters: each phase turns a failing test
green, and a later phase would be unverifiable without the earlier one.

**A. Harness and tests.** Done. It establishes measurement, so every later
phase is checked rather than argued.

**B. Fix environment resolution.** The fixture already ships full history, so
this single fix should make `build-package` pass on its own. Doing it before
anything else proves it in isolation, against a machine whose manifests are
known good.

**C. Generalise the bundle.** Add the `git_bundle` source kind, move desktop-vwl
onto it, rewrite `nex-init-manifests` to clone rather than `git init`, and
delete `.nex-dev-prepare`. The fixture's override of `nex-init-manifests` then
becomes redundant and is removed, which is itself the proof that the general
mechanism works.

**D. Rename the store.** Mechanical, but it touches on-disk layout, so it needs
a migration and it comes after the behaviour changes rather than tangled with
them.

**E. Rebuild and prove.** Rebuild every assembly, record checksums, run the
full suite green.

## Concrete Steps

1. Resolve environment blobs against the repository that owns the manifest
   rather than against the store. `build/package.rs:64` and
   `system/build.rs:98` both pass `opts.repo_path`; `env.rs:49` runs
   `git -C <path> cat-file blob`. `ManifestRepositories` already knows the
   owning root. Add a test that fails when the lookup uses the store.
2. Run `scripts/test-machine-operations.sh --test build-package`. It must pass.
   If it does not, report why before continuing.
3. Add `git_bundle: <commit>` to `Source` in `manifest/types.rs` and a
   `fetch_git_bundle` beside `fetch_cargo_lock` in `outputs/sources.rs`,
   generating with `pack.threads=1` from a detached worktree and verifying a
   recorded sha256. `scripts/make-test-bundle.sh` is the working recipe.
4. Prove the generated bundle is byte-identical across two strict builds.
5. In `examples/desktop-vwl/desktop-vwl.yaml`, replace the six `dev:` sources
   with one `git_bundle` source installed at `/usr/share/nex/nex.bundle`.
   Delete the extraction loop and the `sed -i '/checksum:/d'` that existed only
   because a working-tree tarball could change between the two `--check`
   passes. Confirm the image no longer carries `vendor/zub-store`, which is
   3.8 GB of another repository's build output.
6. Rewrite `/usr/local/bin/nex-init-manifests` in `base/nex-systemd.yaml` to
   `git clone /usr/share/nex/nex.bundle /nex/manifests`, keeping the guard that
   exits when `/nex/manifests/.git` exists. Then delete the fixture's override
   and confirm the tests still pass.
7. Delete `src/cli/.nex-dev-prepare`. Keep the `dev:` source kind, which stays
   the deliberate unpinnable escape hatch for local iteration.
8. Rename `/nex/repo` to `/nex/store` and `--repo` to `--store` across the 29
   references, with a migration that is safe to run twice.
9. Rebuild every assembly with `--single --check --update-checksum`, record the
   checksums here, and run the full suite.

## Validation and Acceptance

The plan is done when:

- `scripts/test-machine-operations.sh` prints `PASS:` for all five tests, from
  a fresh overlay, in any order, and twice in a row.
- A deliberately broken guest command makes exactly one test fail, and the
  failure names the operation and carries the guest's error line.
- `deploy-and-rollback` proves the reboot by content, not by a string the test
  supplied.
- On a booted machine, `git -C /nex/manifests log --oneline | wc -l` exceeds 1
  and `git -C /nex/manifests cat-file -t 27b6e5dc...` prints `blob`.
- The shipped image carries `/usr/share/nex/nex.bundle` and no
  `/usr/share/nex/manifests` tree, and no `vendor/zub-store`.
- `nex link examples/desktop-vwl/desktop-vwl.yaml` succeeds, where the `dev:`
  sources make it refuse today.
- `/nex/store` exists on a machine upgraded from an older deployment, with its
  objects intact.
- Two strict builds of every assembly agree.

## Idempotence and Recovery

Tests are safe to rerun: each starts by discarding its overlay and creating a
new one from the fixture. The fixture is rebuilt only when its assembly
checksum changes. If a guest hangs, the harness kills QEMU on timeout and keeps
the serial log under the artifact directory. Nothing writes to the host store.

## Artifacts and Notes

Keep per-run artifacts under `.nex/tmp/machine-tests/<test>/`: serial log,
guest stdout, and the guest journal on failure. Do not add them to Git.

Note for whoever runs this: `.nex-dev-prepare` uses `mktemp -d`, and `TMPDIR`
defaults to `/tmp`, which is tmpfs on this host. Set
`TMPDIR=<repo>/.nex/tmp/mktemp` for any build step, or the run dies with
`Disk quota exceeded`.

## Interfaces and Dependencies

This plan adds a test interface and changes two machine interfaces.

Test interface: `scripts/test-machine-operations.sh` with `--test <name>`, the
test scripts under `scripts/machine-tests/`, the fixture
`tests/nex-test-fixture.yaml`, and `scripts/make-test-bundle.sh`, which
generates the fixture's bundle. That bundle is gitignored and never committed:
a bundle of this repository stored inside it would add its whole size to
history on every regeneration.

New manifest interface: the `git_bundle` source kind, usable by any manifest.

Changed machine interfaces: `/nex/store` replaces `/nex/repo` as the one path
a machine's content store is reachable at, `--store` replaces `--repo`, and
`/usr/share/nex/nex.bundle` replaces the `/usr/share/nex/manifests` tree. The
rename reaches the boot path: `store_bind_source()` in
`pkg/core/kernel/initramfs-init.sh` binds whichever directory actually holds a
store -- new name or old -- onto `/nex/store`, so a machine installed before
the rename keeps its objects.

Depends on QEMU, `ssh`, `ssh-keygen`, a built fixture, and the zub binary named
by `ZUB_BIN`. ExecPlan 019, splitting the CLI into its own repository, is
paused and does not depend on this plan, though it is easier afterwards.

The question this plan left open -- whether `base/nex-systemd.yaml` should
carry the kernel's `fs-overlay` output so that a shipped Nex system can stage
an install -- was answered during the work: the human asked for the minimum
that makes staging work, so `nex-systemd` now lists `kernel-fs-overlay`
(`base/nex-systemd.yaml:86`) and the fixture no longer has to add it. Any
machine built from `nex-systemd` can stage. See
`.agents/knowledge/kernel-and-boot.md`.
