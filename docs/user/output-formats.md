# Output formats

`kubelint check` prints its findings in one of these formats, chosen with `--format`:

| `--format` | For | Status |
| --- | --- | --- |
| `human` (default) | people in a terminal | available |
| `json` | scripts and other tools | available |
| `sarif` | GitHub code scanning and security dashboards | planned (M2-08) |
| `github` | annotations in GitHub Actions logs | planned (M2-09) |

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
