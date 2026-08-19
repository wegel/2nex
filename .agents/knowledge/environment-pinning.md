# Environment Pinning

## Why manifests pin environments by Git blob SHA

A package manifest names its build environment in one of two shapes:

    environment: 27b6e5dc7ad152c9a17c2cabfcc5ee93daa9bbf0   # a Git blob
    environment: env/standard.yaml                          # a working-tree path

`load_environment` (`src/cli/src/build/env.rs:14`) picks between them with
`is_sha1`. A 40-character hex value goes to `git cat-file blob` against the
repository. Anything else goes to `fs::read_to_string`.

The blob form is the correct one, and the reason is time travel.

A blob SHA travels with the manifest. Nex builds a package from a historical
manifest whenever a dependency carries a `manifest_ref`, whenever someone
checks out an older commit, and whenever the store is asked for a build that
matches a past manifest hash. In every one of those cases the manifest was
written against a specific environment, and its recorded checksum is a claim
about building in that environment. `git cat-file blob` returns the historical
content no matter where HEAD sits, so the claim stays checkable.

A path floats with the checkout. `fs::read_to_string("env/standard.yaml")`
returns whatever the working tree holds right now. Build a manifest from six
months ago while HEAD carries a newer environment, and the build runs under an
environment that manifest never saw. The output no longer matches the recorded
checksum, and nothing reports why, because the manifest looks unchanged.

The path form is worse in a second way that is easy to miss: `fs::read_to_string`
resolves against the current directory, not against the repository that owns the
manifest. The same command run from a subdirectory fails.

This is the same principle that makes `manifest_ref` pin a dependency to a
commit. Pinning is what lets a past state be rebuilt. An unpinned reference
means the build depends on when it ran.

## The 2026-08-19 divergence

476 packages pinned `27b6e5dc`, an older revision of `env/standard.yaml` that
lacks the `ldconfig` cache regeneration step. The current file hashes to
`248c5014`. Whoever added that step edited the file and did not repin, so the
fix reached only the packages that name the environment by path.

Reference counts at that time:

    27b6e5dc  standard, pre-ldconfig     476 packages
    eeeb4cd4  bootstrap-phase1            23 packages, matches the current file
    cec6d210  bootstrap-phase0            20 packages, matches the current file
    13ec31fd  assemblies                  11 manifests
    env/standard.yaml (path form)          7 packages
    env/linux-kernel.yaml (path form)      1 package

The eight packages using the path form are `looking-glass`, `kvmfr`,
`v4l2loopback`, `linux`, `wireless-regdb`, `nvidia-current`, `nvidia-580`, and
`spice-protocol`.

Repinning the 476 to `248c5014` changes 476 recorded checksums, so every one of
those packages rebuilds. Convert the path-form packages at the same time, for
the reasons above.
