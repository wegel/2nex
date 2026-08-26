# autonomous execution prompt

Follow the agent instructions loaded for this workspace. Read
`.agents/CODE_STYLE.md` and `.agents/PLANS.md` to orient yourself. List the
files under `.agents/knowledge/` so you know what durable notes exist. Use
`semeja`, `rg`, or `grep` to read relevant knowledge notes when the active
ExecPlan touches the same theme.

Find active ExecPlans that match `.agents/execplans/[0-9][0-9][0-9]-*.md`.
Select the lowest-numbered incomplete ExecPlan and begin or continue
execution.

Before beginning or resuming that ExecPlan, inspect dirty and untracked files,
classify them into coherent commit candidates or local-only files, run the
matching checks for each commit candidate, and commit the groups that already
make sense. Do not commit ignored files, local settings, scratch files,
half-finished work, or changes that cannot be checked yet.

## rules

- Work autonomously under the loaded agent instructions. Do not stop for
  routine confirmation.
- Prefer concrete package and assembly work. Build the package or assembly and
  test the resulting files, commands, services, or boot path.
- If the active ExecPlan targets something else, follow that plan and run the
  check that proves its concrete outcome.
- Keep the active ExecPlan current as you make progress.
- During each ExecPlan, keep `.agents/SCRATCH_KNOWLEDGE.md` current with
  anything that might become durable knowledge about how to work in this repo,
  how to test, where files live, how parts interact, and what commands or
  failures mean. Record more than you expect to keep. Correct or delete scratch
  notes when later evidence proves them wrong.
- Run the checks required by the loaded agent instructions and the active
  ExecPlan before each commit.
- Commit frequently with small, buildable commits using scoped imperative
  subjects after those checks pass.
- Stop after the current ExecPlan meets its acceptance criteria. Do not start
  another active plan during the same Ralph run.

## knowledge protocol

At the start of every ExecPlan, list `.agents/knowledge/` filenames and keep
those themes in mind while working. Read the notes that seem relevant to the
active task.

While working, write candidate lessons to `.agents/SCRATCH_KNOWLEDGE.md`.
Include evidence, such as the command, file path, or failure that supports each
note. Prefer concrete facts over guesses, but record tentative facts when they
may save a future agent time.

Before moving an ExecPlan to `.agents/execplans/done/`, reread
`.agents/SCRATCH_KNOWLEDGE.md`. Move verified durable facts into one or more
theme files under `.agents/knowledge/<theme>.md`. Create a theme file when no
existing file fits. Leave temporary, disproven, or task-only notes out of the
durable files.

## blocking protocol

If you hit a hard blocker under the loaded agent instructions, do the
following:

1. State the blocker clearly in your response.
2. State what information or decision is needed to continue.
3. Output exactly this marker on its own line: `@@BLOCKED@@`
4. Stop execution.

## completion protocol

When the current ExecPlan meets every acceptance criterion and all internal
checks pass:

1. Record the exact final checks and results in the ExecPlan's `Completion
   Check` subsection.
2. Keep the ExecPlan in `.agents/execplans/`.
3. Give a stand-alone recap of the whole ExecPlan. Cover the original goal,
   delivered behavior, main choices, proof, artifacts to inspect, and every
   remaining risk or skipped check.
4. Ask the human operator to validate the result and approve the ExecPlan's
   end.
5. Output exactly this marker on its own line: `@@AWAITING_HUMAN@@`
6. Stop execution.

Do not treat "continue", "keep working", or interruption recovery as approval
to finish a plan. The human must clearly approve the completed plan's end.

When the human approves the completed ExecPlan:

1. Move the plan unchanged to `.agents/execplans/done/` and commit that move.
2. If another active ExecPlan remains, output `@@AWAITING_HUMAN@@` and stop.
3. If no active ExecPlan remains, output `@@COMPLETE@@` and stop.

When no active ExecPlan exists and the worktree contains no unfinished
ExecPlan work:

1. Output exactly this marker on its own line: `@@COMPLETE@@`
2. Stop execution.

## continuation

If you were interrupted and not blocked, continue from the current plan state
without asking what to do next.
