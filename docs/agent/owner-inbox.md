# Owner inbox

The asynchronous channel between the autonomous routine and the owner, **YassWorks**.

- **Agent:** add an item when something needs the owner. Always state the default you
  are proceeding with, then keep working. Never block a run on an answer.
- **Owner:** write your answer under **Answer** (edit on GitHub is fine) and commit to
  `main`, or reply in a PR comment. The next run applies it and moves the item to
  "Resolved".

Item format:

```markdown
### OI-NNN: Short title
- Opened: YYYY-MM-DD · Type: action | decision | fyi · Priority: high | normal | low
- Context: why this needs the owner.
- Default: what the agent does until answered.
- Answer: _(owner)_
```

## Open

### OI-001: Repository settings so the routine can merge on green CI
- Opened: 2026-10-30 · Type: action · Priority: high (before the first routine run)
- Context: the routine merges its own PRs once CI passes. Recommended settings:
  1. **Settings → Rules → Rulesets → New branch ruleset** targeting `main`: enable
     "Require status checks to pass" with the check **`ci-success`**, and "Block force
     pushes". Do **not** require approvals (the routine cannot approve its own PRs).
  2. **Settings → General → Pull Requests:** keep "Allow squash merging" on. Optionally
     turn on "Automatically delete head branches".
  3. **Settings → Actions → General:** Actions allowed for this repository.
- Default: until this is done, the routine still only merges when `ci-success` is green,
  but nothing technically prevents a mistake.
- Answer: _(owner)_

### OI-002: Annotation key for object-level suppressions
- Opened: 2026-10-30 · Type: decision · Priority: low (needed in M2)
- Context: besides comments, users should be able to silence a rule on one object with
  an annotation, which survives Helm rendering. Kubernetes convention is to prefix
  annotation keys with a domain you own (like `example.com/ignore`). The project owns no
  domain yet.
- Default: `kubelint/ignore: "KL2001,KL3003"` (a valid key that claims no domain).
- Answer: _(owner)_

### OI-003: Name similarity with kube-linter
- Opened: 2026-10-30 · Type: fyi · Priority: low
- Context: StackRox's `kube-linter` is an established Go tool with a very similar name.
  `kubelint` is free on crates.io, but users may confuse the two. Renaming gets more
  expensive with every user, so if it ever happens, before v0.1 is the time.
- Default: keep the name `kubelint`.
- Answer: _(owner)_

### OI-004: Going public
- Opened: 2026-10-30 · Type: decision · Priority: normal (before v0.1)
- Context: the repository is private. An open-source project needs a public repository
  for users, contributors, crates.io links and GitHub code scanning integration.
- Default: stays private until v0.1 is ready; the routine will raise this again in M3.
- Answer: _(owner)_

### OI-005: Releasing v0.1
- Opened: 2026-10-30 · Type: action · Priority: low (end of M3)
- Context: publishing to crates.io needs the owner's crates.io account and an API token
  stored as a repository secret; tagging `v0.1.0` triggers the release workflow (M3-07).
- Default: the routine prepares everything and adds a "ready to release" item when M3
  is done. It never tags or publishes.
- Answer: _(owner)_

## Resolved

_Nothing yet._
