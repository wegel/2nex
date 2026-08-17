# Package and assembly manifest style

## Keep package manifests reusable

A package manifest must describe a normal upstream package. Do not encode a
particular product, assembly, distribution, Yocto recipe, image layout, or
vendor root filesystem in a reusable package manifest. Put product choices in
the product repository and assembly choices under `asm/`.

Use upstream build switches and normal filesystem paths when the package
supports them. If Nex needs to change upstream source, keep the change narrow
and write it so another assembly can use the same package.

Package manifests maintained by the Nex distribution must not install vendor
defaults or integration fragments below `/etc`. Do not disguise them below
`/usr/etc`. Install immutable files below `/usr/lib` or `/usr/share`, and patch
or configure Nex-built readers to honor `/etc`, `/run`, and `/usr` in that
priority. An assembly that adds an unpatched outside program owns any fixed
`/etc` compatibility path that program needs.

These lookup patches do not apply under `pkg/bootstrap/`. Leave those
seeds on their upstream readers so the bootstrap stays visibly
standard. A missing `/run` or `/usr` tier in a bootstrap program is
not a reason to patch that seed. The shipped copy of the same program
under another `pkg/` tree is the one that must honor the three tiers.

Autotools packages need an explicit `--sysconfdir=/etc`. Autoconf defaults
`sysconfdir` to `${prefix}/etc`, and unlike Meson it does not special-case a
`/usr` prefix, so `./configure --prefix=/usr` alone compiles `/usr/etc/...`
into every reader that uses that directory. That path is banned above, and the
machine's real `/etc` then goes unread. GnuPG shipped this way: its components
looked for `/usr/etc/gnupg` while the `applygnupgdefaults` tool it installs
refused to run without `/etc/gnupg/gpgconf.conf`. A package whose readers never
use `sysconfdir` is unaffected, so pass the flag or confirm with `strings` on a
built binary that no `/usr/etc` path is compiled in.

`scripts/check-package-config-paths.sh` cannot catch that, because it inspects
declared output paths and a compiled reader path is not an output. To sweep the
whole store for the mistake:

```sh
grep -rhoa --binary-files=text -E '/usr/etc/[A-Za-z0-9_.+-]+' .zub/objects/blobs
```

Hits include historical builds and documentation, so check each one against the
current `<pkg>/<version>/files` ref before treating it as a defect.

Run `scripts/check-package-config-paths.sh` after changing package outputs.
Literal `/etc` paths remain valid in build-time reader tests; the check inspects
only declared package output paths.

## Source inputs

Each source entry must set exactly one primary field: `url`, `file`, `dev`,
`cargo_lock`, `go_sum`, or `zig_zon`. `cargo_toml` may accompany
`cargo_lock`. Every source except `dev` must have a lowercase SHA-256.

Use `url:` when an authoritative host exposes immutable bytes and should keep
them at least as long as the package archive. Use `file:` when Nex created or
changed the input, when a URL follows a moving branch or review request, or
when the host offers weaker retention than the package archive.

A `file:` path must stay relative to the Git repository that owns its manifest.
Git must track the file. Do not use an absolute path or `..`. `dev:` is the
explicit local-development exception and must not appear in a pinned package.

## Patches

A patch is an ordinary source input. Do not add patch status or patch order to
the manifest schema. The build script applies patches in the required order and
chooses the working directory and strip level.

Keep an authoritative patch at its immutable online URL when the provider
should retain it as long as the target archive. Keep a patch in Git when Nex
authored, rebased, edited, or combined it, or when its online location can
disappear sooner. A local patch header must state:

- a concise `Subject`
- `Source`, with the original URL or commit when one exists
- `Upstream`, with a submission URL, accepted commit, or a short reason why no
  submission exists
- enough prose to explain the problem and the chosen change

Prefer an existing `git format-patch` header when Nex applies an upstream patch
without edits. Preserve upstream authorship and commit messages.

Apply textual patches without prompts or fuzzy context matching:

```sh
patch --batch --fuzz=0 -Np1 -i "/${SOURCE_fix}"
```

Use the strip level required by the patch. Do not hide failed hunks. Convert a
substantial source-code edit from `sed` into a patch so a reviewer can inspect
the complete change. A small substitution that supplies a build path or
version may remain in the build script.

After changing a patch, update its source SHA-256, run `nex check`, run the
strict two-build package check, and exercise the behavior that the patch
changes. A successful patch command alone does not prove that the package
works.
