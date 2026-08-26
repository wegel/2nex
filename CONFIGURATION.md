# Configuration and `/etc`

Nex boots an immutable system without making the machine's settings immutable.
The selected deployment supplies `/usr`. The machine supplies a separate,
writable `/etc`, while services keep their persistent state under `/var`.

Nex follows the [UAPI Group Configuration Files
Specification](https://uapi-group.org/specifications/specs/configuration_files_specification/).
Programs use the same directory order whether Nex builds a small appliance or a
complete desktop.

## Each Tree Has One Owner

| Path | Owner | Contents | Lifetime |
| --- | --- | --- | --- |
| `/usr/lib` and `/usr/share` | The package or assembly | Immutable vendor defaults and support files | One deployment |
| `/run` | The running system | Temporary overrides, generated files, and caches | One boot |
| `/etc` | The machine owner | Lasting machine and administrator choices | Across upgrades and rollbacks |
| `/var/lib` | Services | Persistent service-owned state | Across upgrades and rollbacks |

A package must not install a vendor default under `/etc`. It must not use
`/usr/etc` as a substitute either. Architecture-dependent defaults normally
belong below `/usr/lib`; architecture-independent data normally belongs below
`/usr/share`.

An assembly may supply initial machine choices such as accounts, network
settings, enabled services, or login policy. A program may create service state
under `/var/lib`, but it must not treat that directory as an administrator
configuration interface.

## Programs Read All Three Configuration Tiers

Nex builds the programs in a Nex system. When an upstream program reads only
`/etc`, Nex should patch that reader instead of preserving a historical package
layout.

For one main file, a program checks these paths in order:

1. the administrator file under `/etc`;
2. the temporary file under `/run`;
3. the vendor file under `/usr`.

The first file found wins completely. An empty administrator file can therefore
mask a lower file when the file format permits an empty file.

Programs that safely combine drop-ins read all three trees. A file under
`/run` replaces a same-named vendor file, and a file under `/etc` replaces both.
A same-named link to `/dev/null` masks the lower file. The program then reads
the resulting filenames in their documented order.

Nex keeps each patch useful to other Linux systems. A patch adds the generic
`/usr`, `/run`, and `/etc` rules without a Nex pathname, product name, or boot
assumption. If upstream already offers vendor-directory build options, the
manifest uses them instead.

## Assemblies Own Machine Policy

An assembly selects packages and supplies the machine policy that turns them
into a system. A flat assembly may write its final machine settings directly
to `/etc`. A Nex-structured assembly stores initial files under
`/usr/share/factory/etc`. Early userspace copies a factory path only when the
persistent host `/etc` does not already contain it.

This rule lets an assembly create an initial `/etc/passwd`, `/etc/group`,
`/etc/hosts`, network configuration, and service enablement links. After the
first boot, the machine owner controls those paths. A package upgrade cannot
replace them.

Factory files are seeds, not another vendor-default search tree. Programs do
not read `/usr/share/factory/etc` directly.

## Provisioners Supply Machine Choices

`nex-install --provision ETC_TREE` copies an explicit set of ordinary `/etc`
files after it copies the selected deployment's factory seeds. The tree uses
paths relative to `/etc`. A provisioner can supply:

- `hostname`, `hosts`, `localtime`, `locale.conf`, `vconsole.conf`, and
  `fstab`;
- `passwd`, `group`, and `shadow` for the administrator account and other
  local accounts;
- NetworkManager connections below `NetworkManager/system-connections/`;
- Systemd network files below `systemd/network/`.

The installer rejects every other path and every special file. It accepts a
symlink only for `localtime`, and that link must point below
`/usr/share/zoneinfo`. The installer forces mode `0600` on `shadow` and
NetworkManager connection files, sets other regular files to mode `0644`, and
makes root own every installed file and link. Files in this tree replace
same-named factory seeds before the machine first boots.

A reusable assembly may omit any choice that has a safe program fallback. For
example, Systemd can boot without `locale.conf` or `vconsole.conf`. A site that
needs a specific language or console layout supplies those standard files in
its provision tree.

## Outside Programs Get Explicit Adapters

Some users will add a program that Nex does not build or patch. Such a program
may know only a fixed `/etc` pathname. The assembly that adds that program also
adds the smallest adapter it needs. It can pass an explicit pathname, wrap the
program with an environment variable, add a stable symlink, or generate a
compatibility file under `/run` and expose it at the expected path.

The package that owns the vendor data still installs that data below `/usr`.
The adapter belongs in the assembly because the assembly knows that it includes
the unpatched consumer. An ordinary Nex assembly should not carry compatibility
paths for programs it does not contain.

An adapter must preserve a real administrator interface. For example, an
assembly must not replace a standard writable configuration directory with a
link to a read-only vendor directory when administrators need to add files
there. In that case the assembly should patch the reader, pass an explicit
path, or generate a combined view under `/run`.

## Upgrades Do Not Merge `/etc`

Nex upgrades a machine as follows:

1. Nex creates a new immutable deployment with a new `/usr` tree.
2. The boot path selects that deployment atomically.
3. The machine mounts the same persistent `/etc` and `/var` trees.
4. Early userspace copies only factory paths that are still missing.
5. Services rebuild temporary views and caches below `/run`.

A program sees the new vendor default immediately unless an administrator file
overrides it. Nex never rewrites or merges an existing `/etc` file merely
because a package changed its default. A removed factory seed may remain in
`/etc`, because the machine owner took ownership when Nex first copied it.

A rollback selects the older `/usr` tree but retains the same `/etc` and
`/var`. Nex does not pretend it can infer how to roll back machine data. An
administrator tool may show differences between `/etc` and vendor defaults,
but it must not guess how to combine them.

Compatibility links should point to stable paths below `/usr` or `/run`. The
link can then survive while an upgrade replaces the implementation behind it.
If an adapter must change shape, the assembly needs an explicit migration that
respects any administrator replacement; the factory-copy rule will not
silently rewrite it.

## Examples

- Nex keeps `/etc/hosts` writable and persistent because the machine owner
  controls local name resolution.
- A daemon may ship `/usr/lib/example/example.conf`, accept a temporary
  `/run/example/example.conf`, and let `/etc/example/example.conf` replace
  both.
- An OpenCL loader built by Nex can merge ICD files from administrator,
  temporary, and vendor directories. Driver packages then install their ICD
  files below `/usr` instead of `/etc`.
- Nex builds the certificate trust cache below `/run/ssl/certs` from trust
  inputs under `/etc`, `/run`, and `/usr`. A stable `/etc/ssl/certs` link keeps
  programs that expect the traditional path working.

These rules keep package artifacts reusable, let machines retain real local
choices, and let Nex replace or roll back the complete operating system without
merging configuration files.
