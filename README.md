# nex

I'm doing a (free) reproducible build system/distro (just a hobby, won't be big and professional like Guix or Nix) for Linux.

Jokes aside: I love the idea of an immutable, reproducible OS, but not the "strange" build/configuration languages that come with Nix and Guix. Build scripts are yaml and shell here. I also don't buy the Holy Full Source Bootstrap and its [357-byte root program](https://guix.gnu.org/en/blog/2023/the-full-source-bootstrap-building-from-source-all-the-way-down/): whatever route you take, you end up trusting the source of the normal GNU toolchain, so nex bootstraps from whatever host toolchain you have and rebuilds it with itself until the outputs **converge** to host-independent checksums. Converge from N diverse hosts (different compilers, versions, distros) and a trusting-trust backdoor would need to survive every lineage and produce identical poisoned bytes from all of them. *No binary is trusted that N diverse lineages did not independently reproduce.* Same trust boundary, minus years of ceremony.

## A package

```yaml
package:
  slug: libxcrypt
  namespace: libs/system
  version: 4.4.37
  checksum: sha256:fb8aece…      # written by tooling, sealed at merge

sources:
- url: https://…/libxcrypt-4.4.37.tar.xz
  sha256: 8a5f42a3…

dependencies:
- manifest: blob:3f2a91c…        # git blob ID = the dependency's identity
  path: pkg/libs/system/glibc.yaml
  use: bundle:dev

build:
  environment: blob:88c02e…
  script: |
    ./configure --prefix=/usr
    make && make DESTDIR="${OUT_DIR}" install
```

The repo is git (SHA-256 object format). A manifest's identity is its blob; since a dependency's blob covers *its* dependencies' blobs, a manifest transitively pins its entire input closure down to the bootstrap. The `checksum` binds that closure to the build's output:

```
plan     = H(manifest without the checksum field)     # computable before building
checksum = H(plan ‖ output)                           # same checksum ⇒ same build, forever
```

Maintainer builds locally and commits the checksum; CI rebuilds and must reproduce it. **Merge is the seal**: every merge is a two-builder reproducibility proof, and git history is the trust root. Binary stores (`zub`, think ostree for build outputs) are untrusted caches:

```
$ nex build pkg/libs/system/libxcrypt.yaml    # no cache? same bytes, built locally
```

## A system

```yaml
assembly:
  slug: nex-systemd
  checksum: sha256:0be371…       # H(plan ‖ boot tree hash): same model, sealed the same way

members:
- manifest: blob:9dd1c04…
  path: pkg/core/init/systemd.yaml
  use: bundle:minimal

substitutions:                    # CVE fix under existing binaries, no world rebuild
- needs: /usr/lib/libcrypt.so.2
  manifest: blob:aa02f1…
  reason: NEX-SA-2026-1143
  evidence: [claim:sha256:e3ab90…]
```

An assembly is a manifest one level up: compose instead of compile. Installed closures are **capsules**, hard-linked trees from the store: shared files stored once, two versions of one library coexisting in different trees. Every "runs against something it wasn't built against" edge is evidence-gated and auditable. Embedded targets flatten to a single rootfs image, a pure function of the assembly's blob:

```
$ nex compose assemblies/nex-systemd.yaml && nex deploy    # atomic swap, old tree kept
```

## Status

Early; the [spec](SPEC.md) is further along than the code. Bootstrap converges to identical checksums on my machine and a VM. That's N=2; the plan is for N to grow until the claim above means something.
