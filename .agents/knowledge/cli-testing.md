# CLI Testing

## Usual CLI Checks

For CLI changes, run:

```bash
rustfmt --check --edition 2021 <changed rust files>
cargo build --manifest-path src/cli/Cargo.toml
cargo test --manifest-path src/cli/Cargo.toml
```

## Current Sandbox Gaps

This sandbox currently lacks an `ostree` executable. The integration test
`test_time_travel_history_search` shells out to `ostree`; the test now skips
when `ostree --version` cannot start and still runs when the command exists.

Some hosts cannot map uid 0 in a user namespace. The
`rebuilds_when_dependency_manifest_changes` unit test now skips only when store
setup or fixture commits return `uid 0 not mapped in namespace`; it still fails
on other store errors.

Store-backed unit tests can also pass `Store::init` and then fail later at
`commit_tree` with `uid 0 not mapped in namespace`. Tests that depend on
temporary zub commits should treat either step as the same host-capability
skip, and should not hide other store errors.

Evidence: during EP005,
`materializer::flatten::flatten_export_tests::flatten_library_exports_chained_relative_symlink_targets`
and
`flatten_library_rejects_relative_symlink_target_that_escapes_package` skipped
at the `commit_tree` step on this host, while the full CLI suite still passed.

Useful narrowed checks from the 2026-06-28 assembly work remain available when
debugging a host-specific failure:

```bash
cargo test --manifest-path src/cli/Cargo.toml --bin nex -- --skip build::orchestration::tests::rebuilds_when_dependency_manifest_changes
cargo test --manifest-path src/cli/Cargo.toml --test blob_ref_tests -- --skip test_time_travel_history_search
```

Evidence: during 002f, `cargo test --manifest-path src/cli/Cargo.toml` failed
first with `uid 0 not mapped in namespace`, then failed in
`test_time_travel_history_search` because `ostree` was absent. After the
targeted skips landed, the full CLI test suite passed in this sandbox.

## Formatting Dirty Rust Files

When the whole CLI tree has unrelated committed formatting drift, check only
the changed Rust files with rustfmt and keep child modules out of the check:

```bash
rustfmt --edition 2021 --config skip_children=true --check <changed rust files>
```

Evidence: during the deployment command pre-task, whole-tree `cargo fmt
--check` failed on unrelated committed files, while the per-file rustfmt check
covered the dirty command files before the CLI tests and command smokes passed.

## Deployment Command Smokes

For deployment-management CLI changes, a temporary sysroot can prove the
commands parse repo state and dry-run safely without touching the host system:

```bash
./src/cli/target/debug/nex deployments --path <tmp>/nex/deployments
./src/cli/target/debug/nex rollback --sysroot <tmp> --dry-run
./src/cli/target/debug/nex gc --sysroot <tmp> --keep 1 --dry-run
./src/cli/target/debug/nex deploy systems/desktop-vwl/0.0.1 --sysroot <tmp> --repo .nex/repo --dry-run
```

`nex deploy` must allow the zub repo and `/nex/deployments` directory to live
on different filesystems. Zub uses hardlinks when it can and falls back to
copies across filesystem boundaries.

Evidence: the hardware install mounted `/dev/nvme0n1p3` on `/nex/repo` and
kept `/nex/deployments` under root on `/dev/nvme0n1p2`. The old deploy command
failed before opening the store with `hardlink constraint violated`; the
regression test
`deploy_reaches_store_open_when_repo_and_deployments_use_different_devices`
now proves deploy reaches `Store::open` instead of rejecting the device
boundary.

For live-upgrade changes, start with the fast hardlink smoke:

```bash
scripts/test-live-upgrade-hardlinks.sh
```

The script creates a tiny synthetic system ref in a temporary zub repo, runs
`nex upgrade` inside a user namespace, and proves the deployed probe file
shares an inode with a blob under `/nex/repo/objects/blobs`.

Use the full QEMU proof after changing deploy, rollback, boot selection,
installed repo layout, or anything that can break a real reboot:

```bash
scripts/qemu-test-live-upgrade.sh
```

This script needs the host commands `mcopy` and `mkfs.vfat`. On Arch, install
`mtools` and `dosfstools` when those commands are missing.

That script boots `systems/desktop-vwl/0.0.1`, pulls
`systems/desktop-vwl-nvidia-580/0.0.1` from a configured local zub remote,
deploys it, checks that `etc/os-release` hardlinks to a repo blob, reboots
into the upgraded deployment, runs `nex rollback --yes`, checks rollback
hardlinks, reboots again, and requires `LIVE-UPGRADE-PASS`.

Evidence: EP009 ran both scripts. The fast smoke printed
`live-upgrade-hardlink-pass ... usr/bin/nex-hardlink-probe links=2 ...`. The
full QEMU proof printed `repo-hardlink ... etc/os-release 2 ...`,
`rollback-hardlink <same inode>`, and `LIVE-UPGRADE-PASS`.
