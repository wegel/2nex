Analyze the work done in this session and suggest improvements to the Claude configuration.

## Steps

1. **Review session work**: Look at what tasks were performed, what errors occurred, what corrections the user made, and what patterns emerged.

2. **Read current configuration**:
   - Read `CLAUDE.md` for project guidelines
   - Read all files in `.claude/agent_docs/` for existing documentation
   - Check `.claude/commands/` for existing commands
   - Check `.claude/hooks/` if it exists

3. **Identify gaps and improvements**:
   - Missing documentation for patterns that came up
   - Corrections the user had to make repeatedly (indicates missing/unclear guidance)
   - New commands that could automate common workflows
   - Hooks that could catch common mistakes
   - Outdated or incorrect information in existing docs

4. **Output suggestions** in this format:

```
## Suggested Modifications

### CLAUDE.md
- [what to add/change and why]

### Agent Docs
- [new doc or modification needed and why]

### Commands
- [new command that would help and why]

### Hooks
- [hook that could prevent errors and why]

### Other
- [any other improvements]
```

Be specific about what to add/change. Reference actual errors or corrections from the session as evidence for why the change is needed.
