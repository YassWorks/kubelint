# Configuration

kubelint works without any configuration. A configuration file lets a project choose
which rules run, how severe their findings are, and which files are left out.

## The configuration file

```toml
[lint]
select = ["correctness", "security", "KL3001"]  # categories, code prefixes, codes, names
ignore = ["KL2004"]
fail-on = "warning"                             # error | warning | info

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
  category or prefix there is an error. Later entries win. An override for a rule that
  does not run is accepted.
- `fail-on` is the lowest severity that makes the run fail with exit code 1. The default
  is `warning`, so `info` findings never fail a run unless `fail-on = "info"`.

**Why categories skip disabled-by-default rules:** `select = ["security"]` stays
low-noise, and turning on an opinionated rule is always a visible act.

**Why an unknown selector is an error:** a typo in `ignore` would otherwise keep a rule
running without anyone noticing.

**Rejected:**

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

- A flag replaces the file's value. It never adds to it: `--select KL1001` runs KL1001
  (and KL0001) whatever the file selects, and `--ignore KL0002` replaces the file's
  whole `ignore` list.
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

### Comments

```yaml
securityContext:
  # kubelint-disable-next-line KL2001 -- the CNI plugin needs this
  privileged: true
  allowPrivilegeEscalation: true  # kubelint-disable-line privilege-escalation-allowed
```

| Comment | Silences |
| --- | --- |
| `# kubelint-disable-next-line <rules>` | the next line that is not only a comment |
| `# kubelint-disable-line <rules>` | its own line |
| `# kubelint-disable-file <rules>` | the whole file, wherever the comment is |

- `<rules>` is a comma-separated list of rule codes or names. An optional `-- reason`
  follows.
- A finding is silenced when its highlighted range starts on a silenced line.
- KL0001 `invalid-yaml` cannot be silenced. A comment that names an unknown rule, names
  KL0001 or names no rule silences nothing, and KL0003 `invalid-suppression` reports it.

**Rejected:** Python-style `# kubelint: ignore[KL2001]`. ESLint's style is more familiar
to most people who write manifests.

### Annotations

```yaml
metadata:
  annotations:
    kubelint/ignore: "KL2001,KL3003"
```

The `kubelint/ignore` annotation silences the listed rules for that one object.

- The value is a comma-separated list of rule codes or names, like a suppression
  comment, without a `-- reason`.
- A finding is silenced when its highlighted range starts inside the object. That
  covers findings about the object made while looking at other objects, such as
  KL1004 `duplicate-object`, which points at the later of two duplicates.
- KL0001 `invalid-yaml` cannot be silenced. An annotation that names an unknown rule,
  names KL0001 or names no rule silences nothing, and KL0003 `invalid-suppression`
  reports it. A value that is not a string is ignored.
- In a `kind: List`, each item is an object of its own: put the annotation on the item.
  An annotation on the List itself silences nothing.

**Why a range:** it covers every finding about the object, including the ones a rule
makes while comparing objects, with one simple test that does not depend on how a rule
is written.

**Rejected:** silencing a rule by not running it on the object. Simpler, but findings
from rules that look at every object at once, such as KL1004, would still appear.

**Why an annotation:** it survives tools that re-serialize YAML and drop comments, such as
`kustomize build`. `kubelint/ignore` is a valid annotation key that claims no domain.

**Rejected:**

- `kubelint.dev/ignore`. It follows the domain prefix convention, but with a domain the
  project does not own.
- One key per rule, such as `ignore.kubelint/KL2001: "reason"` (kube-linter's style).
  More verbose for the common case of silencing two or three rules.
