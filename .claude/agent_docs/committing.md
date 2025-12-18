# Committing Guidelines

## Commit Message Format

```
area: short description
```

- always lowercase (including first letter)
- prefix with area/component (e.g., `cli:`, `zub:`, `kernel:`, `bootstrap:`)
- no period at end
- title only, no body (unless specifically requested)

Examples:
```
cli: move verbose output behind --verbose flag
zub: fix hardlink checkout when target is in sibling directory
zub: switch from SHA256 to BLAKE3 for content hashing
bootstrap: update glibc to 2.40
```

## Git Add

- **never use `git add -A`** or `git add .`
- always explicitly list files: `git add src/cli/src/foo.rs src/cli/src/bar.rs`
- this prevents accidentally committing unrelated changes

## Commit Granularity

- each self-contained feature or fix gets its own commit
- don't batch unrelated changes together
- when asked to commit, check if changes can be logically split

Example: if you fixed a bug AND added a feature:
```bash
# bad
git add -A && git commit -m "fix bug and add feature"

# good
git add src/fix.rs
git commit -m "cli: fix null pointer in parser"
git add src/feature.rs
git commit -m "cli: add export command"
```

## Commit Must Be Self-Contained

Each commit must:
1. **compile** - `cargo check` or `cargo build` passes
2. **work correctly** - doesn't break existing functionality
3. **be independent** - earlier commits don't depend on later ones

When splitting commits, order matters:
- if feature B depends on bugfix A, commit A first
- test that each commit works before moving to the next
