# Make Desktop VWL generic

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

After this work, a third party can use Desktop VWL and its reusable Systemd
base without inheriting a Nex developer's account, home directory, wireless
network, IP address, SSH bypass, or debugging service. The built system will
contain only distribution policy that makes sense for an unknown machine.

An installer, product assembly, or first-boot provisioner will create the
machine's administrator account, hostname, network connections, SSH keys, and
other site choices. The repository's QEMU tests will add disposable test data
to their own writable images instead of storing that data in production
assembly manifests.

This plan changes both `asm/desktop-vwl/desktop-vwl-overlay.yaml` and
`asm/nex-systemd-overlay.yaml`. Desktop VWL extends `asm/nex-systemd.yaml`, and
the base currently supplies some of the same test account and SSH policy. A
generic child cannot safely inherit a nongeneric base. Nvidia and development
desktop manifests extend Desktop VWL, so the plan rebuilds and checks those
children too.

This plan removes product-specific policy. The next plan will decide which
remaining immutable defaults belong below `/usr` and which true machine-state
files should enter `/etc` on first boot.

## Progress

- [x] (2026-08-15 21:23Z) Ran the Ralph worktree pre-task against a clean
  worktree, selected EP014 as the lowest active plan, listed all ten durable
  knowledge files, and read the assembly, graphical QEMU, installer,
  reproducibility, and agent-workflow notes.
- [x] (2026-08-15 21:36Z) Froze the manifest inventory with the new policy
  checker. The old Systemd overlay had eight test-identity or home matches,
  five SSH-bypass matches, three permissive SSH settings, two SSH enablement
  links, and an empty root password. The old desktop overlay added 20 identity
  or home matches, eleven site-network matches, three bypass matches, three
  permissive settings, two SSH links, fourteen debug-helper matches, and an
  empty root password.
- [x] (2026-08-15 21:36Z) Added self-tested reusable-assembly and disposable
  QEMU-identity helpers. The policy checker rejects every known unsafe class,
  accepts a locked generic fixture, fails against the old inventory, and now
  passes against both cleaned overlays.
- [x] (2026-08-15 22:18Z) Removed the built-in interactive user, personal home tree, site network,
  permissive SSH policy, test key helper, and debug-only services. Leave root
  locked and leave remote access disabled until a provisioner configures it.
- [x] (2026-08-15 22:41Z) Changed QEMU and finished-root tests so each test injects the temporary
  identity, credentials, and network state that it needs into a disposable
  test image.
- [x] (2026-08-15) Fixed Linux 6.18 basic module-version records, rebuilt the
  kernel, then built Systemd, Desktop, both Nvidia children, and desktop-dev
  from parent to child. Every strict command built twice and reproduced.
- [x] (2026-08-15) Booted the final Systemd and Desktop refs, rendered Chromium,
  exercised the installer assertion, and completed upgrade plus rollback from
  the final Desktop ref to the final Nvidia-580 ref.
- [x] (2026-08-15) Scanned all five final roots, ran focused desktop and tool
  checks, promoted verified notes into four durable knowledge files, recorded
  the final checks below, and stopped for human review.

## Surprises & Discoveries

- Observation: Desktop VWL inherits product-specific policy from its reusable
  Systemd parent as well as declaring similar policy in its own overlay.
  Evidence: `asm/desktop-vwl/desktop-vwl.yaml` extends `asm/nex-systemd.yaml`,
  while both `asm/nex-systemd-overlay.yaml` and
  `asm/desktop-vwl/desktop-vwl-overlay.yaml` declare `testuser`, empty password
  fields, permissive SSH settings, and test home content.

- Observation: Desktop VWL currently embeds one developer's site network.
  Evidence: `asm/desktop-vwl/desktop-vwl-overlay.yaml` contains a `wegelnet`
  connection with a tracked Wi-Fi secret, address `192.168.3.110/24`, gateway
  `192.168.3.2`, and DNS server `192.168.3.2`.

- Observation: The desktop currently turns test access into production policy.
  Evidence: the overlay enables `nm-autoconnect` and `nex-boot-dump`, permits
  root login and empty SSH passwords, and points OpenSSH at
  `/usr/local/bin/accept-any-key`.

- Observation: The final Desktop VWL deployment keeps its mutable tree outside
  the immutable system tree as intended.
  Evidence: Desktop VWL checksum
  `50ed467e695adcb994ce924ad3ed5123d0feeb92ced1b0db828c7673165d7809`
  has no files or links in the stored deployment's `/etc`; its factory tree
  contains 88 leaves that first boot can seed into writable state.

- Observation: The EP014 pre-task found no tracked, untracked, ignored, or
  half-finished changes to classify.
  Evidence: `rtk git status --short --untracked-files=all` printed no paths at
  commit `5dd6708`, so the plan started from the pushed plan-only checkpoint.

- Observation: The graphical and live-upgrade QEMU scripts already assemble
  writable `/etc`, `/home`, and `/root` trees before making their ext4 images.
  Evidence: `scripts/qemu-test-graphical.sh` and
  `scripts/qemu-test-live-upgrade.sh` create `var-content` trees and use
  `mke2fs -d`. `scripts/prepare-qemu-test-identity.sh` can therefore copy the
  production service accounts, add `nex-test` only to that tree, install an
  ephemeral public key, and enable SSH without modifying a system ref.

- Observation: The direct Systemd QEMU path does not need SSH.
  Evidence: `scripts/qemu-test-systemd.sh` delegates to the direct-initramfs
  serial assertion in `scripts/qemu-test-installer.sh`. The assertion can test
  a locked root, absence of human accounts, disabled SSH, closed TCP port 22,
  and rejection of a blank root password before printing `ASSERT-BOOT-PASS`.

- Observation: The finished-root regression test fails against the EP013
  desktop before reaching later checks.
  Evidence: after checking out `systems/desktop-vwl/0.0.1` to
  `.nex/tmp/ep014-baseline-root`, `scripts/test-generic-system-root.sh` printed
  `FAIL: root is not locked`.

- Observation: the installer image-sizing self-test uses
  `--self-test-image-sizing`, not the initially attempted
  `--self-test-direct-sizing` spelling.
  Evidence: the first command returned `unknown option`; the documented option
  then printed `PASS: installer direct image sizing`.

- Observation: the Linux 6.18.24 package enables `CONFIG_MODVERSIONS` but
  disables both version-record formats, so every loadable module lacks the
  records that the running kernel requires and fails with `Exec format error`.
  Evidence: the live-upgrade guest journal records failures for `virtio_net`
  and `pkcs8_key_parser`; the installed config has `CONFIG_MODVERSIONS=y` but
  neither `CONFIG_BASIC_MODVERSIONS` nor `CONFIG_EXTENDED_MODVERSIONS`; and
  `readelf -S virtio_net.ko` shows no `__versions` section. Linux 6.18's
`kernel/module/Kconfig` defaults `BASIC_MODVERSIONS` to yes when module
  versions are enabled.

- Observation: the live-upgrade fixture's old `/var` sizing formula left too
  little room for a complete pull of the current desktop repository.
  Evidence: a 9,711 MiB staged source repo filled a 21,470 MiB filesystem while
  Zub wrote `/nex/repo/objects/blobs`. A 31,181 MiB image, sized from the
  staged payload plus two additional repo sizes and 2,048 MiB, pulled
  9,617,341,112 bytes across 164,463 objects and completed the full flow.

- Observation: a multi-command SSH probe can hide an early failed `test` when
  a later `printf` succeeds.
  Evidence: the first upgraded-file probe looked for absent `etc/os-release`,
  logged a failed `stat`, and still returned success. The final probe starts
  with `set -eu` and checks the regular immutable file at
  `usr/share/factory/etc/os-release` instead of the `/usr/lib` compatibility
  symlink.

## Decision Log

- Decision: Fix the reusable Systemd base in the same plan as Desktop VWL.
  Rationale: a child assembly cannot undo every unsafe parent artifact
  reliably, and other children must not inherit a test account or SSH bypass.
  Date: 2026-08-15.

- Decision: Ship no enabled interactive account in a generic reusable system.
  Keep the root account locked. Let an installer or provisioner create an
  administrator with an explicit credential.
  Rationale: any built-in credential either leaks access or makes an arbitrary
  product choice. A generic image cannot know the owner or authentication
  method of its eventual machine.
  Date: 2026-08-15.

- Decision: Keep OpenSSH unavailable by default until the machine owner has
  installed an authentication method and explicitly enabled the service.
  Rationale: removing an empty-password bypass is not enough if the image still
  exposes a remotely reachable service before the owner can secure it.
  Date: 2026-08-15.

- Decision: Put test-only identities and access hooks in the QEMU test setup,
  not in an assembly that users can install.
  Rationale: the test still needs to prove login and desktop behavior, but its
  disposable disk gives those values a clear lifetime and prevents them from
  entering a released system ref.
  Date: 2026-08-15.

- Decision: Defer the broad `/etc` placement audit to EP015.
  Rationale: this plan first removes data that should not ship at all. EP015
  can then classify the smaller factory tree without mistaking test policy for
  required machine state.
  Date: 2026-08-15.

- Decision: Rebuild `nex-systemd`, then Desktop VWL, then the two Nvidia
  siblings, then `desktop-dev`; build the siblings sequentially.
  Rationale: Desktop VWL extends the changed Systemd base and snapshots `asm/`
  plus `scripts/`. Each desktop child extends that result. `nex-minimal` and
  the installer assembly do not extend either changed assembly, while the
  direct installer QEMU harness can test the final Desktop VWL ref without
  rebuilding the installer image.
  Date: 2026-08-15.

- Decision: Fix the kernel's missing basic module-version records in this
  plan and make the kernel build require that setting.
  Rationale: the required live-upgrade boot found a system-wide kernel defect,
  not a test-only network choice. Avoiding the module in QEMU would leave every
  real device unable to load its modular hardware drivers.
  Date: 2026-08-15.

- Decision: Size the live-upgrade writable image from measured repository
  content and make its deployment-file probe fail fast.
  Rationale: a full desktop repository can exceed a fixed safety margin, and
  a successful final shell command must not mask a missing immutable file.
  Date: 2026-08-15.

- Decision: Send noisy build and test output to ignored logs and inspect only
  exit status, checksums, pass markers, and focused failure excerpts.
  Rationale: full assembly transcripts contain thousands of routine lines and
  do not help the human or agent when the command succeeds.
  Date: 2026-08-15.

## Outcomes & Retrospective

The reusable Systemd and Desktop family now ships as an unprovisioned generic
system. Root is locked, no human account or home ships, sshd stays disabled,
and the overlays contain no product Wi-Fi, site address, permissive SSH rule,
key bypass, or debug-only boot helper. An installer or product provisioner must
create the administrator, network state, and remote-access policy.

QEMU no longer depends on production test access. A shared helper copies the
factory service accounts into the disposable writable tree, then adds either a
temporary `nex-test` desktop identity or root-only upgrade access, an ephemeral
key, and test-only sshd enablement. The direct serial boot path adds no identity
and proves the locked fresh-system policy itself.

The final assembly checksums are Systemd
`d9ec2179769be8eab52f3fd02b9d0a648f9b243819fa2d54d00048ee6a2c44da`,
Desktop `8a00e2f4d65dc989478f667fba9139d47153bb920a7b6399ba218764a20f15a4`,
Nvidia 580 `785b6956d655a05566f7f28e2b05514c24fc0c4c76af803ce89de1077e44ffdc`,
Nvidia current
`8a49f779dd79e913f86fa1b3c4d928efe229a140ee4a4e2516fb6cf7b0fe1762`,
and desktop-dev
`4e219893cb11830f121c8092119c00b14e69eb9342940c34b794d612a458a0e8`.
Each strict check produced the same checksum in both builds.

The required QEMU paths passed against the final refs. Systemd and installer
printed `generic-access-policy=ready` and `ASSERT-BOOT-PASS`. Chromium reported
the expected page title, `1280 800`, `srgb(240,0,255)`, and
`ASSERT-GRAPHICS-PASS`. The live-upgrade proof booted Desktop `8a00e2f4...`,
upgraded and booted Nvidia 580 `785b6956...`, reused the original deployment
inode during rollback, booted rollback serial 2, and printed
`LIVE-UPGRADE-PASS`.

The plan did not boot real Nvidia hardware or build a full USB installer. Both
Nvidia finished roots ran their packaged `nvidia-smi --help`, the rebuilt
`virtio_net.ko` contains module version records, and the direct installer path
booted the final Desktop ref. A hardware GPU load and full-media install remain
useful downstream checks, but neither is needed to prove removal of reusable
product policy. ShellCheck was not installed; Bash and POSIX syntax checks,
script self-tests, finished-root tests, and the live QEMU paths all passed.
EP015 still owns the classification and upgrade behavior of the remaining
factory `/etc` entries.

## Context and Orientation

`asm/nex-systemd.yaml` defines the reusable bootable Systemd base.
`asm/nex-systemd-overlay.yaml` supplies assembly-owned files and links.
`asm/nex-minimal.yaml` and other manifests may extend or reuse this base; map
the full child graph before editing it.

`asm/desktop-vwl/desktop-vwl.yaml` extends the Systemd base and adds the
Wayland desktop. `asm/desktop-vwl/desktop-vwl-overlay.yaml` supplies desktop
accounts, services, network files, and integration policy. The two Nvidia
manifests and `asm/desktop-dev.yaml` extend Desktop VWL.

Nex-structured assemblies store immutable files in a deployment and copy only
missing factory paths into writable `/etc` and `/var` on first boot. Read
`CONFIGURATION.md` before changing those files. Do not use this plan to move
ordinary vendor defaults merely because they appear in the same overlays;
EP015 owns that classification and its upgrade test.

The main boot tests live under `scripts/`. In particular,
`scripts/qemu-test-systemd.sh`, `scripts/qemu-test-graphical.sh`, and
`scripts/qemu-test-installer.sh` create disposable guests and inspect live
behavior. Supporting finished-root scripts may assume `testuser` exists today.
Find every such assumption before removing the account from the assembly.

Use `/home/wegel/work/perso/zub/target/debug/zub` for current store operations.
The user has authorized changes to the Zub repository when a real Zub bug
blocks this plan. Keep any Zub fix in its own checked commit in that repository,
then update this plan with the cause and proof.

## Plan of Work

Start with a literal and semantic inventory. Search the two overlays, their
parent and child manifests, every QEMU script, finished-root test, and installer
helper for account names, UID 1000, password fields, SSH settings, Wi-Fi
credentials, private address choices, `/home` content, `accept-any-key`,
`nm-autoconnect`, and `nex-boot-dump`. Record each production value and each
test that depends on it.

Write a small repository check with its own failure fixtures. The check must
inspect reusable system overlays and fail for the unsafe constructs this plan
removes. It must not claim to detect every possible secret. Give it an explicit
scope and allow a product assembly to carry product policy outside the reusable
base when that assembly owns the choice. Wire the check into the normal manifest
or repository checks documented in `.agents/TESTING.md`.

Remove the `testuser` account, its subuid and subgid ranges, its home directory,
and its desktop dotfiles from reusable assembly outputs. Keep only accounts
that programs and services require. Lock root with the normal shadow syntax and
prove that neither a console nor SSH accepts a blank password. Do not invent a
default human account.

Remove the `wegelnet` NetworkManager profile, its secret, its static address,
and any helper whose sole purpose is connecting that profile. Leave generic
network daemons and safe backend policy intact. The production assembly should
wait for installer, product, or first-boot network data.

Remove `accept-any-key`, `PermitEmptyPasswords yes`, permissive root SSH login,
and default SSH enablement. Retain only secure, generic OpenSSH behavior. Remove
`nex-boot-dump` and other debug-only boot helpers unless inspection proves that
a helper serves a documented product-independent runtime purpose. Record that
evidence in the plan before retaining one.

Adapt tests before they need the removed values. Create test-only account and
authentication state on the disposable image or through a test-specific
overlay that cannot enter the built system ref. Give the test user UID 1000
only if desktop software or the test protocol requires that exact ID. Generate
or inject a temporary SSH key when a test needs remote access. Do not store a
password, private key, site SSID, or site address in a production manifest.

Build from the lowest changed parent through every affected child. A parent
checksum change invalidates child assembly inputs, so do not build siblings in
parallel when they update shared refs. Run each strict build twice and compare
the output checksum. Then boot the Systemd base, Desktop VWL, and the installer
path against the final desktop ref. The graphical test must render a real app.
The Systemd test must show that the generic image boots without a human account
or provisioned network. A focused security test must prove that root remains
locked and OpenSSH does not listen before explicit provisioning. A separate
test must prove that injected disposable state enables the login path that the
QEMU test actually uses.

Finally, check out each finished system and scan it for the removed account,
home, network, secret, SSH bypass, and debug files. Search both the immutable
deployment and the factory tree. Update durable knowledge only with facts that
the final builds and live guests prove.

## Concrete Steps

Run all commands from the repository root. Prefix shell commands with `rtk` as
required by the local agent setup.

1. Run the pre-task and gather context:

       rtk git status --short --untracked-files=all
       rtk ls .agents/knowledge
       rtk semeja search "test user SSH network setup in reusable assemblies" .
       rtk rg -n 'testuser|wegelnet|192\.168\.3\.|accept-any-key|PermitEmptyPasswords|nm-autoconnect|nex-boot-dump|/home/' asm scripts

2. Map the parent and child assembly graph. Inspect at least
   `asm/nex-minimal.yaml`, `asm/nex-systemd.yaml`,
   `asm/desktop-vwl/desktop-vwl.yaml`, both Nvidia children,
   `asm/desktop-dev.yaml`, and the installer manifests. Update this plan with
   the exact rebuild order before changing checksums. The mapped order is
   `asm/nex-systemd.yaml`, `asm/desktop-vwl/desktop-vwl.yaml`,
   `asm/desktop-vwl/desktop-vwl-nvidia-580.yaml`,
   `asm/desktop-vwl/desktop-vwl-nvidia-current.yaml`, then
   `asm/desktop-dev.yaml`. The two Nvidia siblings must run sequentially.

3. Add the focused reusable-assembly policy check and executable self-test.
   Record its command here after selecting its final path. Run the self-test
   once against failing fixtures and once against the cleaned real tree.

4. Check every changed manifest:

       rtk ./src/cli/target/debug/nex check <changed-manifest>

5. Build every affected assembly in parent-before-child order. Use this exact
   form twice for each manifest:

       rtk ./nex build <manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs

6. Run shell checks and any focused test added or changed by this plan. At a
   minimum, use `bash -n` on each changed Bash script and run that script's
   documented self-test.

7. Boot the final roots with the current Zub binary:

       rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub scripts/qemu-test-systemd.sh systems/nex-systemd/0.0.1 --timeout 120
       rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub scripts/qemu-test-graphical.sh --target-ref systems/desktop-vwl/0.0.1 --app chromium --timeout 300
       rtk env ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub TARGET_REF=systems/desktop-vwl/0.0.1 scripts/qemu-test-installer.sh --direct-initramfs --assert-boot --headless --timeout 180

8. Check out each final system with Zub and run the finished-root scans. Record
   the exact checkout and scan commands after the final test helper exists.

9. Before each commit, run every integrated check that matches the staged
   files. Stage exact paths, commit a useful working checkpoint with a scoped
   imperative subject, and push the checked branch because the user requested
   upstream checkpoints for this work.

## Validation and Acceptance

The plan is complete only when all of the following statements are true:

- The reusable Systemd and Desktop VWL assembly family contains no `testuser`,
  personal home content, `wegelnet`, tracked Wi-Fi credential, site-specific
  IP or DNS address, `accept-any-key`, empty-password SSH permission,
  `nm-autoconnect`, or `nex-boot-dump` artifact.
- Root has a locked password in every reusable finished root. OpenSSH does not
  accept root or blank-password access and does not listen before an explicit
  provisioning step enables it.
- A fresh generic Systemd or desktop guest boots successfully without a human
  account and without a preconfigured network connection.
- The QEMU tests create their own disposable identity and authentication data,
  exercise the intended login or desktop path, and leave that data out of the
  stored production system refs.
- The reusable-assembly policy check fails on each prohibited test fixture and
  passes on the real assembly tree.
- `nex check` passes for every changed manifest.
- Two strict builds of each affected assembly produce the same checksum.
- The Systemd boot test, graphical Chromium render, installer boot test, and
  focused locked-account and provisioning tests pass against the final refs.
- Finished-root scans cover the immutable deployment and factory tree and find
  none of the prohibited product values.
- The plan records all exact commands, checksums, and results. It lists any
  runtime path that could not be exercised rather than treating a file check as
  runtime proof.

### Completion Check

The following final commands passed on 2026-08-15. Commands with large output
wrote their full transcript to the named ignored file under `.nex/tmp/`.

- `./nex build pkg/core/kernel/linux.yaml --verbose --single --check
  --update-checksum --force --compute-deps --record-profile
  --generate-outputs` reproduced kernel build checksum
  `18b77203f0569f6f1730c36092581bbc1f4179f35b5ad03e29be56b94bc089ab`.
  `readelf -S` on final `outputs/drv-net-virtio` found `__versions` in
  `virtio_net.ko`.
- The same strict command built each affected assembly from parent to child.
  Logs `ep014-build-{nex-systemd,desktop-vwl,desktop-vwl-nvidia-580,
  desktop-vwl-nvidia-current,desktop-dev}.log` contain matching first and
  second checksums listed in `Outcomes & Retrospective`.
- `./nex check` passed for the kernel manifest and all five assembly manifests.
  `scripts/check-generic-assembly-policy.sh --self-test`, the real policy scan,
  `scripts/prepare-qemu-test-identity.sh --self-test`, both graphical
  self-tests, the installer image-sizing self-test, all changed shell syntax
  checks, executable-bit checks, and `git diff --check` passed in
  `ep014-final-checks.log`.
- `ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub
  scripts/qemu-test-systemd.sh systems/nex-systemd/0.0.1 --timeout 120` passed
  in `ep014-qemu-systemd.log`.
- `ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub
  scripts/qemu-test-graphical.sh --target-ref systems/desktop-vwl/0.0.1
  --app chromium --timeout 300` passed in `ep014-qemu-graphical.log`.
- `ZUB_BIN=/home/wegel/work/perso/zub/target/debug/zub
  TARGET_REF=systems/desktop-vwl/0.0.1 scripts/qemu-test-installer.sh
  --direct-initramfs --assert-boot --headless --timeout 180` passed in
  `ep014-qemu-installer.log`.
- `PATH=/home/wegel/work/perso/zub/target/debug:$PATH
  RUSTUP_TOOLCHAIN=stable scripts/qemu-test-live-upgrade.sh` passed against the
  final Desktop and Nvidia-580 refs in
  `ep014-qemu-live-upgrade-final-refs.log`.
- `scripts/test-generic-system-root.sh` passed on final checkouts of Systemd,
  Desktop, both Nvidia variants, and desktop-dev. The Desktop checkout also
  passed `scripts/test-desktop-command-configs.sh`,
  `scripts/test-desktop-xdg-autostart.sh`, and
  `scripts/test-desktop-libvirt-integrations.sh`. Both Nvidia roots ran
  `nvidia-smi --help`; desktop-dev reported Rust and Cargo 1.91.1.

## Idempotence and Recovery

Strict builds may update manifest checksums and generated dependency metadata.
Run parent assemblies before children and inspect `git diff` after each build.
If a build fails, keep the last checked manifest state and rerun the same
manifest after fixing the cause. Never discard unrelated dirty files.

QEMU tests must create disposable images and test credentials. A failed test
may remove only the exact temporary path that it created. Do not reuse a guest
that contains state from a previous run when proving first-boot behavior.

If changing the test setup exposes a Zub defect, reproduce it with the smallest
command in `/home/wegel/work/perso/zub`, add a regression test there, commit and
push the Zub fix separately, and then resume this plan with the fixed binary.

## Artifacts and Notes

The EP013 baseline assembly checksums are:

- Nex Minimal: `6bf44276a0de73189bc0c9b9a5d7060b401749f7600a0ecd57eb895d66f3a6e7`
- Nex Systemd: `e4834b42b6be0c89737c948e00a90120666cf6a9f9b4b497bbd0b06b12470e80`
- Installer: `499b0b54a68b50427488a1b001a719b2c449c8bb64f50b55b630c162a8ba0c50`
- Desktop VWL: `50ed467e695adcb994ce924ad3ed5123d0feeb92ced1b0db828c7673165d7809`
- Nvidia 580 desktop: `d5b5547967fa8d24cc799bd2d814079faca50eb1445dd0d180f843ba02260077`
- Nvidia current desktop: `4d29fe18f4067c1c1d8de6305ba6a665efc40cec988963a42c1a5985d8485413`
- Desktop development system: `00e27517697fa1c68fbec05963fb6be654de40d81c9b12b42279c79b873a5c96`

These values identify the starting refs. New values are expected when the
assembly contents change.

The initial executable checks passed:

- `rtk scripts/check-generic-assembly-policy.sh --self-test`
- `rtk scripts/prepare-qemu-test-identity.sh --self-test`
- `rtk scripts/check-generic-assembly-policy.sh` after cleaning the overlays
- Bash or POSIX shell syntax checks for all five changed scripts
- both graphical script self-tests
- `rtk scripts/qemu-test-installer.sh --self-test-image-sizing`

The policy check deliberately failed before the overlay cleanup and reported
every inventory class listed in `Progress`. The finished-root test deliberately
failed against the stored EP013 desktop because root had an empty password.

## Interfaces and Dependencies

The plan may change assembly overlays, assembly checksums, QEMU setup scripts,
finished-root tests, and a focused repository policy checker. It must not add a
new Nex-only account format, secret store, or network format.

The installed system continues to use standard account files, OpenSSH,
NetworkManager or Systemd network interfaces, and Systemd unit enablement.
Installers and product assemblies own concrete machine choices. EP015 will
place the remaining defaults and machine state in their final `/usr`, factory,
or live `/etc` locations and will test upgrades across two system versions.
