# 0010. Command-line configuration flags

- Status: Accepted
- Date: 2026-11-04

## Context

ADR 0005 says that `--config <path>` picks a configuration file, `--isolated` ignores
every configuration file and "CLI flags override the file". It does not say which flags
exist, how a list is written on the command line, or whether a flag adds to the file's
list or replaces it. These are a public contract: CI scripts will hard-code them.

## Decision

`kubelint check` takes these flags:

| Flag | Replaces | Value |
| --- | --- | --- |
| `--config <PATH>` | the discovered file | a TOML file that must exist |
| `--isolated` | the discovered file | none; conflicts with `--config` |
| `--select <RULES>` | `[lint] select` | selectors, comma-separated, repeatable |
| `--ignore <RULES>` | `[lint] ignore` | selectors, comma-separated, repeatable |
| `--fail-on <SEVERITY>` | `[lint] fail-on` | `error`, `warning` or `info` |

- **A flag replaces the file's value, it never adds to it.** `--select KL1001` runs
  KL1001 (and KL0001) whatever the file selects. `--ignore` replaces the file's whole
  `ignore` list, so `--ignore KL0002` brings back a rule the file ignored.
- `[lint.severity]` has no flag. Severity overrides are project policy and belong in the
  file.
- **Errors.** A configuration file that cannot be parsed, or a selector in it that cannot
  be resolved, is printed as a snippet of the file pointing at the bad text, and the run
  exits with code 2. A bad selector given in a flag is a one-line error, also exit code 2.
  A path given with `--config` that cannot be read is an exit-code-2 error.
- Discovery starts at the current working directory, not at the linted paths. One run
  uses one configuration.

## Alternatives considered

- **`--extend-select` / `--extend-ignore` that add to the file (Ruff).** Useful, but two
  flags per list double the surface before anyone has asked for it. They can be added
  later without breaking anything.
- **Flags that add to the file's lists.** Then there is no way to run a narrower set than
  the file from the command line, which is the common "just check this one rule" case.
- **Discovery from each linted path.** Needed for monorepos with several configurations,
  but it means one run can mix rule sets, which makes output hard to reason about. A
  single configuration per run is the simple start.

## Consequences

- `crates/kubelint/src/settings.rs` builds a `RuleSelection` from the file and the flags
  and remembers which list came from the file, so a `SelectionError` can point back at
  the file only when the selector was written there.
- `kubelint_output::HumanRenderer::render_error` renders an error that is not a lint
  finding as a snippet. Later configuration errors (`[files] exclude` globs) reuse it.
