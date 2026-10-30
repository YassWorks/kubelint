# 0006. Testing strategy

- Status: Accepted
- Date: 2026-10-30

## Context

A linter's correctness is mostly "which diagnostics, at which positions, with which
text". Hand-written assertions for that are tedious and brittle. The project is also
developed largely by an autonomous agent, so tests are the main safety net that lets it
merge on green CI.

## Decision

- **Snapshot tests with `insta`** for anything whose output is text: rule results, the
  human renderer, JSON/SARIF output, CLI runs, YAML tree dumps.
- **Rule fixtures:** `crates/kubelint_linter/resources/test/fixtures/<category>/<CODE>.yaml`.
  Each fixture contains violating cases and passing cases (true negatives), separated
  into YAML documents with a comment above each saying what it tests. The harness lints
  the fixture with only that rule enabled and snapshots the rendered diagnostics.
- **CLI tests** run the real binary via `assert_cmd` and snapshot stdout, stderr and
  exit code.
- **Unit tests** next to the code for pure logic (parsers, validators, selector
  resolution), table-driven where possible.
- Every bug fix starts with a failing test.
- Snapshots are updated with `INSTA_UPDATE=always cargo test` and the `.snap` diff is
  reviewed like code. CI never updates snapshots.

## Alternatives considered

- **Inline expected-diagnostic annotations in fixtures** (like rustc's `//~ ERROR`).
  Precise, but needs a custom harness; snapshots give most of the value today.

## Consequences

- Reviewing a rule PR means reading its fixture and snapshot, which double as
  documentation of the rule's behavior.
- Snapshot churn is visible in diffs; a renderer change touches many snapshots, which is
  expected and should be its own PR.
