# Working on Assembly Manifests

## Assembly Manifest Structure

Assemblies are complete bootable systems composed of packages.

```yaml
system:
  schema: 1
  name: Desktop System with vwl/Wayland
  slug: desktop-vwl
  version: 0.0.1
  description: Desktop system with systemd, vwl compositor, wayland tools
  checksum:                        # auto-filled by --update-checksum
  nex_structure: true

overlays:
- asm/desktop-vwl/overlay.yaml     # system configuration files

dependencies:
# build-time only tools - NOT in final rootfs, NO transitive deps
- name: glib2
  commit: x86_64/pkg/dev/libs/glib2/2.80.2/bundles/dev

packages:
# go into the final target rootfs - PULLS transitive deps automatically
- name: systemd
  commit: x86_64/pkg/core/init/systemd/257.5/bundles/minimal

build:
  environment: <env-hash>
  script: |
    # runs with dependencies + packages available
```

## Dependencies vs Packages

| Aspect | dependencies | packages |
|--------|--------------|----------|
| In final rootfs | NO | YES |
| Transitive deps | NOT pulled | Pulled automatically |
| Purpose | Build-time tools | Runtime software |

**dependencies**: Build-time tools needed to configure the target. They do NOT end up in the final rootfs and do NOT pull their transitive dependencies. Use for things like `glib-compile-schemas`, `localedef`, etc.

**packages**: Software for the final system. They DO end up in the target rootfs and their transitive dependencies ARE pulled automatically.

```yaml
dependencies:
# build tools - won't be in final image, deps not pulled
- name: glib2
  commit: x86_64/pkg/dev/libs/glib2/2.80.2/bundles/dev  # for glib-compile-schemas

packages:
# these + their transitive deps go in the final rootfs
- name: networkmanager
  commit: x86_64/pkg/net/manager/networkmanager/1.50.0/bundles/dev
  # automatically pulls: glibc, dbus, newt, slang, popt, etc.
```

## Adding Packages

Just add the package ref. Don't manually add its dependencies:

```yaml
packages:
- name: networkmanager
  commit: x86_64/pkg/net/manager/networkmanager/1.50.0/bundles/dev
# DON'T manually add slang, popt, newt - they're pulled automatically
```

## Overlays

Overlays provide configuration files that don't come from packages:

```yaml
overlays:
- asm/desktop-vwl/overlay.yaml
```

Overlay files typically include:
- `/etc/passwd`, `/etc/group`, `/etc/shadow`
- `/etc/fstab`, `/etc/hostname`
- systemd unit overrides
- default user configs

## Build Script

Runs after packages are assembled. Both `dependencies` and `packages` are available, but only `packages` (and their transitive deps) end up in `/target`.

```yaml
build:
  script: |
    set -eux

    # create directories (mount points, runtime dirs - can't be in overlay)
    mkdir -p /target/{proc,sys,dev,run,tmp,root}
    mkdir -p /target/var/{log/journal,lib/systemd}

    # generate locales (localedef from glibc, which is in packages)
    mkdir -p /target/usr/lib/locale
    localedef -i en_US -c -f UTF-8 /target/usr/lib/locale/en_US.UTF-8

    # compile gsettings schemas (glib-compile-schemas from dependencies)
    glib-compile-schemas /target/usr/share/glib-2.0/schemas/
```

## Building an Assembly

```bash
./nex build asm/<name>/manifest.yaml --update-checksum
```

## Testing Assemblies

**Do not attempt to deploy or test assemblies yourself.** After building, ask the user to test the assembly and report back with any issues. The user has their own deployment and testing workflow.

## Finding Package Refs

```bash
zub refs | grep networkmanager | grep bundles
zub ls-tree -r x86_64/pkg/net/manager/networkmanager/1.50.0/bundles/dev
```
