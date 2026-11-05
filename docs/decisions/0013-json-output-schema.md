# 0013. JSON output schema for `kubelint check`

- Status: Accepted
- Date: 2026-11-05

## Context

`kubelint check --format json` is for scripts, dashboards and other tools. Its shape is
a public contract (vision: "fits existing workflows"; architecture: "breaking it is a
breaking change"). SARIF (M2-08) and GitHub workflow commands (M2-09) will be built from
the same diagnostics, and an editor integration (M5) will need positions it can trust.

## Decision

- The output is **one JSON object** with three fields: `version` (an integer, now `1`),
  `diagnostics` (an array) and `summary` (an object). The full field list is in
  [`docs/user/output-formats.md`](../user/output-formats.md).
- **Versioning:** adding a field is not a breaking change. Renaming, removing or changing
  the meaning of a field is, and bumps `version`. Consumers should ignore fields they do
  not know.
- **Field names** are `snake_case`, as in ADR 0012.
- **Diagnostics** come in the same order as the human output (file, byte offset, rule
  code). Each has `code`, `name` (the rule name, `null` only if unknown), `severity`,
  `message`, `help` (`null` when absent), `location` and `related` (secondary
  locations, possibly empty).
- **A location** is `path`, `start`, `end` and `message` (the label's note or `null`).
  Paths are as given on the command line or found during discovery, with `/` as the
  separator on every platform; standard input is `<stdin>`.
- **A position** is `line` (1-based), `column` (1-based, counted in UTF-8 bytes) and
  `offset` (0-based byte offset into the file). `end` is exclusive.
- **The summary** has `by_severity` (`error`, `warning`, `info` counts), `files_checked`,
  `objects_checked` and `files_skipped`. It has no timing, so the same input always gives
  byte-identical output.
- Fixes are not part of version 1. They will be added as a new optional field when M5
  introduces them.
- Only the findings go to standard output. Notes (skipped Helm charts) and errors that
  stop the run (invalid configuration, exit code 2) stay on standard error in the human
  format, so standard output is always either one valid JSON document or empty.
- Exit codes are the same as for the human format (ADR 0003).

## Alternatives considered

- **A bare array of diagnostics** (like `kubelint rules --format json`): leaves no room for
  the summary or a version, which matter more here than for the rule list.
- **Columns in UTF-16 code units or characters.** UTF-8 bytes match `offset`, are what
  the engine stores, and are exact for every consumer that reads the file as bytes. SARIF
  (M2-08) and the language server (M5) convert to what their standards require.
- **JSON Lines (one diagnostic per line).** Easier to stream, but kubelint sorts findings
  before printing anyway, and one document is simpler to consume with `jq`.
- **Including the elapsed time.** Useful, but it makes output non-deterministic; the human
  format already shows it.

## Consequences

- `kubelint_output` depends on `serde` and `serde_json` (both pre-approved) and renders
  with private `Serialize` structs, so the schema is visible in one file
  (`crates/kubelint_output/src/json.rs`).
- Tests snapshot the exact output, so any accidental schema change shows up in review.
