# 0011. `[files] exclude` pattern semantics

- Status: Accepted
- Date: 2026-11-04

## Context

ADR 0005 adds `[files] exclude = ["vendor/**"]` to the configuration but does not say
what the patterns are matched against, which glob dialect they use, or whether they
apply to files named on the command line. Users will copy patterns between projects and
CI setups, so this is a public contract.

## Decision

- Patterns are **globs matched against paths relative to the directory of the
  configuration file** that lists them, with `/` as the separator on every platform.
  Running kubelint from a subdirectory does not change what a pattern means.
- Dialect: `globset` with `literal_separator`. `*` and `?` match within one path
  component, `**` matches any number of directories, `{a,b}` and `[ab]` work as usual.
- A pattern that matches a directory excludes everything below it, so `vendor` and
  `vendor/**` are the same. A path outside the configuration file's directory is never
  excluded.
- **Excluded files are left out even when named on the command line.** pre-commit and
  similar tools pass file names, and they should honor the project's exclusions. Standard
  input is never excluded.
- Excluded files are not counted in the summary's "skipped" number, which is reserved
  for files kubelint left out on its own (Helm templates) and might surprise the user.
- An invalid pattern is a configuration error, shown as a snippet of the file, exit
  code 2.

## Alternatives considered

- **Gitignore syntax** (patterns without `/` match at any depth). Familiar, but its
  rules about leading and trailing slashes are subtle, and `.gitignore` itself is already
  honored during directory search for that style of exclusion.
- **Named files bypass `exclude` (Ruff without `--force-exclude`).** Matches "I asked
  for this file", but makes pre-commit hooks lint vendored files, the most common
  complaint about that behavior.

## Consequences

- `crates/kubelint/src/exclude.rs` normalizes paths lexically (no symlink resolution)
  before matching.
- Excluded directories are still walked and their files filtered afterwards. If that
  shows up in the benchmark (M3-03), prune them during the walk instead.
