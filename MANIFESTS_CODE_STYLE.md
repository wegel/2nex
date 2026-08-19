# Manifest style and quality bar

This file defines how Nex package manifests under `pkg/` and assembly manifests
under `asm/` must read. A package manifest builds one program or library. An
assembly manifest combines package outputs into one root filesystem.

The Nex formatter in `src/cli/src/manifest/format.rs` should enforce these
rules. Agents must inspect every formatter diff because a formatter bug must
not remove a supported field, a required comment, or package behavior.

## 1. General YAML shape

- Use two spaces for mappings. Never use tabs.
- Put sequence dashes at the indentation level of the sequence key's value.
- Put one space after `:` and after `-`.
- Remove trailing whitespace.
- End every file with one newline.
- Put one blank line between top-level sections.
- Do not put repeated blank lines inside a section.
- Keep one blank line between named entries under `bundles` and `outputs`.
- Use UTF-8 only when a package name or upstream value requires it.
- Do not use YAML anchors, aliases, flow mappings, or implicit merge keys.
- Use flow sequences only for short generated values such as `build.profile`.

Use comments to explain why a package, flag, dependency, or output differs from
the upstream default. Do not narrate fields that already explain themselves.
Put comments on their own line above the item they explain. Keep comments when
formatting a manifest.

## 2. Package section order

A package manifest must use this top-level order:

1. `package`
2. `sources`
3. `dependencies`
4. `build`
5. `bundles`
6. `outputs`
7. `resolution`

Omit an optional section instead of writing an empty mapping. Write an empty
source or dependency list as `sources: []` or `dependencies: []` only when the
schema requires the key.

Fields under `package` must use this order:

1. `schema`
2. `name`
3. `slug`
4. `namespace`
5. `version`
6. `description`
7. `homepage`
8. `checksum`
9. `stable_checksum`
10. `seed`

Keep `name` readable for humans. Keep `slug` and `namespace` stable because
store refs and other manifests use them. Pin `version`, source hashes, and the
package output checksum. A manifest with `stable_checksum: false` must explain
why its output cannot have a stable checksum.

## 3. Where an assembly manifest lives

Four directories hold assembly manifests, and which one a manifest belongs in
follows from what it is:

- `base/` holds layers that other assemblies extend and that nobody runs
  directly, such as `flat-systemd.yaml` and `nex-systemd.yaml`. A manifest here
  must be reusable and must carry no site, product, or machine detail.
- `examples/` holds complete systems that Nex ships as de-branded patterns,
  such as `edgebox-rootfs.yaml` and `desktop-vwl/desktop-vwl.yaml`. They are
  built and run, and other assemblies may extend them.
  `scripts/check-generic-assembly-policy.sh` enforces that they stay generic.
- `installer/` holds the installer, which is neither a layer nor an example.
- `asm/` is not Nex's. It is ignored by Git and is where the person using this
  repository keeps assemblies for their own machines, either as loose files or
  as a checkout of their own repository.

Nex ships nothing in `asm/`. An assembly that names a specific graphics card,
network, or user belongs there or in the owner's own repository, never in
`base/` or `examples/`.

Reference another assembly with a repository-qualified path, `nex:` followed by
a path from the Nex repository root:

```yaml
system:
  extends: nex:base/nex-systemd.yaml
```

The `nex:` prefix resolves against the upstream Nex repository when the manifest
lives in a product repository that carries Nex as an `upstream/nex` submodule,
and against the owning repository when it does not. A plain relative path still
resolves against the repository that owns the manifest.

## 4. Assembly section order

An assembly manifest must use this top-level order:

1. `system`
2. `sources`
3. `dependencies`
4. `packages`
5. `providers`
6. `exclude`
7. `files`
8. `build`

Fields under `system` must use this order:

1. `schema`
2. `name`
3. `slug`
4. `version`
5. `architecture`
6. `boot_method`
7. `description`
8. `nex_structure`
9. `extends`
10. `stable_checksum`
11. `checksum`

Use `extends` for an assembly that adds to another assembly. Use `exclude` only
when the child must remove a named package or dependency from its parent. Put
one blank line before the generated `checksum` field when the header also
contains `nex_structure` or `extends`.

Use `files` for the literal files, symlinks, and directories the assembly places
in the built root. Each entry uses the field order `path`, `mode`, `content`,
`source`, `symlink`, `directory`, `replace`. A `source` path is relative to the
repository root that owns the manifest. A child assembly's entries apply after
its parent's entries.

Use `providers` when an assembly binds an abstract runtime capability to a
specific package output or bundle. Sort capability names alphabetically. The
value must be an exact output or bundle ref from a normal package manifest:

```yaml
providers:
  graphics.egl: x86_64/pkg/libs/graphics/mesa/24.2.7/outputs/graphics-runtime
```

## 5. Sources

Keep source entries in the order the build script consumes them. Fields in each
entry must use this order:

1. `name`
2. `url`
3. `file`
4. `dev`
5. `sha256`
6. `cargo_lock`
7. `cargo_toml`
8. `go_sum`
9. `zig_zon`

Use one of `url`, `file`, or `dev` for the main source location. Pin every
download with `sha256`. A local `dev` source may omit `sha256`. Prefer an
upstream release archive over a moving branch or an unpinned generated file.

## 6. Dependencies and packages

Each dependency entry must put `name` before `commit`, then `manifest_ref` when
the schema needs it:

```yaml
dependencies:
- name: glibc
  commit: x86_64/pkg/libs/system/glibc/2.39/outputs/lib
```

Do not sort an existing dependency list mechanically. Package layering and
build scripts can make list order meaningful. Put a new dependency beside
related dependencies. When writing a new manifest, copy the dependency order
from the nearest package that uses the same language and build system, then add
package-specific libraries in the order the build introduces them.

Each assembly package entry must put `name` before `commit`:

```yaml
packages:
- name: bash
  commit: x86_64/pkg/cli/shells/bash/5.2.21/outputs/bin
```

Group assembly packages by the job they perform, such as kernel, networking,
audio, or developer tools. Put one short comment above each group. Within a
group, keep packages in the order the root filesystem should layer them. Use
the smallest output or bundle that supplies the files the running system needs.
Do not install a development bundle merely because it is convenient.

## 7. Build section and shell

Fields under `build` must use this order:

1. `environment`
2. `profile`
3. `script`

Put one blank line before `script` when `environment` or `profile` precedes it:

```yaml
build:
  environment: 27b6e5dc7ad152c9a17c2cabfcc5ee93daa9bbf0
  profile: [4097:848, 8264:3501]

  script: |
    set -eu
    cd "${WORK_DIR}"
```

Write embedded shell so the first failed command stops the build. Use `set -eu`
unless the selected shell supports and needs `pipefail`. Quote paths and
variable expansions unless word splitting is intentional. Use braces for
variables next to text, such as `"${OUT_DIR}/usr/lib"`. Keep environment
variables next to the command that consumes them.

Build scripts must not call `sudo`. They must not fetch unpinned network
content. They must normalize timestamps, archive ordering, generated metadata,
and other host-dependent output when a build tool does not already do so. Do
not hide failures with `|| true` unless the command is genuinely optional and
a nearby comment names the accepted failure.

An assembly that inherits its parent's build script may use `script: ""`. Do
not use an empty script for a package manifest.

## 8. Bundles

Sort bundle names alphabetically. Use the compact sequence form established by
current package manifests:

```yaml
bundles:
  dev:
  - dev
  - lib

  full:
  - bin
  - dev
  - lib
```

Sort included output names alphabetically. A bundle must contain only outputs
that belong together for a concrete build or runtime use.

## 9. Outputs

Sort output names alphabetically. Put one blank line between outputs. Each
output must contain a non-empty `files` list, and each file entry must put
`path` before `needs`. Put output `provides` before `capability_files`, and put
both before `files`, when an output supplies a runtime capability. Use
`capability_files` when a broad output provides several capabilities but an
assembly should flatten only the files for one capability:

```yaml
outputs:
  bin:
    files:
    - path: /usr/bin/example
      needs:
      - /usr/lib/ld-linux-x86-64.so.2
      - /usr/lib/libc.so.6

  graphics-runtime:
    provides:
    - graphics.egl
    capability_files:
      graphics.egl:
      - /usr/lib/libEGL.so.1
    files:
    - path: /usr/lib/libEGL.so.1

  lib:
    files:
    - path: /usr/lib/libexample.so.1
```

Sort file paths with natural numeric order. Sort each `needs` list the same
way. Use absolute installed paths. Do not list a directory when the output
contains individual files that Nex can record. Remove empty generated outputs
instead of keeping an output name with no files.

Keep generated output lists current. Run the package build with
`--generate-outputs` when installed files change. Do not weaken a
reproducibility check to keep stale output metadata.

## 10. Resolution

`resolution` maps each required installed file to the package that supplies it.
Use `self` when the same package supplies the file. A generated graphics entry
may map a file to a capability with a concrete package fallback for
package-level build and check contexts:

```yaml
resolution:
  /usr/lib/libEGL.so.1:
    capability: graphics.egl
    fallback: mesa
```

Group string entries by provider, sort provider names alphabetically, and sort
paths naturally within each provider group. Put nested capability entries after
string entries and sort them naturally by path.

Do not guess a provider. Build the dependency first and let `nex compute-deps`
derive the map. Review the generated map before committing it.

## 11. Scalars and quoting

Leave ordinary names, paths, URLs, checksums, and store refs unquoted. Use
double quotes when YAML could parse a string as another type or when the value
contains a comment marker, leading or trailing whitespace, or `: `.

Quote a version that consists only of numbers and dots:

```yaml
version: "3.10"
```

Do not quote a version that YAML already treats as a plain string:

```yaml
version: 6.12.58
version: 0.40.0+20251205
```

## 12. Formatter and checks

Run these commands from the repository root for every changed manifest:

```bash
./nex format path/to/manifest.yaml
git diff -- path/to/manifest.yaml
./nex check path/to/manifest.yaml
```

The formatter diff must preserve every supported field and useful comment. If
the formatter removes `extends`, `files`, `exclude`, or another schema field,
fix `src/cli/src/manifest/format.rs` before accepting the formatted file. Never
delete valid manifest behavior merely to make `nex check` pass.

For a package manifest, run the strict build command required by `AGENTS.md`
and prove that at least one installed command or required file works. For an
assembly manifest, build the assembly and inspect its root filesystem, service,
or boot path. A YAML parse check alone does not prove a manifest works.
