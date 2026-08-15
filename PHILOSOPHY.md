# Nex Philosophy

Nex aims to become a universal way to build, package, install, and update Linux
systems. The same model should serve a small embedded image, a developer
workstation, a large desktop, and other Linux systems without creating separate
packaging worlds for each one.

Nex refers to several related things:

- The Nex concept: reproducible builds, explicit inputs, isolated package
  dependencies, and system state identified by the checksum of its contents.
- The Nex tools: the builder, package manager, zub store integration,
  deployment tools, and boot tools.
- The Nex distribution: the manifests and assemblies that this project
  maintains for complete Linux systems.

People should be able to apply Nex beyond the userland that this repository
happens to provide. Another manifest tree could use a different C library, init
system, kernel policy, or package selection. This project will probably provide
some of those choices, but the Nex model must not require glibc or systemd.

## One Model From Embedded Systems To Desktops

Nex should use the same package format, store format, build rules, and tools
across machine classes. An assembly manifest selects package outputs and
combines them into a root filesystem.

An embedded device will often use a frozen, read-only root filesystem. It does
not need to carry a full package manager or support package-level changes on
the device. A device that must replace its whole system at once and retain the
old system for rollback can also carry zub and use whole-system deployments.

A desktop can keep packages in the Nex layout, install user packages without
root, switch package versions, and commit system changes as new deployments.
Both systems consume the same built package artifacts. Zub deduplicates shared
content rather than forcing each system style to maintain another package
format.

Flat root filesystems remain valid Nex products. Containers, embedded images,
recovery systems, and machines with another boot or update process may need
ordinary files under standard Linux paths instead of a live Nex package layout.

## Reproducibility Is Mandatory

After the bootstrap toolchain converges, Nex must produce identical bits from
the same declared inputs. Nex must not accept flaky checksums, timestamp drift,
filesystem-order drift, host contamination, or any other excuse for different
outputs.

The checksum belongs to a specific target, including at least the CPU
architecture and chosen optimization profile. An x86-64 generic build and an
ARM64 build will naturally have different checksums. Nex still needs to
experiment with whether one manifest holds several target checksums or each
target gets a separate manifest. Nex must require exact results for each target
regardless of that file format.

Nex may use a host compiler to build the first bootstrap seed. Different host
compilers may produce different seed bits. Nex places trust in the open-source
upstream compiler and C library used for that seed, then requires later
toolchain and package builds to converge on deterministic checksums. Manifests
outside the bootstrap tree must carry reproducible output checksums.

The official continuous-integration builder must build a manifest and reject it
when the output does not match its recorded checksum. Developers must fix
nondeterministic output instead of disabling or weakening the check.

## Manifests Must Explain The Build

A smart Linux developer should understand a package by reading one YAML
manifest and its plain shell build script. Nex must not require a custom
language before someone can audit how a package is built.

Nex favors simple, explicit commands over a clever framework. A small amount of
repeated shell is cheaper than hidden behavior spread through a large package
DSL. Helpers may remove real repetition, but they must not hide sources,
dependencies, build flags, installed files, or checksum-relevant behavior.

Every input that can affect output must appear in the manifest or its pinned
build environment. Nex fetches declared sources, verifies their hashes, and
then runs the build without network access. Normal builds run in a filesystem
sandbox so package scripts cannot read or modify arbitrary host files.

Package authors must list build-time dependencies explicitly and by hand. The
builder does not pull compilers, headers, configure helpers, interpreters, or
build tools automatically from installed binaries because those programs may
affect output without remaining in the final package. Runtime libraries follow
a different rule: after a package installs files, Nex inspects ELF objects and
records the libraries that those installed files need. Manifests therefore use
manual build inputs and generated runtime metadata together.

Nex should stay close to upstream defaults. A manifest may patch upstream
software for reproducibility, compatibility, security, or necessary Nex
integration, but the patch and its reason must remain visible. Optional features
depend on the package: the normal manifest should usually resemble upstream,
while explicit variants may offer smaller or differently configured builds.

Package manifests split installed files into outputs such as `bin`, `lib`,
`dev`, `doc`, and `locale`. Assemblies and dependent packages should select
only the outputs they need. Nex should not pull headers, documentation, optional
services, or large dependency trees into a runtime system without a concrete
reason.

## Packages Carry Their Runtime Libraries

A Nex capsule is a package root that contains a program, the exact runtime
libraries it needs, and the libraries that those libraries need. For example,
one program may use glibc 2.39 while another uses glibc 2.40. Each program loads
its own chosen libraries instead of accepting whichever version happens to own
`/usr/lib`.

Zub may hardlink or otherwise deduplicate identical files. Each package can
therefore carry its own dependency set without storing duplicate bytes.
Packages should remain independent even when the store shares their bytes.

Nex may expose one selected version through familiar paths such as
`/usr/bin/foo`. Other installed versions remain available through explicit Nex
paths. The exact symlink and loader design remains open to change while the
project tests difficult packages, but Nex must preserve the ability to install
and run conflicting dependency versions side by side.

This package model should make Docker images, AppImage files, and similar
dependency-bundling formats unnecessary for ordinary software distribution.
Nex does not claim that package dependency separation creates a security
boundary. Users should still use namespaces or virtual machines when programs
need filesystem, process, or network isolation. Nex may eventually provide
convenient tools for those boundaries.

## Git Defines The System

Git stores the manifests that define packages and systems. A zub store caches
the results. The store must never become the only record of how to recreate a
system.

A user should be able to delete the store and rebuild the same outputs from the
same Git revision. An old Git revision should continue to describe and rebuild
the complete old system, subject to the declared source files remaining
available.

Dependencies should point to immutable manifest content. Git branches and tags
may tell users which versions to follow, but mutable branch names must not
change the inputs of an already pinned build.

## Caches Are Untrusted And Replaceable

Anyone may host a binary cache. Nex may download an output from any configured
cache, but it accepts that output only when its checksum matches the trusted
manifest. A cache provides speed and availability, not authority.

When no cache has the expected output, Nex should build it from source. Users
must not depend on access to one company, service, or machine to recover their
systems.

Binary-only upstream software can still use a Nex manifest. Such a manifest
pins the upstream archive, verifies its hash, installs its files
deterministically, and records a reproducible output checksum. The package does
not become source-buildable merely because Nex packages it, but it still gains
the same cache verification and dependency model.

## Registries Are Open

This project will provide an official manifest repository or registry. Its CI
should accept a manifest only after the package builds reproducibly, matches its
checksum, uses declared inputs, passes a functional check, and splits its
outputs sensibly.

Anyone may host, fork, or modify a manifest repository. Users may configure
other registries and build caches. Nex still needs experiments before it fixes
the exact rules for registry priority and conflicting package names. Nex must
not silently let an untrusted registry replace a package from a trusted one.

Users trust a manifest repository as they trust other software maintainers. A
checksum proves that a cached output matches the reviewed manifest; it cannot
prove that upstream software or its build script is harmless. Nex makes the
inputs and result auditable, but it cannot replace community review or trust in
program authors.

## Users And Systems Share Artifacts

Most programs should build and install for an unprivileged user. Some packages
that control hardware or system services will require root, but Nex should keep
those exceptions narrow.

System and user installations must consume the same package artifacts. Nex
should not rebuild or repackage identical software merely because one user
installs it under a home directory and an administrator installs it for the
whole machine.

Nex must not require a background package-management daemon. The CLI performs
builds, installs, updates, deployment changes, and cleanup when a person or an
external scheduler invokes it.

## System Changes Are Atomic

A Nex-managed system should boot an immutable, read-only deployment. Mutable
data such as home directories, application state, logs, and device identity
must live outside that deployment.

Every committed system change creates a new deployment. A user may build a
complete assembly or stage several individual package changes, but Nex commits
the result together as another complete system state. Nex should generally
retain the previous deployment until the new one boots successfully.

A rollback switches the system deployment without rolling back personal files
or application data. Nex must not garbage-collect the only known-good system
while a new deployment remains unproven.

A kernel update may create a deployment that differs only in its kernel
package. Zub can hardlink every unchanged file, so Nex does not need to rebuild
or duplicate the rest of the system. The project may still adjust how kernels
and deployments relate as real machines expose constraints.

## Packages Ship Defaults; Hosts Own Configuration

Nex follows the UAPI Group Configuration Files Specification. Packages and
assemblies place vendor defaults under `/usr`. Programs may accept temporary
overrides under `/run`. The machine owner writes lasting, machine-specific
settings under `/etc`. A Nex-managed machine mounts or otherwise supplies
`/etc` from persistent host state outside the selected deployment, so the
deployment can stay read-only while an administrator changes files such as
`/etc/passwd`, `/etc/group`, `/etc/hosts`, or network settings.

Programs should search `/etc`, then `/run`, then `/usr` when they choose one
main file. A main file in a higher-priority tree completely replaces the same
file below it. Programs that can safely combine drop-ins should read them in
filename order across all three trees. A same-name file in a higher-priority
tree replaces the lower file, and a later filename has higher priority. An
empty file or a link to `/dev/null` masks the same file below it. Scripts and
structured documents may use full-file selection when combining fragments
would change their meaning.

Package upgrades must not merge or overwrite host-owned files in `/etc`.
Because programs read vendor defaults directly from `/usr`, a new deployment
can update those defaults while leaving the host's explicit choices intact. A
rollback selects the earlier `/usr` tree and keeps the same host-owned `/etc`.
Tools may show the administrator how a local file differs from the vendor
default, but they must not guess how to combine the two.

Nex builds the programs in a complete Nex system. When one of those programs
reads only `/etc`, Nex should patch its reader instead of preserving a
historical package layout. Reusable package manifests must not install vendor
defaults under `/etc` or disguise them under `/usr/etc`. They should enable an
upstream vendor-directory option when one exists. A source patch must implement
the standard `/usr`, `/run`, and `/etc` rules without a Nex path, product name,
or assembly policy.

An immutable assembly may keep initial host files under
`/usr/share/factory/etc` and populate a missing host path from them. Software
must not read that factory tree directly, and an upgrade must not replace a
host file that already exists. An assembly that includes an unpatched outside
program may add an explicit compatibility link, wrapper, or generated runtime
view for that program. The adapter belongs to the assembly, not to a reusable
package that does not know whether an assembly needs it.

Product repositories place their own vendor defaults and service choices in
product packages or assemblies. Flat root filesystems must continue to work
without the Nex deployment or boot tools. `CONFIGURATION.md` describes the
concrete directory rules, compatibility adapters, and upgrade behavior.

## Nex Prefers Its Boot Path But Does Not Require It

The Nex distribution prefers its custom UEFI boot manager, kernels with the
common boot drivers built in, and a constant built-in early userspace instead
of a generated initramfs. This design lets the boot manager read deployments
directly and keeps each boot state reproducible.

The wider Nex concept does not require that boot path. Embedded boards may need
vendor firmware, another bootloader, or an initramfs. Flat assemblies may boot
under tools that Nex does not control. Nex should keep its package and assembly
model useful in all of those cases.

## Architectures And Build Profiles

Nex currently focuses on x86-64, but the concept must support other
architectures such as ARM64 and RISC-V. Each architecture should probably have
an official generic optimization profile so public caches can serve most
machines. Users may also build explicit profiles for particular CPUs or size
constraints.

Nex still needs practical experiments before it fixes the manifest schema,
store-ref format, cache lookup rules, and user interface for these variants.
The project should preserve the principle and allow the mechanism to evolve.

## Change The Design While We Still Can

Nex remains exploratory. Maintainers should replace a flawed design when tests
or real systems expose a better answer, even when that breaks manifests,
stores, or existing installations. Maintainers must not preserve old interfaces
when doing so would freeze an unclear model.

The project should distinguish firm principles from current mechanisms. Nex
must always provide reproducible post-bootstrap outputs, explicit inputs,
readable manifests, replaceable caches, side-by-side dependencies, atomic
system state, and tools that require no daemon. Maintainers still need to test
checksum layouts, registry priorities, architecture variants, package symlinks,
kernel switching, and some boot details.

Nex will have succeeded when a significant community uses it on both embedded
devices and full desktops, can reproduce what it runs, can rebuild without a
privileged cache provider, and no longer needs a separate packaging answer for
each class of Linux software.
