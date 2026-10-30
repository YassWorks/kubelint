# 0003. Diagnostics, severities and exit codes

- Status: Accepted
- Date: 2026-10-30

## Context

"Friendly" output is the product's identity. The data model must support precise
snippets, multiple labels, help text, machine formats and future autofix without
changing shape later. CI users need predictable exit codes.

## Decision

- **Data:** `Diagnostic { code, severity, message, primary: Label, secondary: Vec<Label>,
  help: Option<String>, fix: Option<Fix> }`. A `Label` is a `TextRange` in a file plus an
  optional short message. A `Fix` is a list of `Edit { range, replacement }` plus an
  `Applicability` (`Safe`, `Unsafe`). Diagnostics are plain data in
  `kubelint_diagnostics`; rendering lives in `kubelint_output`.
- **Human rendering** uses `annotate-snippets`, the renderer used by rustc, giving the
  familiar `error[KL2001]: ... --> file:line:col` layout.
- **Severities:** `error`, `warning`, `info`. Each rule has a default; users can override
  per rule.
- **Failing a run:** the run fails if any diagnostic is at or above `fail-on` (default
  `warning`). `info` never fails a run unless `fail-on = "info"`.
- **Exit codes:** `0` clean, `1` findings at or above the threshold, `2` kubelint could not
  do its job (bad arguments, bad config, I/O failure). Invalid YAML is a finding (KL0001),
  not exit code 2.

## Alternatives considered

- **No severities, every finding fails (Ruff, kube-linter).** Simpler, but users of
  Kubernetes tooling expect error vs warning, and some rules are advisory by nature.
- **`miette` for rendering.** Popular and pretty, but more opinionated; `annotate-snippets`
  matches rustc's look exactly and is what Ruff uses.
- **Warnings never fail by default (ESLint).** Makes kubelint easy to ignore in CI; a
  linter that does not fail CI rarely gets acted on.

## Consequences

- Machine formats map directly from the data model (SARIF levels: error, warning, note).
- Autofix in M5 needs no data model change.
