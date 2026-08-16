# AGENTS.md

This file tells Carlos and other coding agents how to work in `nex`.

Read and follow `RUST_CODE_STYLE.md` before Rust changes. Read and
follow `.agents/MANIFESTS_CODE_STYLE.md` before package or assembly manifest
changes. Read and follow `.agents/TESTING.md` before any change that affects
code, packages, assemblies, boot behavior, or user-visible results. Changes
that touch several areas must follow every relevant file.

## Project

`nex` builds a source-bootstrappable Linux system from YAML manifests and shell
scripts. Package manifests under `pkg/` build individual programs and
libraries. Assembly manifests under `asm/` combine built packages into
bootable systems or root filesystems.

Git stores the manifests and scripts that define the system. The zub store
holds build outputs as a cache. Treat Git as the source of truth and the store
as disposable.

## Operating Modes

Carlos has two modes in this repository.

`Discussion Mode` is the normal interactive mode. The human asks for work,
inspects choices, and can steer at any time.

`Ralph Mode` starts when a human runs `carlos` from the repository root and
presses Ctrl+R. Carlos then loads `.agents/ralph-prompt.md`. In Ralph Mode,
the agent selects the lowest-numbered incomplete ExecPlan under
`.agents/execplans/` and works without routine prompts. Active ExecPlans use a
three-digit sequence and a descriptive name, such as
`.agents/execplans/001-ostreefy-package-replication.md`.

Ralph usually develops packages, assemblies, and supporting CLI behavior
autonomously. Some ExecPlans may target docs, test tools, developer workflow,
repo cleanup, or another concrete repo goal. Ralph works on one ExecPlan per
run. Carlos submits another turn whenever Ralph stops without a terminal
marker, so Ralph continues until it reaches a hard blocker or the human review
gate.

Before Ralph starts or resumes the selected ExecPlan, Ralph performs a
worktree pre-task. Ralph runs `git status --short --untracked-files=all`,
inspects dirty and untracked files, separates ignored/local-only files from
commit candidates, and groups commit candidates by coherent behavior. Ralph
then commits any group that already makes sense as a checked, buildable commit.
Ralph runs the matching integrated checks before each pre-task commit. Ralph
does not commit ignored files, local settings, scratch files, half-finished
work, or unrelated human edits that cannot be checked yet. Ralph records the
decision in the active ExecPlan when the dirty tree affects that plan.

## Ralph Rules

Ralph does not start substantial work without an ExecPlan. An ExecPlan is a
self-contained Markdown file that tells a new reader what to change, why it
matters, how to verify it, and what the agent has already learned.

Ralph keeps these sections current in every active ExecPlan:

- `Progress`
- `Surprises & Discoveries`
- `Decision Log`
- `Outcomes & Retrospective`

At the start of each ExecPlan, Ralph lists the files under
`.agents/knowledge/` and reads any notes whose themes match the active work.
Ralph can use `semeja`, `rg`, or `grep` to search those notes.

During each ExecPlan, Ralph keeps `.agents/SCRATCH_KNOWLEDGE.md` current with
candidate durable facts about how to work in this repo, how to test, where
files live, how packages and assemblies interact, and what commands or failures
mean. Ralph records more than seems necessary, then corrects or removes scratch
notes when later evidence proves them wrong.

Before moving an ExecPlan to `.agents/execplans/done/`, Ralph rereads
`.agents/SCRATCH_KNOWLEDGE.md` and promotes verified durable facts into one or
more theme files under `.agents/knowledge/<theme>.md`. Ralph creates a theme
file when no existing file fits, and does not promote temporary, disproven, or
task-only notes.

Before the human review gate, Ralph may stop only for a hard blocker:

- a required file, secret, service, or human decision is missing
- two authoritative repo documents give conflicting instructions
- the active ExecPlan allows multiple incompatible designs and does not choose one
- continuing would violate the active ExecPlan or a repo invariant

When blocked, Ralph states the blocker, states what decision or input is
needed, prints `@@BLOCKED@@` on its own line, and stops.

When an ExecPlan meets every acceptance criterion and all internal checks
pass, Ralph leaves the plan in `.agents/execplans/`, records the final checks
in its `Completion Check`, and gives the complete recap required by
`.agents/PLANS.md`. Ralph asks the human to validate the result and approve
the plan's end, prints `@@AWAITING_HUMAN@@` on its own line, and stops. A
generic request to continue does not approve the plan's end.

One explicit end approval authorizes Ralph to move the unchanged plan to
`.agents/execplans/done/` and commit that move. Ralph does not start the next
plan during the same run. It prints `@@AWAITING_HUMAN@@` when another active
plan remains or `@@COMPLETE@@` when none remains. If Ralph starts with no
active plan, it prints `@@COMPLETE@@` and stops.

Carlos watches all three markers. It exits Ralph Mode on
`@@AWAITING_HUMAN@@`, `@@BLOCKED@@`, or `@@COMPLETE@@`. A Ralph turn without
one of those markers is unfinished, so Carlos asks the same agent to continue.

## Integrated Commit Checks

This repo does not use reviewer agents. Each commit carries its own quality
gate.

Every commit must be a useful `git bisect` point. Code must compile, manifests
must pass their checks, and the behavior changed by the commit must work. Do
not commit a knowingly broken intermediate state. When one working change
requires edits across several files, keep those edits in one commit rather than
committing a broken half first.

Before every commit, run the checks that match the files staged for that
commit. Record the commands and results in the active ExecPlan when Ralph is
working from a plan.

Compilation, `nex check`, and file-existence checks are not sufficient proof on
their own. Run a test that exercises the changed behavior and can fail when that
behavior is broken. If the repository lacks such a test, add or improve one in
the same commit. Follow `.agents/TESTING.md`.

For Rust code, run the relevant Rust check or test command. For manifest work,
run `nex check` on each changed manifest. For package build work, run the
strict package build command named by the active ExecPlan or handoff note.
For assembly work, build the assembly and test the resulting root filesystem or
boot path that the ExecPlan names. For any other work, run the check named by
the ExecPlan and make sure the check proves the changed behavior or artifact.

Do not commit code, manifests, or behavior changes when a matching check fails.
Fix the failure first, or record the hard blocker if the failure needs human
input. A documentation-only commit does not need a compile check, but its paths,
commands, and claims must match the repository.

## Command Output

Commands that may print long build, assembly, or test output must write their
full output to a file. Ralph inspects the command's exit status and searches the
log for the small set of facts needed for the active plan, such as checksums,
pass markers, or error lines. Ralph reads larger excerpts only when a command
fails and the extra context helps diagnose the failure.

Choose a log path under `.nex/tmp/` or another ignored work directory. Do not
add command logs to Git. Keep short commands and intentionally concise test
summaries visible when their output helps the human follow the work.

## Git Rules

Work on the current branch unless the human asks for a different branch.

Follow the subject style already used by this repository:

```text
<lowercase scope>: <lowercase imperative phrase>
```

Use an existing scope when one fits. Common scopes in this history include
`cli`, `pkg`, `pkg/kernel`, `asm`, `bloot`, `installer`, `env`, and `misc`.
Keep the subject concise, omit a trailing period, and describe what the commit
does. Examples from the repository's style:

- `cli: reject floating dependency refs`
- `pkg: add chromium runtime deps`
- `asm: include pipewire in desktop image`

Keep each commit focused on one working change. Do not mix unrelated cleanup,
refactors, package updates, or documentation into the same commit. Stage
generated checksums and output metadata with the code or manifest change that
requires them.

Never use `git add -A`. Stage exact files. Never run `git push` unless the
human asks for it.

The worktree may already contain human changes. Do not revert changes you did
not make. If a human change affects the active task, work with it and call out
any risk in the ExecPlan.

At the start of Ralph Mode and before each new ExecPlan, analyze the dirty
worktree before writing plan code or manifests. Commit coherent existing work
only after the right checks pass. Leave unrelated or unsafe work unstaged and
explain why in the ExecPlan or handoff.

## Repo Checks

For Rust code in the CLI, run:

```bash
cargo test --manifest-path src/cli/Cargo.toml
```

For manifest edits, run the relevant `nex check` command, for example:

```bash
./src/cli/target/debug/nex check pkg/apps/web/chromium.yaml
```

For package build work, prefer the stricter build command used by the current
handoff notes when it applies:

```bash
./nex build <manifest> --verbose --single --check --update-checksum --force --compute-deps --record-profile --generate-outputs
```

For assembly work, build the changed assembly and run the smallest runtime
check that proves the system works. Depending on the plan, that can mean
checking binaries inside a built root filesystem with `unshare`, booting a QEMU
test script, or running another repo script that exercises the boot path.

Do not use `sudo` unless the human explicitly asks for it.

## Current Context

Read the lowest-numbered incomplete ExecPlan under `.agents/execplans/` first.
Each ExecPlan must include the context needed for its work. Project notes such
as `CURRENT_TODO.md`, `SONIQ_YOCTO_PARITY_PLAN.md`, and
`OSTREEFY_REPLICATION_REPORT.md` can supply source material for a new ExecPlan,
but they do not set Ralph's work order.

Plans below `.agents/execplans/paused/` preserve unfinished work that the human
has taken out of the active queue. Ralph does not execute a paused plan unless
the human moves it back to `.agents/execplans/` or names it explicitly.
