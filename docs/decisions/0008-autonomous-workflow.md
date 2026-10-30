# 0008. Autonomous development workflow

- Status: Accepted
- Date: 2026-10-30

## Context

The owner wants the project built mostly by a Claude
Code routine that runs once a day in a fresh, stateless cloud session. The owner may be
unavailable for a week or more. The workflow must keep making steady, coherent progress
without a human in the loop, while staying easy to review and reverse.

## Decision

- **The repository is the memory.** Everything a run needs is in markdown under `docs/`:
  vision, architecture, ADRs, roadmap, rule catalog, playbook, journal and owner inbox.
  Nothing relies on chat history or external tools.
- **The routine prompt is a bootloader.** It holds only the mandate and the
  non-negotiables, and points to `docs/agent/playbook.md`, which holds the procedure and
  can evolve through normal PRs. The prompt itself lives in the routine configuration,
  owned by the owner, not in the repository.
- **Small PRs, merged by the agent on green CI.** One roadmap task per PR. The agent
  squash-merges its own PR once every required check passes, then starts the next task
  from the updated `main`. No human approval is required. The owner reviews history
  after the fact and can revert any PR.
- **Sequential within a run.** One PR at a time: open, wait for CI, merge, next. The run
  ends with a journal PR. At least two PRs per run; three to five is the target.
- **The owner is asynchronous.** Questions go to `docs/agent/owner-inbox.md` together with
  the default the agent will assume; the agent never blocks waiting for an answer.
- **Concepts are documented.** Every PR has a "Concepts" section explaining the ideas it
  relies on, and new concepts land in `docs/learning/glossary.md`.

## Alternatives considered

- **Owner merges everything (stacked PRs).** Safer per PR, but nothing lands during an
  absence, and an early problem blocks the whole stack.
- **Several runs per day.** Faster, but the owner chose one run per day.

## Consequences

- CI is the only gate, so CI must stay strict and must never be weakened to get a PR
  through. Test quality is a first-class concern.
- `main` must stay green; fixing a red `main` preempts all other work.
- Every PR is small and self-explanatory, so reviewing a week of history is practical.
