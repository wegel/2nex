# Build sandbox

How a package build script runs, and what is and is not available inside it.
These facts come from EP016, where each one cost at least one failed build.

## The sandbox is smaller than the dependency list suggests

A dependency gives the build only the bundle it names, not the whole package.
`pkg/core/userland/coreutils.yaml` defines `bundles/dev` as `bin` plus `lib`,
while `sha256sum`, `md5sum`, `sha1sum`, and `cksum` sit in a separate
`checksum` output group. A manifest that depends on `coreutils/bundles/dev`
therefore has `cat`, `cp`, `basename`, `head`, `tail`, `timeout`, `tr`, `rm`,
`mkdir`, and `rmdir`, and also `b2sum` and `sum`, but no `sha256sum`. A build
that needs one must depend on `coreutils/outputs/checksum` or `bundles/full`.

Nothing is present unless a dependency supplies it. An mpv smoke failed with
`sed: command not found` because that manifest depends on coreutils, findutils,
tar, and gzip but not on sed. Before writing a smoke, list the tools it uses
and check them with `zub ls-tree -r <dep>/bundles/dev`.

## Check a tool explicitly; errexit will not

The launcher runs the script under `bash -o errexit -o nounset`, but a failed
command substitution inside an assignment does not stop it. After `sha256sum`
was missing, a Chromium build carried on to `[202/202] LINK gn` with a
half-computed variable, and the only trace was one line in the middle of the
log.

## The work directory does not survive between builds

`nex build` logs `Setting up composite rootfs at .nex/tmp/build...` and rebuilds
the root each invocation, so `WORK_DIR` starts empty and a stopped build loses
its compiled objects. A Chromium build stopped at 33926 of 54756 objects
restarted at `[1/202]`, re-extracted its source, and re-applied its patches,
even though neither the manifest nor the patch had changed. A re-extract marker
inside a script only prevents extracting twice within one run. Weigh the lost
compute before interrupting a long build.

## No `/dev/stdout`, and no dropping privileges

The script runs as root under
`unshare --user --pid --mount --uts --fork --ipc --net --map-root-user` and
then `unshare --root=<dir>`, which is a chroot. Two consequences:

- `/dev/stdout` does not resolve, because it is normally a symlink into
  `/proc`. A vim `redir >> /dev/stdout` produced an empty file. Redirect to a
  real path instead.
- The namespace maps exactly one id, so a smoke cannot become unprivileged.
  `setgid(65534)` fails with `EINVAL` because that id is unmapped, and
  `unshare(CLONE_NEWUSER)` fails with `EPERM` because Linux forbids a new user
  namespace from inside a chroot.

When a program refuses to run as root, interpose the guard rather than fighting
the sandbox. Looking Glass exits when `getuid() == 0`, so its manifest builds a
two-function `LD_PRELOAD` object returning 65534 from `getuid` and `geteuid`.
The configuration readers stay untouched, so the smoke still exercises the
shipped binary.

## Writing a smoke in a manifest

The build script may write real absolute paths: `/etc`, `/run`, and
`/usr/lib` are all writable inside the sandbox, which is what makes tier tests
possible at all.

A heredoc inside a `script: |` block must be indented to the block's own
indentation, terminator included. YAML strips the common indent before the
shell sees the script, so a body written at column 0 ends the block scalar and
`nex check` fails with `could not find expected ':'` pointing at the first
heredoc line. `pkg/libs/graphics/opencl-icd-loader.yaml` has the working shape.

Never pipe a program's output straight into a parser. Write it to a file under
`${WORK_DIR}`, parse the file, and print the file on mismatch. An mpv smoke
that piped into `sed` hid both the missing `sed` and the program's own error,
turning a broken smoke into an unexplained failure.

Rehearse an expensive smoke against an already-built copy first. The Chromium
probe was validated against the host browser before the build ran, which
confirmed that `--vmodule=config_dir_policy_loader=1` reaches `VLOG_POLICY`.
One Chromium compile takes hours; one vim compile takes two and a half minutes,
which is why vim could absorb two probe mistakes and Chromium could not.

## Reading what a build actually produced

`zub checkout` fails on this host with `EPERM` while creating a subdirectory,
with or without `--copy`, and leaves a partial tree. To inspect one built file,
find it with `zub ls-tree -r <ref>` and extract it with
`zub cat-file '<ref>:usr/lib/<file>' > /tmp/<file>`.

Blobs are stored uncompressed, so the whole store can be searched for a
compiled string:

    grep -rhoa --binary-files=text -E '/usr/etc/[A-Za-z0-9_.+-]+' .zub/objects/blobs

That finished over 23 GB in a few minutes. It also matches historical build
outputs and documentation, so a hit is a candidate rather than a defect: always
re-check it against the current `<pkg>/<version>/files` ref before acting.

## What the sandbox is missing when a smoke runs

EP016 wrote a behavior test into a dozen package builds, and the same handful
of sandbox gaps cost real build cycles each time. Check these before blaming
your own code.

`lo` exists but is down. samba's `SMBC_module_init()` calls
`load_interfaces()` right after loading configuration and exits when it finds
no usable interface and no `interfaces=` line. Run `ip link set lo up` first,
which means adding `iproute2` as a build dependency.

There is no useful `/etc/passwd`. libssh's `ssh_connect()` resolves a username
through `getpwuid_r()` with no environment fallback, so it fails before any
network activity. Set the username explicitly, for libssh through
`SSH_OPTIONS_USER`.

The root process keeps `CAP_DAC_OVERRIDE`. A test that makes a file unreadable
with `chmod 000` and expects an open to fail will silently succeed. fuse3's
fail-closed path was instead exercised with `ENOTDIR`, by making the directory
a plain file, which reaches the same non-`ENOENT` branch and no capability can
bypass.

`/dev/fuse` is a plain regular file, so no real FUSE mount completes. Only the
policy decisions that happen before the `mount(2)` call can be proven.

A caller-owned file descriptor is not closed for you. libssh's smoke deadlocked
because `SSH_OPTIONS_FD` hands the session a socket that `ssh_free()` leaves
open, parking the client in `waitpid()` and the forked server in `poll()`.

## Write smokes against exported symbols only

Check with `nm -D` before compiling anything that calls into the library you
just built. The enchant2 repair lost four consecutive full builds to a smoke
referencing `normalize_dictionary_tag()`, which `broker.c` defines but the
library does not export.

A symbol can be exported yet absent from the installed header, which is a
legitimate but deliberate choice to record: enchant's
`enchant_broker_get_ordered_providers()` and `enchant_get_conf_dirs()` are both
exported and both missing from `enchant.h`, so that smoke declares its own
prototypes. Driving the installed command line tool instead avoids the private
prototypes at the cost of parsing output.

## Proving a repair fails without its patch

Copy the manifest, replace only the `patch --batch ...` line with `true`, and
build that copy:

    sed 's|^\( *\)patch --batch --fuzz=0 -Np1 -i /\${SOURCE_uapi_config_patch}|\1true # omitted|' \
        <manifest> > .nex/tmp/<pkg>-unpatched.yaml
    ./nex build .nex/tmp/<pkg>-unpatched.yaml --verbose --single --check --force

Match the quoting exactly. Manifests differ: some write
`-i "/${SOURCE_...}"` with quotes and some write `-i /${SOURCE_...}` without,
and a `sed` that fails to match produces an identical copy that then builds
successfully and appears to disprove the gap. Always `diff` the copy against
the original before trusting the result.

Where a package applies more than one patch, omit only the one under test.
samba's negative proof had to keep `samba-linux-credential-probe.patch` applied.

A good negative proof fails behaviorally rather than by crashing. The ones EP016
produced read: `got "WORKGROUP", want "VENDORWG"` for samba, `got "NONE", want
"true vendor"` for libssh, `expected "table mainrun" ... got ... table
mainvendor` for iproute2, and `order: nex-uapi-other nex-uapi-want` for
enchant2.

## An interrupted `--update-checksum` run edits phase0 seeds

Killing an assembly build that carried `--update-checksum` left a new
`checksum:` line on five `pkg/bootstrap/phase0/` manifests. No phase0 manifest
carries a committed `checksum:` key, because phase zero sets
`stable_checksum: false` deliberately. Revert those lines with
`git checkout --` before committing anything else, and inspect `git status`
for stray phase0 checksums after any interrupted build.
