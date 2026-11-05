# 0014. SARIF output

- Status: Accepted
- Date: 2026-11-05

## Context

`kubelint check --format sarif` exists so that GitHub code scanning (and other security
dashboards) can show kubelint's findings on pull requests and in the "Security" tab.
SARIF 2.1.0 is a large standard; this ADR records which parts kubelint emits and how its
diagnostics map onto them. The architecture already decided on hand-written `serde`
structs instead of a SARIF crate.

## Decision

- **One run** per log, with `tool.driver` named `kubelint`, its version and
  `informationUri` set to the repository.
- **Every built-in rule** is listed in `tool.driver.rules`, in code order, whether or
  not it found anything: `id` (the code), `name`, `shortDescription` and
  `fullDescription` (the summary), `help.text` and `help.markdown` (the explanation, the
  same Markdown `kubelint rule` prints), `defaultConfiguration.level` and the category as
  the single entry of `properties.tags`. Listing every rule keeps `ruleIndex` stable and
  lets dashboards show documentation for rules that later fire.
- **Results:** `ruleId`, `ruleIndex` (left out when the code has no rule entry), `level`,
  `message.text`, `locations` (the primary label) and `relatedLocations` (secondary
  labels, left out when empty). A label's note becomes the location's `message`.
- **Levels:** `error` to `error`, `warning` to `warning`, `info` to `note`.
- **Help** is appended to the message as a second line, `help: ...`, because SARIF
  results have no help field of their own and GitHub shows only the message next to the
  code.
- **Columns** are counted in UTF-16 code units (`columnKind: "utf16CodeUnits"`), the
  SARIF default and what GitHub expects. Regions use `startLine`, `startColumn`,
  `endLine` and `endColumn` (exclusive); no byte offsets, which SARIF counts in
  characters.
- **Paths:** relative paths stay relative, with `/` separators and `uriBaseId:
  "%SRCROOT%"`, and are percent-encoded outside the URI unreserved set (so `<stdin>`
  becomes `%3Cstdin%3E`). Absolute paths become `file:` URIs without a base. GitHub
  resolves relative URIs against the repository root, so kubelint should run from there.
  `originalUriBaseIds` is not emitted, because it would put the machine's working
  directory into the output.
- No timing, no invocation details: the same input gives byte-identical output.

## Alternatives considered

- **A SARIF crate** (`serde-sarif`): a large generated API for the handful of fields used
  here, and another dependency to vet. Rejected, as planned in the architecture.
- **Listing only the rules that ran or fired:** smaller, but rule indexes would change
  with configuration, and dashboards lose the documentation of rules that are enabled
  later.
- **Columns in Unicode code points** (`columnKind: "unicodeCodePoints"`, the only other
  kind SARIF allows; UTF-8 bytes are not an option): UTF-16 code units are the default,
  so consumers that ignore `columnKind` still read the columns correctly.

## Consequences

- `kubelint_output::SarifRenderer` takes a `SarifRule` list from the CLI, because the
  output layer cannot see the rule registry.
- Tests check the fields GitHub requires structurally and snapshot the exact output with
  stand-in rules, so adding a real rule does not change the snapshot.
