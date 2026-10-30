# Architecture Decision Records

An ADR records one significant decision: the context, what was decided, what else was
considered and what follows from it. ADRs exist so that nobody (especially a stateless
agent) re-argues a settled question, and so that a decision can be revisited
deliberately instead of drifting.

## When to write one

- Choosing or replacing a core dependency.
- Changing a public contract: CLI flags, config format, rule codes, output schemas.
- Changing crate boundaries or the layering rule.
- Any choice where a future contributor would reasonably ask "why is it like this?"

## How

1. Copy the template below into `NNNN-short-title.md` with the next free number.
2. Status starts as `Accepted` (the PR review is the approval). Use `Proposed` only when
   the decision needs the owner, and add an owner-inbox entry pointing to it.
3. To reverse a decision, write a new ADR that supersedes the old one, and set the old
   one's status to `Superseded by NNNN`. Never rewrite history in an accepted ADR.

```markdown
# NNNN. Title

- Status: Accepted
- Date: YYYY-MM-DD

## Context
## Decision
## Alternatives considered
## Consequences
```

## Index

| # | Title | Status |
| --- | --- | --- |
| 0001 | [Workspace and crate layout](0001-workspace-and-crate-layout.md) | Accepted |
| 0002 | [YAML parsing with saphyr-parser and our own tree](0002-yaml-parsing.md) | Accepted |
| 0003 | [Diagnostics, severities and exit codes](0003-diagnostics-severities-exit-codes.md) | Accepted |
| 0004 | [Rule codes, names and categories](0004-rule-codes-and-categories.md) | Accepted |
| 0005 | [Configuration and suppressions](0005-configuration-and-suppressions.md) | Accepted |
| 0006 | [Testing strategy](0006-testing-strategy.md) | Accepted |
| 0007 | [Toolchain and lint policy](0007-toolchain-and-lint-policy.md) | Accepted |
| 0008 | [Autonomous development workflow](0008-autonomous-workflow.md) | Accepted |
