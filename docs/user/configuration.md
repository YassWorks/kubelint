# Configuration

kubelint works without any configuration. A configuration file lets a project choose
which rules run, how severe their findings are, and which files are left out.

## The configuration file

```toml
[lint]
select = ["correctness", "security", "KL3001"]  # categories, code prefixes, codes, names
ignore = ["KL2004"]
fail-on = "warning"                             # error | warning | info
kubernetes-version = "1.37"                     # 1.34 to 1.37, default the latest

[lint.severity]
KL3003 = "error"

[files]
exclude = ["vendor/**"]
```

- The file is TOML, named `kubelint.toml` or `.kubelint.toml`.
- kubelint looks for it in the current working directory, then in each parent
  directory. The first file found is used. One run uses one configuration.
- Every key is optional, and an empty file is valid.
- An unknown key or a value of the wrong type is an error, shown as a snippet of the
  file, and kubelint exits with code 2.

**Why TOML:** YAML is the very format whose pitfalls kubelint exists to catch.

**Rejected:** a YAML configuration file. It could still be added later as a second
format if users ask for it.

## Selecting rules

`select`, `ignore` and `[lint.severity]` name rules with **selectors**:

| Selector | Example | Matches |
| --- | --- | --- |
| Category | `security` | the category's rules that are enabled by default |
| Code prefix | `KL2`, `KL20` | the rules enabled by default whose code starts with it |
| Code | `KL2009` | that rule, even if it is disabled by default |
| Name | `writable-root-filesystem` | that rule, even if it is disabled by default |

- Matching is exact and case-sensitive. A code prefix is `KL` followed by one to three
  digits.
- Without `select`, every rule that is enabled by default runs. `select` runs the union
  of its selectors.
- `ignore` is applied after `select` and always wins. A category or prefix in `ignore`
  never removes a disabled-by-default rule that `select` named.
- A selector that matches no rule is an error, with a "did you mean" suggestion when one
  is close. A valid prefix whose rules are all disabled by default is not an error.
- KL0001 `invalid-yaml` always runs, at `error`. Naming it in `ignore` or
  `[lint.severity]` is an error. A category or prefix that covers it (`syntax`, `KL0`)
  leaves it on.
- `[lint.severity]` overrides the severity of one rule, named by code or name. A
  category or prefix there is an error, and so is a second entry for the same rule (its
  code and its name). An override for a rule that does not run is accepted.
- `fail-on` is the lowest severity that makes the run fail with exit code 1. The default
  is `warning`, so `info` findings never fail a run unless `fail-on = "info"`.

**Why categories skip disabled-by-default rules:** `select = ["security"]` stays
low-noise, and turning on an opinionated rule is always a visible act.

**Why an unknown selector is an error:** a typo in `ignore` would otherwise keep a rule
running without anyone noticing.

**Why one entry per rule in `[lint.severity]`:** a TOML table has no order, so when a
rule's code and its name both set a severity, neither can win by coming later.

**Rejected:**

- The later entry wins. The parser reads the table sorted by key, and codes sort before
  names, so the name always won over the code, whatever the order in the file.
- Categories that include every rule (Ruff). Selecting `security` would switch on the
  rules that ship disabled because they are noisy.
- "Most specific selector wins" (Ruff). It needs ordering rules users have to learn.
  With "`ignore` wins", listing what you want in `select` does the same job.
- Unknown selectors as warnings. Friendlier to configurations written for a newer
  version, but typos would pass silently.

## Command-line flags

`kubelint check` takes these flags:

| Flag | Replaces | Value |
| --- | --- | --- |
| `--config <PATH>` | the discovered file | a TOML file that must exist |
| `--isolated` | the discovered file | none; conflicts with `--config` |
| `--select <RULES>` | `[lint] select` | selectors, comma-separated, repeatable |
| `--ignore <RULES>` | `[lint] ignore` | selectors, comma-separated, repeatable |
| `--fail-on <SEVERITY>` | `[lint] fail-on` | `error`, `warning` or `info` |
| `--kubernetes-version <VERSION>` | `[lint] kubernetes-version` | a supported version, see below |

- A flag replaces the file's value. It never adds to it: `--select KL1001` runs KL1001
  (and KL0001) whatever the file selects, and `--ignore KL0002` replaces the file's
  whole `ignore` list. The file's `ignore` still applies to `--select`, so a rule the
  file ignores runs only with `--ignore` naming something else, or with `--isolated`.
- `[lint.severity]` has no flag. Severity overrides are project policy and belong in the
  file.
- A bad selector in a flag is a one-line error, exit code 2. A `--config` path that
  cannot be read is also exit code 2.

**Why flags replace:** running fewer rules than the file selects ("just check this one
rule") is the common command-line case.

**Rejected:**

- `--extend-select` and `--extend-ignore` (Ruff). Two flags per list double the surface
  before anyone asked for it. They can be added later without breaking anything.
- Looking for a configuration next to each linted path. One run could then mix rule
  sets, which makes the output hard to reason about.

## Kubernetes version

```toml
[lint]
kubernetes-version = "1.36"
```

- The Kubernetes version to check against. KL1008 uses it for the names whose rules
  changed between releases, and the schema rules of [M4](../roadmap.md) will use it for
  the API schema.
- kubelint supports the latest four minor releases: 1.34, 1.35, 1.36 and 1.37. The
  default is the latest.
- Write the minor version, with or without a `v` (`1.36`, `v1.36`). A patch number is
  accepted and ignored (`1.36.4`), since patch releases do not change the API.
- The value is a string. `kubernetes-version = 1.30` is an error, because TOML reads it
  as the number 1.3.
- A version outside the supported ones is an error that lists them, exit code 2. In the
  file it is shown as a snippet; from `--kubernetes-version` it is a one-line error.

**Why `[lint]`:** the version decides which findings a run reports, like `select` and
`fail-on`.

**Why a string:** as a TOML number, `1.30` and `1.3` are the same value.

**Rejected:** a top-level key (every other key lives in a table); accepting the version
of a cluster outside the window by falling back to the nearest one (a run would check
an API the cluster does not have without saying so).

## Excluding files

```toml
[files]
exclude = ["vendor/**", "**/testdata"]
```

- Patterns are globs matched against paths relative to the directory of the
  configuration file, with `/` as the separator on every platform. Running kubelint
  from a subdirectory does not change what a pattern means.
- `*` and `?` match within one path component, `**` matches any number of directories,
  `{a,b}` and `[ab]` work as usual.
- A pattern that matches a directory excludes everything below it, so `vendor` and
  `vendor/**` are the same. A path outside the configuration file's directory is never
  excluded.
- Excluded files are left out even when named on the command line. Standard input is
  never excluded.
- Excluded files are not counted as "skipped" in the summary. That number is for files
  kubelint leaves out on its own, such as Helm chart templates.
- An invalid pattern is an error, shown as a snippet of the file, exit code 2.

**Why named files are excluded too:** pre-commit and similar tools pass file names, and
they should honor the project's exclusions.

**Rejected:**

- Gitignore syntax. Its rules about leading and trailing slashes are subtle, and
  `.gitignore` itself is already honored when searching directories.
- Named files bypassing `exclude` (Ruff without `--force-exclude`). pre-commit hooks
  would then lint vendored files.

## Suppressing findings

Comments and the `kubelint/ignore` annotation silence single findings or single objects.
See [suppressions](suppressions.md).
