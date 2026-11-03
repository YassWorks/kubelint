# 0009. Rule selection semantics

- Status: Accepted
- Date: 2026-11-03

## Context

ADR 0005 fixes the configuration format (`select`, `ignore`, `[lint.severity]`,
`fail-on`) and ADR 0004 says that `KL2` or `security` selects a category. Neither says
how selectors combine, whether a category includes the rules that ship disabled by
default, or what happens to a selector that matches nothing. These answers are a public
contract: once users write config files, changing them silently changes which rules run.

## Decision

- A **selector** is a category name (`security`), a code prefix (`KL` and one to three
  digits, like `KL2` or `KL20`), a rule code (`KL2001`) or a rule name
  (`privileged-container`). Matching is exact and case-sensitive.
- **Broad selectors (categories and prefixes) match only rules enabled by default.** A
  rule that ships disabled (opinionated or context-dependent, ADR 0004) runs only when its
  code or name is selected. This keeps `select = ["security"]` low-noise and makes turning
  on an opinionated rule a deliberate, visible act. The same holds in `ignore`: a broad
  selector there never removes an off-by-default rule that was selected by name.
- No `select` means every rule enabled by default. `select` is a union of its selectors.
  `ignore` is applied after `select` and always wins.
- **A selector that matches no rule is an error**, so typos never pass silently. A valid
  prefix whose rules are all disabled by default (like `KL11` once schema rules exist) is
  not an error.
- **KL0001 `invalid-yaml` always runs at `error`.** Naming it in `ignore` or
  `[lint.severity]` is an error; broad selectors that cover it (`syntax`, `KL0`) leave it
  on.
- **Severity overrides name one rule**, by code or name; a category or prefix there is an
  error. Later entries win. An override for a rule that does not run is accepted.
- **`fail-on`** is a severity: the run fails when any finding is at or above it.

## Alternatives considered

- **Broad selectors include every rule (Ruff).** Simple to explain, but selecting
  `security` would switch on rules the project ships disabled because they are noisy,
  against the "low noise" requirement in the vision.
- **Most specific selector wins (Ruff's `select`/`ignore` resolution).** Lets
  `ignore = ["KL3"]` and `select = ["KL3001"]` coexist, but needs ordering rules users
  have to learn. With "ignore wins", listing what you want in `select` does the same job.
- **Unknown selectors are warnings.** Friendlier to configs written for newer versions,
  but a typo in `ignore` would silently keep a rule running. "Did you mean"
  suggestions (M2-10) soften the error instead.

## Consequences

- `kubelint_linter::RuleSelection::resolve` implements this and is the only place it
  lives; config (M2-03), the CLI and suppressions (M2-04) pass selector strings through it.
- Errors carry the list (`select`, `ignore`, `severity`) and index of the bad selector so
  the config loader can point at it.
