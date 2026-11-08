# Output formats

`kubelint check` prints its findings in one of these formats, chosen with `--format`:

| `--format` | For | Status |
| --- | --- | --- |
| `human` (default) | people in a terminal | available |
| `json` | scripts and other tools | available |
| `sarif` | GitHub code scanning and security dashboards | available |
| `github` | annotations in GitHub Actions logs | available |

Whatever the format, the exit code is the same: `0` when nothing at or above the
`fail-on` severity was found, `1` when something was, `2` when kubelint could not run
(for example an invalid configuration file). Notes and errors that stop the run are
printed on standard error in the human format.

## `human`

Code snippets pointing at the exact line and column, with help and the rule name, then a
summary line. `--color auto|always|never` controls colors. The layout is for people and
may change between versions; do not parse it.

## `json`

One JSON document on standard output. The schema is versioned: fields may be added in
any release, but renaming or removing a field, or changing its meaning, increments
`version`. Ignore fields you do not know.

- Standard output holds only that document, or nothing when the run stops with exit
  code 2. Notes (such as skipped Helm charts) and errors go to standard error.
- Field names are `snake_case`.
- There is no timing, so the same input always gives byte-identical output.
- Fixes are not part of version 1. They will be a new optional field.

```console
$ kubelint check --format json deploy/
```

```json
{
  "version": 1,
  "diagnostics": [
    {
      "code": "KL1001",
      "name": "missing-api-version",
      "severity": "error",
      "message": "`Service` object has no `apiVersion`",
      "help": "add `apiVersion: v1`",
      "location": {
        "path": "deploy/broken.yml",
        "start": { "line": 1, "column": 1, "offset": 0 },
        "end": { "line": 1, "column": 14, "offset": 13 },
        "message": null
      },
      "related": []
    }
  ],
  "summary": {
    "by_severity": { "error": 1, "warning": 0, "info": 0 },
    "files_checked": 2,
    "objects_checked": 2,
    "files_skipped": 0
  }
}
```

### Fields

| Field | Type | Meaning |
| --- | --- | --- |
| `version` | integer | Schema version, currently `1`. |
| `diagnostics` | array | The findings, sorted by file, then position, then rule code. |
| `diagnostics[].code` | string | Rule code, such as `KL1001`. |
| `diagnostics[].name` | string or null | Rule name, such as `missing-api-version`. |
| `diagnostics[].severity` | string | `error`, `warning` or `info`, after configuration. |
| `diagnostics[].message` | string | What is wrong, in one sentence. |
| `diagnostics[].help` | string or null | How to fix it. |
| `diagnostics[].location` | location | Where the problem is. |
| `diagnostics[].related` | array of locations | Other places involved, such as the first of two duplicates. |
| `summary.by_severity` | object | Number of findings per severity: `error`, `warning`, `info`. |
| `summary.files_checked` | integer | Files linted. |
| `summary.objects_checked` | integer | Kubernetes objects linted. |
| `summary.files_skipped` | integer | Files found but not linted (Helm chart templates). |

A **location** has:

| Field | Type | Meaning |
| --- | --- | --- |
| `path` | string | The file, as given or found, with `/` separators. `<stdin>` for standard input. |
| `start` | position | First byte of the highlighted range. |
| `end` | position | Just after the last byte of the range (exclusive). |
| `message` | string or null | A short note about this location. |

A **position** has `line` (1-based), `column` (1-based, counted in UTF-8 bytes) and
`offset` (0-based byte offset into the file).

**Why columns in UTF-8 bytes:** they match `offset`, they are what the engine stores, and
they are exact for every tool that reads the file as bytes. SARIF and `github` convert to
the UTF-16 columns their consumers expect.

**Rejected:**

- A bare array of diagnostics. It leaves no room for the summary or a version.
- JSON Lines, one diagnostic per line. kubelint sorts findings before printing anyway,
  and one document is simpler to consume with `jq`.
- The elapsed time in the summary. It would make the output differ on every run.

### Examples with `jq`

```console
$ kubelint check --format json . | jq -r '.diagnostics[] | "\(.location.path):\(.location.start.line) \(.code)"'
$ kubelint check --format json . | jq '.summary.by_severity.error'
```

## `sarif`

[SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html), the
standard format for static analysis results. GitHub code scanning reads it and shows each
finding on the pull request and in the repository's "Security" tab.

- One run, with `tool.driver` named `kubelint`, its version and the repository as
  `informationUri`.
- Every built-in rule is listed under `runs[0].tool.driver.rules` in code order, whether
  it found anything or not: `id` (the code), `name`, the summary as `shortDescription`
  and `fullDescription`, the explanation as `help.text` and `help.markdown`, the default
  level, and the category as the only tag.
- Each finding is a result with `ruleId`, `ruleIndex`, `level` (`error`, `warning`, or
  `note` for `info`), a message with the help as a second line (`help: ...`), its
  location, and its related locations when it has any.
- Columns count UTF-16 code units (`columnKind: "utf16CodeUnits"`), the SARIF default.
- Relative paths stay relative, with `/` separators and `uriBaseId: "%SRCROOT%"`, so run
  kubelint from the repository root. Absolute paths become `file:` URIs.
  `originalUriBaseIds` is not written, because it would put the machine's working
  directory into the output.
- No timing and no invocation details: the same input gives byte-identical output.

**Why every rule is listed:** `ruleIndex` stays the same whatever the configuration, and
dashboards can show the documentation of a rule that is enabled later.

**Why the help goes into the message:** SARIF results have no help field, and GitHub shows
only the message next to the code.

**Rejected:**

- A SARIF crate such as `serde-sarif`. A large generated API for the handful of fields
  used here.
- Listing only the rules that ran or fired. Rule indexes would change with
  configuration.
- Columns in Unicode code points. UTF-16 is the default, so tools that ignore
  `columnKind` still read the columns correctly.

### Uploading to GitHub code scanning

```yaml
# .github/workflows/kubelint.yml
name: kubelint
on: [push, pull_request]
permissions:
  contents: read
  security-events: write   # needed to upload SARIF
jobs:
  kubelint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install kubelint
        run: cargo install --locked --git https://github.com/YassWorks/kubelint kubelint
      - name: Lint manifests
        # Findings make kubelint exit with 1; let the upload step run anyway.
        run: kubelint check --format sarif . > kubelint.sarif || true
      - name: Upload to code scanning
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: kubelint.sarif
          category: kubelint
```

Code scanning is free for public repositories and needs GitHub Advanced Security for
private ones. Downloading a binary from the
[latest release](https://github.com/YassWorks/kubelint/releases/latest) is faster than
`cargo install`; a ready-made GitHub Action is planned.

## `github`

[Workflow commands](https://docs.github.com/actions/reference/workflow-commands-for-github-actions)
that GitHub Actions turns into annotations: each finding appears in the job log and on
the changed line of the pull request's "Files changed" tab. No upload step or extra
permission is needed.

```console
$ kubelint check --format github .
::error file=deploy/broken.yml,line=1,col=1,endLine=1,endColumn=14,title=KL1001 missing-api-version::`Service` object has no `apiVersion`%0Ahelp: add `apiVersion: v1`
Found 1 error. Checked 2 files and 2 objects.
```

- `error` and `warning` findings become `::error` and `::warning`; `info` becomes
  `::notice`.
- Each command has `file`, `line`, `col`, `endLine`, `endColumn` and `title`, in that
  order. The title is the rule code and name (`KL1001 missing-api-version`), or only the
  code when the name is unknown.
- The message has the help as a second line. Related locations are not shown, because an
  annotation has one location.
- Columns count UTF-16 code units, like SARIF. Paths are printed as found, so run
  kubelint from the repository root.
- Escaping follows the Actions runner: `%`, `\r` and `\n` in messages, and also `:` and
  `,` in property values.
- The summary line (without timing) ends the output as plain log text.

**Rejected:**

- No summary line. The job log would not say how many files were checked, which matters
  most when nothing is found.
- One annotation per related location. It would count each finding several times in
  the annotation totals.

```yaml
# a step in a GitHub Actions job
- name: Lint manifests
  run: kubelint check --format github .
```

The step fails when kubelint finds something at or above `fail-on` (exit code 1), which
is usually what a CI gate wants.

## `kubelint rules` and `kubelint rule`

`kubelint rules` prints a table with one rule per line, in code order: `CODE`, `NAME`,
`CATEGORY`, `DEFAULT` (`on` or `off`), `SEVERITY` and `SUMMARY`. The table is for people
and its layout may change.

`kubelint rules --format json` prints a JSON array in code order. New fields may be
added; renaming or removing one is a breaking change.

| Field | Type | Example |
| --- | --- | --- |
| `code` | string | `"KL0001"` |
| `name` | string | `"invalid-yaml"` |
| `category` | string | `"syntax"` |
| `default_enabled` | boolean | `true` |
| `default_severity` | string | `"error"` |
| `summary` | string | `"The file is not valid YAML."` |

`kubelint rule <CODE|NAME>` prints the rule's documentation as Markdown: a heading with
the code and name, the summary, a line with the category, severity and default state,
then the explanation. An unknown code or name is an error, exit code 2.

**Rejected:**

- A top-level object with a `version` field for `kubelint rules --format json`. Nothing
  needs it yet.
- Plain text for `kubelint rule`. Markdown reads fine in a terminal and is the same text
  the SARIF output carries.
