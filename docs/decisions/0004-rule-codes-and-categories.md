# 0004. Rule codes, names and categories

- Status: Accepted
- Date: 2026-10-30

## Context

Users reference rules in config files, suppression comments and CI dashboards. Those
references must stay valid across releases, and should be readable by humans.

## Decision

- Every rule has a **code** `KL` + 4 digits and a **name** in kebab-case that describes
  the problem (`privileged-container`, not `check-privileged`). Both work everywhere a
  rule can be referenced.
- The first digit is the **category**: `0` syntax, `1` correctness (`11xx` reserved for
  schema validation), `2` security, `3` reliability. Selecting `KL2` or `security` selects
  the whole category.
- Codes are allocated only in `docs/rules/README.md`. Codes are never reused. A released
  rule is never renamed; if a rule must go, it is deprecated first (it keeps working and
  warns for at least one minor release) and its code stays reserved.
- A rule ships **enabled by default** only if it is high-signal on real-world manifests.
  Opinionated or context-dependent rules ship disabled.

## Alternatives considered

- **Names only (ESLint, kube-linter).** Readable, but long in suppression comments and
  harder to sort or group.
- **Codes only (older Pylint).** Compact, but opaque. Ruff moved toward showing both.

## Consequences

- Output shows both: `error[KL2001]: container "api" is privileged (privileged-container)`.
- Category prefixes give users coarse control without listing rules one by one.
