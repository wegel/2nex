# Ralph Mode reference

Carlos activates Ralph Mode when a human runs `carlos` from the Nex repository
root and presses Ctrl+R. Carlos reads `.agents/ralph-prompt.md` and submits it
to the coding agent. Discussion Mode remains the normal interactive mode.

## 1. How Ralph chooses work

Ralph reads `AGENTS.md`, `.agents/PLANS.md`, and the active plan. Active plans
match `.agents/execplans/[0-9][0-9][0-9]-*.md`. Ralph selects the
lowest-numbered plan unless the human names another one.

Files below `.agents/execplans/done/` record finished plans. Files below
`.agents/execplans/paused/` preserve unfinished work that the human removed
from the active queue. Ralph ignores both directories when choosing work.

Before Ralph changes a package, assembly, script, or Rust source, Ralph runs
the worktree pre-task from `AGENTS.md`. Ralph separates human edits and local
files from checked commit candidates and records any relevant result in the
active plan.

## 2. How Carlos keeps Ralph working

Ralph works on one ExecPlan without asking for routine confirmation. Carlos
submits a continuation turn when Ralph ends a turn without a terminal marker.
Ralph resumes from the current plan, commits, test results, and worktree.

Ralph keeps the plan's `Progress`, `Surprises & Discoveries`, `Decision Log`,
and `Outcomes & Retrospective` sections accurate. Chat updates remain brief
and do not pause the work.

Ralph starts with the smallest check that can disprove the current change.
Package work then runs `nex check`, the strict two-build package command, and
a smoke test against the built files. Assembly work builds the assembly and
tests the resulting root filesystem, service, installer, or QEMU boot path.

## 3. When Ralph may stop early

Before the human review gate, Ralph stops only when one of these facts prevents
safe work:

1. The plan requires a credential, file, service, permission, or physical
   action that Ralph cannot obtain.
2. Two authoritative repository files require incompatible behavior.
3. The plan permits incompatible designs and repository evidence cannot choose
   one safely.
4. The next step would violate the plan or a repository invariant.

Ralph names the blocker and the exact input needed, prints `@@BLOCKED@@`, and
stops. A difficult build, an unfamiliar package, a failed test, or a large
amount of remaining work does not by itself qualify as a blocker.

## 4. How Ralph records knowledge

At the start of the plan, Ralph lists `.agents/knowledge/` and reads notes that
match the work. Ralph writes candidate facts to the ignored
`.agents/SCRATCH_KNOWLEDGE.md` while investigating. Each fact names evidence
such as a command, source path, built checksum, or observed failure.

Before the human review gate, Ralph moves verified reusable facts into a theme
file below `.agents/knowledge/`. Ralph leaves temporary, disproved, and
task-only notes out of those durable files. `.agents/kb.md` remains the broad
cross-project notebook requested for package replication and UAPI work.

## 5. How the human review gate works

After every acceptance criterion and internal check passes, Ralph performs the
self-check required by `.agents/PLANS.md`. Ralph records the exact proof under
`Validation and Acceptance` in a `Completion Check` subsection.

Ralph leaves the completed plan in `.agents/execplans/`, gives a stand-alone
recap of the whole plan, asks the human to validate the result and approve the
plan's end, prints `@@AWAITING_HUMAN@@`, and stops. Ralph does not start the
next plan or archive the current plan while it waits.

A generic request to continue does not approve the plan. The human must name
the completed plan or clearly answer Ralph's approval request.

One explicit approval authorizes Ralph to move the unchanged plan to
`.agents/execplans/done/` and commit that move. Ralph then prints
`@@AWAITING_HUMAN@@` when another active plan remains or `@@COMPLETE@@` when
none remains.

## 6. Commits and checks

Ralph works on the current branch unless the human chooses another branch.
Ralph stages explicit paths and never uses `git add -A` or `git add .`. Every
product commit must compile, pass the relevant manifest checks, and exercise
the changed behavior. `AGENTS.md`, `.agents/TESTING.md`, and the active plan
name the required commands.

Ralph does not push unless the human authorizes a push. An earlier explicit
request to keep the active branch pushed remains valid until the human revokes
it or changes the branch workflow.

## 7. Terminal markers

Carlos recognizes these markers on their own lines:

- `@@AWAITING_HUMAN@@`: the finished plan awaits approval, or an archived plan
  awaits a request to start the next plan.
- `@@BLOCKED@@`: one hard blocker prevents safe work.
- `@@COMPLETE@@`: no active ExecPlan remains.

Carlos exits Ralph Mode when it sees any terminal marker. A Ralph turn without
a marker remains active, so Carlos submits the continuation prompt.
