# 0012. `kubelint rules` and `kubelint rule` output

- Status: Accepted
- Date: 2026-11-04

## Context

The vision promises that every rule explains itself and that `kubelint rule <code>`
prints its documentation. Scripts and editor integrations will also want the rule list
in a machine-readable form. `kubelint rules --format json` is therefore a public output
schema, and the subcommand names are part of the CLI contract.

## Decision

- `kubelint rules` prints an aligned table: `CODE`, `NAME`, `CATEGORY`, `DEFAULT`
  (`on`/`off`), `SEVERITY`, `SUMMARY`, one rule per line in code order. The table is for
  people; its layout may change.
- `kubelint rules --format json` prints a JSON array in code order. Each element has
  exactly these fields, and new fields may be added later (adding is not a breaking
  change, removing or renaming is):

  | Field | Type | Example |
  | --- | --- | --- |
  | `code` | string | `"KL0001"` |
  | `name` | string | `"invalid-yaml"` |
  | `category` | string | `"syntax"` |
  | `default_enabled` | boolean | `true` |
  | `default_severity` | string | `"error"` |
  | `summary` | string | `"The file is not valid YAML."` |

- `kubelint rule <CODE|NAME>` prints Markdown: a heading with the code and name, the summary, a line with the category, severity and default state, then the
  rule's explanation. An unknown code or name is an exit-code-2 error.
- Field names use `snake_case`, the convention the `--format json` diagnostics output
  (M2-07) will follow too.

## Alternatives considered

- **A top-level object (`{"version": 1, "rules": [...]}`)** leaves room for metadata,
  but nothing needs it yet, and the diagnostics JSON (M2-07) is where a schema version
  matters. Can be revisited before v0.1 ships.
- **Plain text for `kubelint rule`.** Markdown reads fine in a terminal and is exactly
  what M3-01 will write into `docs/rules/<CODE>.md`, so the two stay identical.

## Consequences

- `crates/kubelint/src/rules_cmd.rs` renders both from `RuleMeta`; nothing is
  duplicated by hand.
- M3-01 (generated rule docs) can reuse `explain()` for the page body.
