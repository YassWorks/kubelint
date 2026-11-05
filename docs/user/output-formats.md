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

One JSON document on standard output. The schema is versioned
([ADR 0013](../decisions/0013-json-output-schema.md)): fields may be added in any
release, but renaming or removing one changes `version`. Ignore fields you do not know.

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

### Examples with `jq`

```console
$ kubelint check --format json . | jq -r '.diagnostics[] | "\(.location.path):\(.location.start.line) \(.code)"'
$ kubelint check --format json . | jq '.summary.by_severity.error'
```

## `sarif`

[SARIF 2.1.0](https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html), the
standard format for static analysis results. GitHub code scanning reads it and shows each
finding on the pull request and in the repository's "Security" tab. The mapping is
recorded in [ADR 0014](../decisions/0014-sarif-output.md). In short:

- Every built-in rule is listed under `runs[0].tool.driver.rules`, with its summary and
  its full explanation as `help.markdown`.
- Each finding is a result with `ruleId`, `level` (`error`, `warning`, or `note` for
  `info`), a message (with the help as a second line), its location and any related
  locations.
- Columns count UTF-16 code units, the SARIF default. Relative paths stay relative, so
  run kubelint from the repository root.

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
private ones. Release binaries and a ready-made GitHub Action are planned (M3-07, M3-08).

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
- The title is the rule code and name; the message has the help as a second line.
- Columns count UTF-16 code units, like SARIF. Paths are printed as found, so run
  kubelint from the repository root.
- The summary line (without timing) ends the output as plain log text.

```yaml
# a step in a GitHub Actions job
- name: Lint manifests
  run: kubelint check --format github .
```

The step fails when kubelint finds something at or above `fail-on` (exit code 1), which
is usually what a CI gate wants.
