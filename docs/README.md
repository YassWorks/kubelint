# kubelint documentation

| Document | What it is for |
| --- | --- |
| [`vision.md`](vision.md) | What kubelint is, who it is for, and what it will not do. Locked with the owner. |
| [`roadmap.md`](roadmap.md) | Milestones and PR-sized tasks. The current plan. |
| [`architecture.md`](architecture.md) | Crates, data flow, key types and design principles. |
| [`conventions.md`](conventions.md) | How code, tests, docs, commits and PRs are written. |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Setup, local checks, and how to add a rule. |
| [`rules/`](rules/README.md) | The rule catalog: every rule code, planned or implemented. |
| [`decisions/`](decisions/README.md) | Architecture Decision Records (ADRs): why things are the way they are. |
| [`agent/`](agent/playbook.md) | Memory and operating manual for the autonomous development routine. |

## The `agent/` folder

kubelint is developed largely by a daily, stateless Claude Code routine
([ADR 0008](decisions/0008-autonomous-workflow.md)). These files are its memory:

| File | Purpose |
| --- | --- |
| [`agent/playbook.md`](agent/playbook.md) | The procedure every run follows. |
| [`agent/routine-prompt.md`](agent/routine-prompt.md) | The exact prompt configured in the routine, versioned. |
| [`agent/journal/`](agent/journal/README.md) | One entry per run: what happened and what comes next. |
| [`agent/owner-inbox.md`](agent/owner-inbox.md) | Questions and actions for the owner, each with the default being assumed. |
