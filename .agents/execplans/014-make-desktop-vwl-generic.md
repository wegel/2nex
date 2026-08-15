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

- [ ] Run the Ralph worktree pre-task, list `.agents/knowledge/`, and read the
  notes about assemblies, QEMU, installers, configuration, and reproducible
  builds.
- [ ] Freeze a checked inventory of every account, credential, home file,
  network setting, SSH exception, test helper, and debug service supplied by
  the reusable Systemd and Desktop VWL assemblies.
- [ ] Define the generic installed-system contract and add a repository check
  that rejects the known classes of product-specific data from reusable
  assembly manifests.
- [ ] Remove the built-in interactive user, personal home tree, site network,
  permissive SSH policy, test key helper, and debug-only services. Leave root
  locked and leave remote access disabled until a provisioner configures it.
- [ ] Change QEMU and finished-root tests so each test injects the temporary
  identity, credentials, and network state that it needs into a disposable
  test image.
- [ ] Check and build the affected assembly family from parent to child, then
  exercise the Systemd, graphical desktop, and installer boot paths.
- [ ] Scan the finished roots for prohibited product data, promote durable
  knowledge, record the final checks, and stop for human review.

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

## Outcomes & Retrospective

Not started. At completion, summarize the generic system contract, list each
removed product assumption, identify how tests now provision disposable state,
record exact output checksums and boot results, and state every skipped check
or remaining product-specific value.

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
   the exact rebuild order before changing checksums.

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

## Interfaces and Dependencies

The plan may change assembly overlays, assembly checksums, QEMU setup scripts,
finished-root tests, and a focused repository policy checker. It must not add a
new Nex-only account format, secret store, or network format.

The installed system continues to use standard account files, OpenSSH,
NetworkManager or Systemd network interfaces, and Systemd unit enablement.
Installers and product assemblies own concrete machine choices. EP015 will
place the remaining defaults and machine state in their final `/usr`, factory,
or live `/etc` locations and will test upgrades across two system versions.
