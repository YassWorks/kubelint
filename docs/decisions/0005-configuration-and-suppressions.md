# 0005. Configuration and suppressions

- Status: Accepted
- Date: 2026-10-30

## Context

Zero config must work, but teams need to tune rule selection, severities and file sets,
and to silence a specific finding with a reason.

## Decision

**Config file:** TOML, named `kubelint.toml` or `.kubelint.toml`, discovered by walking
up from the working directory (first match wins). `--config <path>` picks a file,
`--isolated` ignores all config. CLI flags override the file.

```toml
[lint]
select = ["correctness", "security", "KL3001"]  # categories, code prefixes, codes, names
ignore = ["KL2004"]
fail-on = "warning"                               # error | warning | info

[lint.severity]
KL3003 = "error"

[files]
exclude = ["vendor/**"]
```

Unknown keys are errors with a precise location, never silently ignored.

**Suppression comments** (ESLint-style, familiar to most developers):

- `# kubelint-disable-next-line <rules>`: the next non-comment line.
- `# kubelint-disable-line <rules>`: trailing comment, same line.
- `# kubelint-disable-file <rules>`: the whole file.
- `<rules>` is a comma-separated list of codes or names; an optional `-- reason` follows.

A diagnostic is suppressed when its primary label starts on a suppressed line.

**Object-level suppression via annotations** is planned but its key is an open owner
decision (see `docs/agent/owner-inbox.md`). The working assumption is
`kubelint/ignore: "KL2001,KL3003"`.

## Alternatives considered

- **YAML config.** Familiar to Kubernetes users, but YAML is exactly the format whose
  pitfalls this tool exists to catch, and Rust's YAML deserialization ecosystem is less
  settled than `toml`. Can be added later as an alternative format if users ask.
- **`# kubelint: ignore[...]` (Python style).** Less familiar to the target audience than
  ESLint's style.

## Consequences

- Config errors are rendered with the same snippet renderer as lint findings.
- Selector resolution needs the rule registry, so it lives in `kubelint_linter`, while
  `kubelint_config` only parses strings.
