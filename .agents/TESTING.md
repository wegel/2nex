# Testing and commit quality

This file defines the evidence an agent must collect before committing a
change. A successful compile or manifest parse is necessary, but it does not
prove that the changed behavior works.

## The commit rule

Every commit must leave the repository in a working state and provide a useful
`git bisect` point. Before committing, the agent must run a test that exercises
the behavior changed by that commit.

The agent must not commit when:

- the code does not compile
- a relevant test fails
- a package does not build reproducibly
- an assembly does not build
- the agent only checked that expected files exist
- the agent only watched a manual test without a machine-readable result
- the changed behavior has no test and the agent could reasonably add one

When the repository lacks a suitable test, the agent must add or improve a test
in the same commit as the behavior change.

## Choose a test that can fail

Start each task by stating what observable result would distinguish working
code from broken code. Write or select a test that checks that result.

A useful regression test should fail against the broken behavior and pass
after the fix when practical. A test that passes before and after a change may
still cover surrounding behavior, but it does not prove the fix by itself.

Do not accept these checks as sole proof:

- `cargo check`
- `cargo build`
- `nex check`
- a successful YAML parse
- `test -e`, `test -f`, or `test -x`
- checking that a process started
- checking only an exit code when the program can exit successfully without
  performing the requested work

Use those checks as early layers, then exercise the result.

## Rust and CLI changes

Run formatting, focused tests, and the broader crate test suite:

```bash
cargo fmt --manifest-path src/cli/Cargo.toml -- --check
cargo test --manifest-path src/cli/Cargo.toml <focused test name>
cargo test --manifest-path src/cli/Cargo.toml
```

Add a unit test for local logic. Add an integration test when the change crosses
the filesystem, zub store, process, manifest, deployment, or command-line
boundary. Invoke the CLI as a user would and assert its output plus resulting
state.

## Package manifests

Run `nex check`, build twice through `--check`, and exercise the installed
output.

For a command package, run one or more installed commands inside the package
capsule or a built root filesystem. Assert useful output or a state change, not
only `--help` when a stronger check exists.

For a library package, compile and run a small consumer when practical. If the
library exists only to support another package, build and run that consumer.

For a service package, start the service in an isolated root or virtual machine
and probe the service through its real interface.

For a kernel, initramfs, bootloader, or storage package, boot a virtual machine
or suitable test device. Assert the boot result, mount state, selected
deployment, and failure behavior that the commit changes.

## Assembly manifests

Build the assembly reproducibly, then boot or enter its root filesystem.
Execute the newly added commands, start relevant services, and verify the
system state that users need.

An assembly test must catch missing libraries and broken loader paths. Checking
that `/usr/bin/foo` exists does not catch those failures.

## End-to-end tests

Automated boot and installer tests must:

- run without interactive input
- enforce a finite timeout
- capture serial output and other useful logs
- wait for an explicit success condition
- fail when the success condition never appears
- verify state inside the guest when host-side logs cannot prove it
- clean up temporary processes, mounts, sockets, and images
- print the captured log on failure

A timeout exit is not success. A person seeing a login prompt is not an
automated assertion.

## Test scope and speed

Run the smallest test that directly proves the change first. Then run broader
tests when the changed code serves several packages or system paths.

Slow tests are acceptable when only a real package build, assembly build, or
virtual-machine boot can prove the behavior. Do not replace a necessary test
with a weaker test merely to make commits faster.

## Record the evidence

An active ExecPlan must record:

- every command run
- whether it passed
- the important assertion or output
- tests not run and the concrete reason

Before committing, inspect the exact staged diff and make sure every staged
file belongs to the behavior that the tests covered.
