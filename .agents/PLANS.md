# ExecPlan Guide

An ExecPlan is a Markdown file that lets a new agent or human finish one piece
of work without prior chat context. Put active ExecPlans under
`.agents/execplans/`. Name each plan with a three-digit sequence followed by a
short, descriptive slug, such as
`.agents/execplans/001-ostreefy-package-replication.md`. Start the sequence at
`001` and increase it for each new plan. Do not reuse a number.

Ralph executes active ExecPlans in numerical order unless the human names a
different plan. Move completed ExecPlans to `.agents/execplans/done/` without
changing their filenames. Files such as `.agents/PLANS.md`,
`RUST_CODE_STYLE.md`, `.agents/MANIFESTS_CODE_STYLE.md`,
`.agents/TESTING.md`, and `.agents/ralph-prompt.md` are instructions, not
ExecPlans. Ralph should treat only files that match
`.agents/execplans/[0-9][0-9][0-9]-*.md` as active ExecPlans.

## Knowledge Notes

At the start of every ExecPlan, Ralph must list the filenames under
`.agents/knowledge/`. Those files hold durable facts from earlier work. Ralph
should read a knowledge file when its theme touches the active plan. Ralph can
use `semeja`, `rg`, or `grep` to find relevant notes.

During every ExecPlan, Ralph must keep `.agents/SCRATCH_KNOWLEDGE.md` current.
Use that file for candidate facts about how to work in this repo, how to test,
where files live, how packages and assemblies interact, and what failures or
commands mean. Record tentative notes when they may help later. Correct or
remove scratch notes when better evidence proves them wrong.

Before Ralph completes an ExecPlan, Ralph must reread
`.agents/SCRATCH_KNOWLEDGE.md` and copy verified durable facts into one or more
theme files under `.agents/knowledge/<theme>.md`. Create a theme file when no
current theme fits. Do not promote facts that remain unverified, only helped
the finished task, or later proved wrong.

## Required Shape

Each ExecPlan must explain the real outcome first. Tell the reader what someone
can do after the change and how to see it working.

For Nex package work, the plan must name the package manifest, the command that
builds it, and the command that proves the installed package works. For assembly
work, the plan must name the assembly manifest, the build command, and the
root filesystem, service, or boot check that proves the system path works.
For other work, the plan must name the changed files, the concrete outcome, and
the command or inspection that proves the outcome.

Each ExecPlan must contain these sections:

- `Purpose / Big Picture`
- `Progress`
- `Surprises & Discoveries`
- `Decision Log`
- `Outcomes & Retrospective`
- `Context and Orientation`
- `Plan of Work`
- `Concrete Steps`
- `Validation and Acceptance`
- `Idempotence and Recovery`
- `Artifacts and Notes`
- `Interfaces and Dependencies`

## Progress

The `Progress` section must use checkboxes. Add a timestamp whenever the agent
finishes a step or splits a step into done and remaining parts.

Example:

```md
- [x] (2026-06-27 15:30Z) Read the manifest and reproduced the failing check.
- [ ] Patch the manifest so `nex check` passes.
```

## Decision Log

Record every meaningful choice in this format:

```md
- Decision: Use the existing package manifest path instead of a new package name.
  Rationale: Existing assemblies already point at that path, so this keeps the
  package graph stable.
  Date/Author: 2026-06-27 / Ralph
```

## Writing Rules

Write for a reader who does not share your chat history. Define repo-specific
terms when you first use them. Name concrete files and commands. Do not say
"the usual check" when you can name the command.

Do not hide key facts in links to external material. If a fact must guide the
work, write the fact in the ExecPlan.

## Skeleton

Use this skeleton for new plans:

```md
# Short action-oriented title

This ExecPlan is a living document. Agents must keep `Progress`, `Surprises &
Discoveries`, `Decision Log`, and `Outcomes & Retrospective` current as work
proceeds.

Agents must also keep `.agents/SCRATCH_KNOWLEDGE.md` current during this plan.
At plan completion, agents must promote verified durable notes into
`.agents/knowledge/<theme>.md`.

## Purpose / Big Picture

Explain what someone gains after this change and how they can see it working.

## Progress

- [ ] First concrete step.

## Surprises & Discoveries

- Observation: None yet.
  Evidence: Not started.

## Decision Log

- Decision: None yet.
  Rationale: Not started.
  Date/Author: Not started.

## Outcomes & Retrospective

Not started.

## Context and Orientation

Name the files, commands, and concepts that matter.

## Plan of Work

Describe the edits and checks in order.

## Concrete Steps

Show exact commands with the repository root as the working directory.

## Validation and Acceptance

Say what command must pass and what behavior a human should observe. Package
plans must go beyond `nex check`: build the package and run at least one
installed command, inspect a required installed file, or run a targeted smoke
test. Assembly plans must build the assembly and prove the changed root
filesystem or boot path works. Other plans must name the check that proves the
specific outcome, such as a script run, generated file comparison, docs link
check, or CLI test.

## Idempotence and Recovery

Say how to rerun the steps and how to recover from partial work.

## Artifacts and Notes

Keep short transcripts, diffs, and findings here.

## Interfaces and Dependencies

Name the final commands, files, manifest paths, public interfaces, package
inputs, and store refs that the work creates or changes. State required
versions when they matter.
```

## Human Review Gate

Before asking the human to approve a completed ExecPlan, add a `Completion
Check` subsection under `Validation and Acceptance`. Compare the finished work
with every plan requirement, inspect the exact diff and commits, record the
commands and results that prove the outcome, and name every remaining risk or
skipped check.

The review message must stand on its own for a smart reader who did not follow
the work. It must cover:

- the original observable goal and why it mattered;
- the package, assembly, command, or workflow that now works;
- the main milestones and lasting choices across the whole plan;
- the automated checks and built-artifact exercises that prove the result;
- the exact artifact, command, or file the human should inspect;
- every remaining risk, missing external input, or skipped check; and
- the exact human checks requested before approval.

In Ralph Mode, keep the finished plan under `.agents/execplans/` until the
human explicitly approves its end. After approval, move the file unchanged to
`.agents/execplans/done/` and commit that move. Never edit an archived plan.
