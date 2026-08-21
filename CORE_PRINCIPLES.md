# Nex Core Principles

Status: active design principles and open questions.

`PHILOSOPHY.md` explains the broader project rationale. This file keeps the
small set of rules that current design work must preserve. Detailed proposals
must derive their machinery from these rules and must label every unresolved
choice.

Nex must solve four separate problems: building and booting the foundational
system, changing a running system temporarily, letting users manage their own
packages, and running applications or complete systems in isolated
environments. This file is the canonical home for the principles that govern
all four areas. The foundational system now has a defined conceptual model.
The live-modification, user-realm, and isolated-execution still need to be
fully defined.

## Base System

An Assembly is a system definition stored at a path in its own Git repository.
The Assembly repository and the Nex manifests repository have separate
histories. Nex starts from the Assembly path at the commit object ID resolved
from that repository's `HEAD`. Each dependency edge pins the exact commit of
the repository that owns the dependency manifest. Those manifests pin their
own dependencies in turn, so the root Assembly and all transitive dependency
edges form one exact manifest closure across repositories.

A branch, tag, `HEAD`, checkout path, detached checkout, or worktree may help a
user select a Git commit while authoring or updating a system. Nex records the
resulting object ID, not the name or checkout arrangement used to find it. A
Git commit pins every manifest and repository-local file that Nex reads from
that repository snapshot. A content hash pins the bytes of a fetched external
source.

In these principles, Nex *selects* when it resolves a user-facing name to an
object ID, and Nex *pins* when it records that immutable object ID. A Git object
is *available* when Nex can read it locally or obtain it from a source that
promises to supply it. Nex *retains* an object when it protects the required
copy from collection. Git calls an object *reachable* when Git can walk to it
from a ref, reflog, or another retention root. Reachability can retain an
object, but it does not form part of the build identity.

Nex must retain or be able to obtain every commit, tree, blob, and
content-addressed external source needed by the exact manifest closure.

Every build uses exact declared inputs and produces reproducible output after
the bootstrap converges. Manifests remain readable descriptions of those
inputs and build steps rather than programs in a hidden package language.

A realization is one immutable Zub commit produced from that fully pinned
Assembly closure. The Zub commit is the built system. Nex does not need a
separate lock, result database, or image record that restates the same object
graph.

A system checkout is a checkout of a realization. Zub hardlinks the stored
files into it. Every system checkout is read-only, without exception. Writable
host paths mounted into the running system are not part of the checkout.

A machine keeps one or more system checkouts as directories under one
designated location. Every checkout there is a complete bootable system. Its
presence registers and retains it; Nex has no separate installed or retained
state for system checkouts. An ordering convention such as `.0`, `.1`, and so
on selects preferred and fallback boot choices. The exact directory and
rotation rules remain open.

A system change selects another exact Assembly closure, creates or obtains
another Zub commit, and creates another system checkout. Nex never modifies an
existing realization or system checkout. Creating either one does not change
the running system. The machine changes versions by booting another checkout.

Rollback boots a present checkout of an older Zub commit. It does not reverse
writes to persistent host or user data. Nex removes a system checkout only
when it is unused. Removing it ends its retention, but Nex may collect the
underlying Git or Zub objects only when no other system checkout or user
profile needs them.

The bootloader can boot any system checkout Nex selects. Its implementation is
outside these principles.

## Zub Stores

The local Zub store plays the same broad role as a bare Git repository. It is
the object database where realized files live, and it owns the immutable
objects, trees, commits, and refs behind system checkouts. The ability to reproduce
those objects from Git does not make the working local store disposable.

Remote Zub stores act as binary caches and distribution peers. Someone may
build an artifact once and push it; another machine may pull and verify that
exact artifact instead of rebuilding it. A remote supplies bytes, not trust.
Nex accepts a pulled artifact only when it matches the identity trusted through
the manifest.

Several Zub commits can reference the same objects. Read-only hardlinked
system checkouts therefore share package and file data without
copying those bytes or exposing stored objects to mutation.

The local store treats every present system checkout as a retention root. Nex
may use internal refs to implement that rule, but those refs do not create a
second user-visible retention state. Local garbage collection removes only
objects that no present system checkout or user profile needs. Losing the
whole local store requires a rebuild or pull from another store; that is
recovery from lost installed storage, not ordinary cache cleanup.

Nex must not require an always-running package-management daemon. Commands and
external schedulers initiate builds, updates, installs, and cleanup. This rule
does not yet choose how an unprivileged user may safely add a reusable object to
machine-wide storage.

## Live Modification

Nex must support temporary changes to a running system. This capability is
separate from the foundational system-checkout lifecycle. The design has not
yet chosen whether an overlay, another checkout, or another mechanism supplies
it. Whatever mechanism Nex chooses must preserve the immutable realizations
and read-only system checkouts described above.

## User Realm

The following constraints guide later user-realm work. They do not yet approve
a mechanism.

Users manage their own package profiles without modifying the immutable system
checkout. Adding or removing a user package creates or selects another exact
profile tree.

The base system supplies the packages required by the machine and the tools a
user needs to obtain and run additional packages. User and system installs
consume the same package artifact format.

When one user has already realized an exact manifest and package artifact,
another user should be able to use that result rather than build or package it
again, subject to rules that the design still needs to define. Storing one
physical package instance per machine is the preferred outcome, but the design
must first prove that ownership, integrity, privacy, and filesystem behavior
make it sound.

Per-user state should primarily record which shared results that user has
selected. It should not duplicate package bytes merely to represent a different
selection.

## Isolated Execution

Nex must run applications and complete systems inside explicit isolation
boundaries. Package capsules already solve dependency isolation by carrying an
exact runtime closure. Runtime isolation adds controlled filesystem, process,
user, capability, and network views; dependency isolation alone does not create
a security boundary.

Isolated execution consumes the same package and Assembly realizations as
normal system and user installs. Nex does not copy the dependency closure into
a separate container image or require a background container daemon merely to
run it in isolation. Remote Zub stores distribute the same exact realizations
for direct, installed, and isolated use.

Writable runtime data remains outside immutable Zub realizations. Programs
should run without root whenever the requested access and the host kernel
permit it. The builder and runtime isolation may share kernel primitives such
as namespaces, but each applies policy for its own purpose.

## Persistent Data

System checkouts and package profiles own built software. They do not own home
directories, application data, logs, machine identity, or service databases.

The machine owner keeps lasting configuration under `/etc`; services keep
lasting data under `/var`; users keep their data in their home directories.
A system-checkout rollback does not claim to roll those trees backward.

Packages ship vendor defaults under `/usr`. Nex does not overwrite an existing
host-owned `/etc` file during an ordinary package or system-checkout change.

## Open Questions

Still need to decide:

- whether one physical Zub store can safely serve the system and every user;
- how an unprivileged user publishes a reusable manifest and artifact without
  gaining authority over machine or other-user state;
- which files belong under `/nex/users/<uid>` and who owns each one;
- whether Nex needs any private package object at all, and when privacy or
  policy would forbid machine-wide reuse;
- how user profiles reuse local Zub objects without duplicating package data;
- how Nex keeps pinned Git objects available and protects the required copies
  from collection, both locally and in repositories that supply missing
  objects;
- whether a complete manifest repository, selected checkouts, or both must
  remain present on every running machine as repositories grow;
- how Nex identifies package-manifest repositories beyond the Nex manifests
  repository and resolves packages with conflicting names;
- what user-facing commands create, inspect, order, boot, and remove system
  checkouts and profiles;
- which system changes Nex can activate safely without rebooting, which
  processes or services it must restart, and which changes require rebooting;
- which filesystem, process, user, capability, network, and resource boundaries
  isolated execution applies by default or by explicit request;
- how Nex represents the writable state and lifetime of an isolated run;
- how Nex derives Git and Zub garbage-collection roots from present system
  checkouts and user profiles;
- what metadata, if any, links a Zub realization to its Assembly repository,
  resolved commit, and path;
- what, if any, per-user or global storage limits Nex requires.

## Not Current Decisions

No current principle requires or approves:

- a privileged "broker" or background Zub service;
- one private Zub store per user;
- a root-curated shared-capsule publication layer;
- a mandatory hard quota for every user;
- a particular live-modification mechanism, including a writable overlay on
  the booted system checkout;
- a special one-shot test boot;
- a separate container image format or mandatory container daemon;
- a particular orchestration model for isolated workloads;
- separate request, lock, candidate, and installed databases;
- a raw boot-region design;
- any particular path or command names before the design chooses them.
