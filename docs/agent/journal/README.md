# Journal

One file per routine run, named `YYYY-MM-DD.md` (`YYYY-MM-DD-2.md` for a second run the
same day). Files are never edited after their PR merges, so they never conflict. Each run
reads the three most recent entries during orientation.

Keep entries short and factual. Link PRs by number.

## Template

```markdown
# YYYY-MM-DD

## Summary
One or two sentences: what moved forward today.

## Pull requests
- #NN `feat(scope): title` (merged | open: why | closed: why)

## Decisions
- Decisions made today that are not obvious from the PRs, and ADRs written.

## Problems
- What went wrong or was slow, and what was done about it. "None" is fine.

## Owner inbox
- Items added or resolved today. "None" is fine.

## Next run should
1. The concrete next task(s), with roadmap IDs.
2. Anything half-done and where it lives.
3. Warnings for tomorrow (flaky test, tricky upcoming task, open question).
```
