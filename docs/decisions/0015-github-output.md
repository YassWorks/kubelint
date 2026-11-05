# 0015. GitHub Actions output

- Status: Accepted
- Date: 2026-11-05

## Context

`kubelint check --format github` prints GitHub Actions workflow commands so findings
show up as annotations without SARIF upload or extra permissions. The line format is
GitHub's, but which fields kubelint fills and how is a public contract other tools may
parse.

## Decision

- One command per finding, in the usual order: `::error`, `::warning` or `::notice`
  (for `info`), then `file`, `line`, `col`, `endLine`, `endColumn` and `title`, then
  `::` and the message.
- `title` is the rule code and name (`KL1001 missing-api-version`), or just the code if
  the name is unknown.
- The message is the diagnostic message, then `help: ...` on a second line when there is
  help. Secondary labels are not shown (annotations have one location).
- Columns are UTF-16 code units, like SARIF (ADR 0014); paths use `/` and are printed as
  found.
- Escaping follows the Actions runner: `%`, `\r` and `\n` in messages; also `:` and `,`
  in property values.
- The last line is the human summary line without timing, as plain log text.

## Alternatives considered

- **Leaving out the summary line:** the job log would not say how many files were
  checked, which helps when nothing is found.
- **One annotation per secondary label:** would double-count findings in the
  annotation totals.

## Consequences

- `kubelint_output::GithubRenderer` reuses the human summary line and `LineIndex`'s
  UTF-16 columns; no new dependency.
