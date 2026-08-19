# Agent Workflow

## Durable Agent Instructions

Nex keeps shared agent rules, ExecPlans, and knowledge under the tracked
`.agents/` tree. Carlos reads `.agents/ralph-prompt.md`; coding agents read
`AGENTS.md`, `RUST_CODE_STYLE.md`, `.agents/MANIFESTS_CODE_STYLE.md`, and
`.agents/TESTING.md` as the changed files require.

Nex removed the tracked `.claude/` tree on 2026-08-14. Its package and assembly
facts had already moved into `package-manifests.md` and
`system-assemblies.md`. `RUST_CODE_STYLE.md` supersedes its short coding note,
and `AGENTS.md` supersedes its commit note. The old Claude commands only asked
an agent to suggest more Claude files or write free-standing notes, so the
Ralph knowledge protocol replaces them.

Keep Carlos runtime files out of Git. `.agents/.gitignore` excludes
`bash_history`, `waiting/`, `local/`, `SCRATCH_KNOWLEDGE.md`, and paused local
plans. Durable plan history belongs under `.agents/execplans/done/`, while
verified reusable facts belong under `.agents/knowledge/`.

## Ralph Worktree Pre-Task

Before Ralph starts or resumes a selected ExecPlan, inspect the dirty worktree
and decide which existing changes already make sense as checked commits.

Use:

```bash
git status --short --untracked-files=all
git diff --stat
git check-ignore -v <path>
```

Group commit candidates by coherent behavior, run the matching integrated
checks, and stage exact paths. Do not commit ignored files, local settings,
scratch files, half-finished work, or changes that cannot be checked yet.

Evidence: The human requested this pre-task on 2026-06-28. The first pre-task
run committed `.gitignore`, the tracked package-manifest agent note, root
guidance docs, and `OSTREEFY_REPLICATION_REPORT.md`, while leaving local
settings, stale TODOs, unchecked assembly/script changes, and unchecked CLI
deployment work unstaged.

## Disposable Workdirs

Use `.agents/cleanup-workdirs.sh` for disposable `tmp/`, `.nex/tmp/`, and
similar smoke or inspection roots. Add new disposable paths to that script and
run the script instead of issuing ad hoc cleanup commands.

Do not manually remove builder-managed package or assembly build directories
unless a specific failure requires it. Reproducible builds can usually restart
from scratch and let the builder manage its own temporary paths.

Evidence: During ExecPlan `002e`, the human corrected direct cleanup with
"supposed to use a script for such cleanup". The Loupe assembly smoke added
`.nex/tmp/desktop-vwl-loupe-smoke` to `.agents/cleanup-workdirs.sh`, then ran
the script before checking out `systems/desktop-vwl/0.0.1`.

## Sandbox Tools

Agents may install missing host-side tools in the sandbox when those tools help
inspect logs, parse data, run checks, or debug a repo problem. Do not treat a
missing helper such as Ruby, Python packages, QEMU display backends, or other
developer utilities as a blocker while the sandbox can install it.

This permission applies to the agent sandbox. It does not change Nex package
rules: package build-time dependencies still belong explicitly in manifests,
and runtime dependencies still come from installed-file analysis.

Evidence: On 2026-07-01, after a checksum audit needed a YAML parser, the human
said, "You can always install whatever you jeed in this sandbox; record this
knowledge."

## Supervising background workers

A worker launched as `claude --print ... &` does not survive the supervising
session being torn down, and neither do its builds. Launch with
`setsid nohup ... < /dev/null &` so the worker outlives the session, and treat
any long build the worker starts as needing the same treatment.

Capture the worker's PID with `pgrep -x claude` and read
`/proc/<pid>/cmdline` to identify it. A `pgrep -f` on the flags matches the
short-lived `sh -c` wrapper, which exits within seconds, so a
`while [ -d /proc/$p ]` wait then reports the worker finished when it had
barely started. The supervising session's own process appears in that list too;
it is the one carrying `--input-format stream-json`.

When a worker is killed mid-build, its `nex build` can survive as an orphan
with `ppid=1`. Before relaunching that same repair, check for one and kill it,
or the two builds will collide in the shared
`.nex/tmp/build_rootfs_<pkg>_<ns>` directory and race on the manifest's
checksum. Two workers given the same assignment in one worktree will also
write to the same output JSON.

A worker killed mid-task often leaves usable work. Check the manifest's
`checksum:` against `HEAD` before redoing anything: if it changed, a build
completed and the evidence can be verified directly from the logs rather than
paid for twice. iproute2 and libxml2 were both recovered that way.

Some workers return their report through a plan file under `~/.claude/plans/`
and leave only a one-line pointer in the result field, so a very short result
is not evidence of a failed batch.

## Editing a large records file safely

Replacing one record by slicing between its heading and "the next heading"
deletes every record in between if they are not actually adjacent. This
happened three times in EP016, losing the `readline`, `attr`, and `libtirpc`
records, each restored from `HEAD` afterwards.

Compute the real next heading rather than assuming which one follows, and run
the file's own checker immediately after every rewrite. Gate the commit on that
checker with `&&` so a failure blocks it: running the check beside the commit
rather than before it is what let one of those deletions reach a commit.
