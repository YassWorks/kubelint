# Roadmap

What comes next, in order. Within a milestone, items are listed so that each one only
needs the items above it. Work on the first unchecked item of the current milestone.

- Tick an item in the pull request that completes it.
- An item too big for one pull request is split in place into smaller items first.
- New items go at the end of their group, with a short reason when it is not obvious.
- When the current milestone has five or fewer open items, break the next outline into
  items.

## Current focus

**M4: Schema validation.** Check every built-in object against the real Kubernetes API
schema of a chosen version: unknown fields, wrong types, missing required fields, bad
enum values, and removed or deprecated API versions.

## Done

- **M0: Foundation** (2026-10-30). Vision, architecture, conventions, rule catalog,
  Cargo workspace with one crate per layer, CI on Linux, macOS and Windows.
- **M1: Walking skeleton** (2026-11-03). YAML tree with byte ranges and error recovery,
  Kubernetes object and workload views, the rule engine and its snapshot harness, KL0001,
  KL1001, KL1002, human output, `kubelint check` with Helm chart skipping.
- **M2: Configuration, output formats and the rule set** (2026-11-11). `kubelint.toml`
  with rule selection, severity overrides, `fail-on` and `[files] exclude`; the matching
  CLI flags; suppression comments; `kubelint rules` and `kubelint rule`; parallel
  linting with deterministic output order; JSON, SARIF and GitHub output; "did you
  mean" suggestions; the quantity and image reference parsers and the name and label
  validators; KL0002, KL0003, KL1003 to KL1011, KL2001 to KL2009 and KL3001 to KL3005.
- **M3: Ready for real use** (2026-11-15). The release workflow; the `kubelint/ignore`
  annotation; `cargo xtask gen-rule-docs` with a staleness test; user docs (getting
  started, suppressions, CI integration); the benchmark corpus and a measurement well
  inside the budget; real-world validation over 956 public objects, with each false
  positive fixed (KL1004 only within one file, `\'` in double-quoted scalars, secret
  switches in KL2008, template placeholders in KL1008 and KL1010, KL2002 and KL2003 off
  by default); a review of every message, help line and explanation; the
  pre-commit hook and the GitHub Action.

## M4: Schema validation (target: 2026-12-08)

- [x] Design: the [schema validation](architecture.md#schema-validation) section of
  `architecture.md` decides the schema source (OpenAPI v3 of `kubernetes/kubernetes`),
  the version window (the latest 4 minor releases), the storage format (a stripped JSON
  form per version, committed and embedded) and the validator (span-aware, our own)
- [x] `kubelint_schema` crate: the in-memory schema model (object properties, required
  fields, scalar types, `oneOf`, lists and maps) and a lookup by `apiVersion` and
  `kind`, tested with small hand-written schemas
- [ ] `cargo xtask import-schemas`: turn the OpenAPI v3 files of one Kubernetes version,
  read from a local sparse checkout of its release tag, into
  `crates/kubelint_schema/schemas/<version>.json` and `schemas/sources.toml`, and a
  test that fails when a committed file was written by an older importer. The upstream
  files are not vendored (about 12 MB per version)
- [ ] Import every version of the window and embed it in the binary; record the binary
  size
- [ ] `--kubernetes-version` flag and `kubernetes-version` config key, defaulting to the
  latest supported version
- [ ] End to end CI job: split every rule fixture into objects, create every namespace
  they use, and check each case against a real API server
  (`.github/scripts/e2e-apiserver.sh`, every version of the window), judged on the
  rule's own field. A correctness finding is rejected with an error naming that field
  (KL1011 is accepted with a warning, KL1001 and KL1002 are rejected by `kubectl`
  itself, KL1004 needs both objects in one request). A Pod Security finding (`baseline`
  for KL2001 and KL2004 to KL2007, `restricted` for KL2002 and KL2003) names the check
  the server reports in a namespace labeled with that level: a Pod is rejected, a
  workload gets a warning. A case the rule passes does not trigger that check. Cases the
  server cannot judge (template placeholders, custom resources, schema problems) are
  marked in the fixture and skipped. A mismatch fails CI. Placed before the schema
  rules so they are checked against the API server from their first PR
- [ ] Span-aware validator that walks an object against its schema and stays silent on
  kinds it does not know (custom resources). User-provided CRD schemas come after M4
- [ ] KL1101 `unknown-field`, with "did you mean" suggestions
- [ ] KL1102 `invalid-field-type`, without reporting again the values KL1009 and KL1010
  already report
- [ ] KL1103 `missing-required-field`
- [ ] KL1106 `invalid-enum-value`, from a hand-written table of allowed values (the
  OpenAPI documents have none), starting with the fields users get wrong most
  (`imagePullPolicy`, `restartPolicy`, `protocol`, Service `type`)
- [ ] API version lifecycle table (deprecated in, removed in, replacement) and KL1104
  `removed-api-version` and KL1105 `deprecated-api-version`. Removed versions are absent
  from the OpenAPI files, so the table is written by hand from the Kubernetes
  deprecation guide
- [ ] Measure the lint time against the M3 benchmark with every schema rule enabled; fix
  hotspots if over budget
- [ ] User docs: choosing the Kubernetes version, and what schema validation covers

## M5: Autofix and editor support (outline)

- Fix infrastructure: apply non-overlapping edits, `--fix`, `--diff`, `--unsafe-fixes`.
- Safe fixes where one exists (for example quoting a KL1009 value, adding
  `allowPrivilegeEscalation: false` for KL2002).
- Choose the LSP library (documented in `architecture.md`), `kubelint_lsp` crate,
  `kubelint server` subcommand.
- VS Code extension in `editors/vscode/`.

## Known gaps

Small problems found along the way. Fix one when it fits a milestone, or schedule it.

- A file that is not valid UTF-8 stops the run with exit code 2. A per-file finding
  would be friendlier, but needs a new rule code. This includes UTF-16 with a byte order
  mark, which Windows PowerShell 5.1 writes with `>` and kubectl accepts, and it
  contradicts "invalid YAML is a finding" in `architecture.md`.
- KL0002 does not check the `items` wrapper of a `kind: List`.
- The YAML 1.1 scalar tables in `kubelint_yaml::resolve` were written from memory of
  `gopkg.in/yaml.v2` `resolve.go` and are not yet compared with the upstream source.
- An empty selector (`selector: {}`) on an `apps/v1` workload is rejected by the API
  server but not reported. The API server's validation code rejects it, not the OpenAPI
  schema, so M4 schema rules will not catch it: it needs its own correctness rule.
- A namespaced object without `namespace` and the same object with
  `namespace: default` are not reported as KL1004 duplicates.
- KL2007 reports a socket file only. A `hostPath` on a directory that holds one
  (`/var/run`, `/run/containerd`) gets the KL2005 warning, not the KL2007 error.
- KL1009, KL1011 and KL2008 each walk `env` and read `name` and `value` by hand. A
  `Container::env_vars()` view in `kubelint_k8s` would share that.
- KL2002 and KL2009 share the same lookup of one boolean `securityContext` field
  (wrong value, unset or null, or a schema problem). A helper in `security/mod.rs`
  would share it.

- KL0003 points every bad rule in a `kubelint/ignore` annotation at the whole value,
  not at the rule's own text, and says nothing about a value that is not a string or an
  annotation on a `kind: List` itself.

- KL1008 checks names only for the kinds in `BuiltinKind`. The API server also checks
  the names of other built-in kinds (`Node`, `EndpointSlice`) and of every custom
  resource as a DNS subdomain, so those could be checked with the "most kinds" format.

- The release workflow treats every change under `crates/` as code, so a change that
  touches only `crates/xtask` publishes a release with an unchanged binary.

### From the end to end audit of 2026-11-15

Each one was reproduced with the release binary of `78b2116`, and each Kubernetes fact
with `kubectl apply --dry-run=server` on v1.34.13 and v1.37.2. Most severe first.

- A YAML alias bomb exhausts memory and aborts the whole run: the builder copies the
  anchored node for every alias, so a 551 byte file of nested aliases needs gigabytes.
  kubectl stops it with "excessive aliasing". Keep an expansion budget and report
  KL0001.
- KL0002 recurses without a limit: a block sequence nested 30,000 deep (60 KB)
  overflows the stack and aborts the run, and 8,000 is enough in a worker thread when
  other files are linted.
- Merge keys (`<<: *anchor`) are not expanded. KL1005, KL1010 and KL1003 report errors
  on objects the server accepts (`labels: {<<: *sel, tier: x}`), and KL2001 misses a
  `privileged: true` that comes in through a merge.
- Directory search applies `.gitignore` patterns such as `gen/*` inside a directory
  named on the command line, so `kubelint check gen` checks 0 files and exits 0. A
  named directory that yields no file should at least be reported.
- KL1008 judges some names unlike the server. PodDisruptionBudget names are only path
  segments (`My_X` is accepted). The server ignores `metadata.namespace` on
  cluster-scoped kinds. StatefulSet names are DNS labels (no dots, at most 63
  characters), CronJob names have at most 52 characters and Job names at most 63.
  Service names may start with a digit from 1.36 (`RelaxedServiceNameValidation`) but
  not on 1.34 and 1.35, which needs the `--kubernetes-version` flag.
- Explicit YAML tags are dropped: `value: !!str 8080` is reported by KL1009 and KL1010
  although kubectl sends a string, and `!!int "8080"` is not reported although the
  server rejects it. `1e400` is read as a number, while kubectl keeps a float out of
  range as a string.
- KL2003 reports only a container that may run as root, but its catalog row and the end
  to end CI item say it checks the `restricted` requirement `runAsNonRoot`:
  `runAsUser: 1000` without `runAsNonRoot: true` passes while restricted rejects it. It
  also skips Windows pods (restricted requires `runAsNonRoot` there), reports pods with
  `hostUsers: false` (restricted accepts them from 1.35) and accepts a pod-level
  `runAsUser: 0` that every container overrides (restricted rejects it).
- The default security rules do not cover all of `baseline`, as the catalog implies:
  `hostPort`, an `Unconfined` seccomp or AppArmor profile, unsafe sysctls, SELinux types
  and `hostProcess` are rejected by baseline and reported by no rule (see Rule ideas).
- Block scalar (`|`, `>`) ranges end at the next token, so every format highlights the
  lines after the value.
- KL1005 returns early when `spec.template.spec` is missing, so its own explanation
  example is not reported, although the server rejects the selector.
- KL1007 misses PersistentVolume `spec.capacity`, generic ephemeral volume claim
  templates and HorizontalPodAutoscaler metric targets.
- An alias to an anchor in another document is read as null with no finding. kubectl
  fails with "unknown anchor".
- KL3005 misses `replicas: 1.0`, `1.` and `!!int "1"`, which the server stores as 1.
  KL3003 reports malformed references (`:latest`, `/app`, `app//x`) as unpinned.
- KL2008 reports env vars that hold a path
  (`GOOGLE_APPLICATION_CREDENTIALS: /var/secrets/key.json`) and misses the suffixes
  `PASS`, `PWD` and `PASSPHRASE`. KL2005 never reports a runtime socket, even when
  KL2007 is off.
- Rule explanations state facts the audit disproved. KL0001, KL1001, KL1002 and KL1003
  say nothing in the file is applied, but kubectl applies the documents before the
  broken one. KL0002 says kubectl rejects a duplicate key; it keeps the last value.
  KL1009 misquotes the server error. KL2001's "Use instead" adds `NET_ADMIN`, which
  KL2006 and baseline reject. KL2007 recommends Kaniko (archived) and a rootless BuildKit
  that does not start under the runtime default seccomp profile. KL3001 says the
  scheduler uses limits; it uses requests. KL3002 says BestEffort pods are evicted
  before all others; eviction orders by usage over requests, then priority. KL3005 says a
  Deployment rollout with one replica leaves no spare pod; `maxSurge` rounds up to 1.
  Several examples are not complete objects the server accepts (no `selector`,
  `containers` or `ports`).
- Output formats: the Actions runner shows at most 10 annotations per type in a step,
  which `output-formats.md` does not say. Standard input becomes a file named
  `<stdin>` in SARIF and GitHub output. A UTF-8 byte order mark counts as a column. A
  syntax error at the end of a file is at `4:12` in human output and `5:1` in the
  others. On Linux, `\` in a path is rewritten to `/` and a directory named `c:` is read
  as a drive. A closed pipe exits with code 2, or panics with 101 when stderr is closed
  too. `NO_COLOR=` (empty) disables color. The GitHub format passes control characters
  from YAML escapes into the log. SARIF output has no cap at GitHub's 25,000 results and
  no `partialFingerprints`.
- CLI: the same file under two spellings (`a.yaml ./a.yaml`) is linted twice. `exclude`
  patterns written `./vendor`, `vendor/` or `/vendor` match nothing. Flag selectors are
  not trimmed (`--select 'KL2001, KL3003'` is an error). `[lint.severity]` errors
  suggest categories, which it rejects. `kubelint-disable-next-line` before a blank line
  silences nothing, and a misspelled directive is silently a comment. A Go template
  outside a chart gets a bare KL0001 with no hint about Helm. CR-only line endings put
  every position on line 1.
- UTF-16 columns are counted from the line start for every finding, so 8,000 findings
  on one 255 KB line take 2.5 s.
- Releases and CI integration: the Windows binary needs `VCRUNTIME140.dll`, so it is not
  self-contained. `cargo install --git` uses the user's default toolchain, not the
  pinned one, and installs version `0.0.0`. The Action's `format: sarif` fails the step
  with nothing in the log, fails when `sarif-file` is in a missing directory and leaves
  an empty file on exit code 2. Action `args` take quotes and globs literally, and a
  `version` without `v` gives a bare 404. CI never runs the `@vX.Y.Z` pin path. The
  hook's `types: [yaml]` also lints `.clang-format` and similar files. The SARIF upload
  snippet lacks `actions: read`, which GitHub's docs ask for in private repositories
  (not run). `CLAUDE.md` says CI runs `cargo fmt --all`; CI runs it with `--check`, and
  its tests with `RUSTFLAGS=-D warnings`. `e2e-apiserver.sh 1.37` without network says
  the version is unknown.
- `CONTRIBUTING.md` step 5 runs only the `kubelint_linter` tests, which misses the CLI
  and corpus tests, and nothing ties the catalog in `docs/rules/README.md` to the
  registry.

## Rule ideas

Not in the catalog yet. Add one to [`rules/README.md`](rules/README.md) with a reason
before building it.

- `requests` greater than `limits`.
- Negative quantities.
- Non-string values in `ConfigMap.data` and in annotations.
- Annotation key syntax.
- `matchExpressions` key and value syntax.
- An `apiVersion` that is newer than the selected Kubernetes version (it exists in a
  later version of the window). The schema validator skips it like a custom resource.
- CronJob names longer than 52 characters.
- The `baseline` Pod Security checks that no rule covers: `hostPort`, an `Unconfined`
  seccomp or AppArmor profile, unsafe sysctls, SELinux type and user options,
  `hostProcess`, and `procMount` (rejected by baseline on 1.34 only).

## Backlog

Not scheduled. Items marked *non-goal* are excluded by [`vision.md`](vision.md) and are
never built. The others need an account or a token the routine does not have.

- Native Helm chart and Kustomize input (*non-goal*).
- Browser playground with WebAssembly (*non-goal*).
- User-defined rules or policies (*non-goal*).
- Publishing to crates.io (needs a crates.io token).
- A PyPI wheel, so the pre-commit hook installs kubelint itself (needs a PyPI account).
- Distribution through Homebrew, Krew or the VS Code marketplace (needs accounts).
