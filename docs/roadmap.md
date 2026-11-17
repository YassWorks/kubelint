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
  server cannot judge (template placeholders, custom resources, schema problems, files
  of other tools that KL1001 and KL1002 pass) are marked in the fixture and skipped. A
  mismatch fails CI. Placed before the schema rules so they are checked against the API
  server from their first PR
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
  contradicts "invalid YAML is a finding" in `architecture.md`. In a directory search,
  one such file stops the whole run, so no result comes out for any other file.
- KL0002 does not check the `items` wrapper of a `kind: List`.
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
  not at the rule's own text, and says nothing about a list or mapping value or an
  annotation on a `kind: List` itself. A number or boolean value gets "not a rule code or
  name" without saying it must be a string, and `null` or `~` is reported although the
  API server accepts it. A list separated by spaces (`KL2001 KL3001`, `KL2001 -- reason`)
  gets the same message, with no hint that rules are separated by commas.

- KL1008 checks names only for the kinds in `BuiltinKind`. The API server also checks
  the names of other built-in kinds (`Node`, `EndpointSlice`) and of every custom
  resource as a DNS subdomain, so those could be checked with the "most kinds" format.
- KL1004 and KL1008 know the scope of the kinds in `BuiltinKind` only. A built-in
  cluster-scoped kind outside it (`Node`, `RuntimeClass`, `CSINode`, `APIService` and
  the rest) keeps its written `metadata.namespace`: KL1008 reports an invalid one and
  KL1004 tells two such objects apart, although the API server ignores it. Adding the
  kinds fixes it.

- The release workflow treats every change under `crates/` as code, so a change that
  touches only `crates/xtask` publishes a release with an unchanged binary.

### YAML layer

- `saphyr-parser` stops flow collections nested deeper than 255 levels, so kubelint
  reports KL0001 on flow nesting from 256 to 9,999 levels that kubectl accepts. That
  error gets the "fix the YAML syntax" help, not the one for the alias and nesting limits.
- KL0002 recurses on nesting depth, so a deeply nested block sequence overflows the
  stack and aborts the run.
- Explicit tags (`!!str`, `!!int`) are ignored, and a float out of range (`1e400`) is
  read as a number where kubectl keeps a string.
- An alias to an anchor in another document is read as null, with no finding.
- A block scalar's range ends at the next token, so a finding on it highlights the
  lines after it.
- CR-only line endings put every position on line 1, and a UTF-8 byte order mark counts
  as a column.
- Mapping keys are read as written, but kubectl types them like values: `yes`, `y`, `on`
  and `true` become the key `"true"`, `1` and `01` become `"1"`, and a float key becomes a
  32-bit decimal (`2.718281828459045` becomes `"2.7182817"`). Of keys that collide this
  way, kubectl keeps the last one without a word, and KL0002 misses it. KL1010 reports
  label keys the server accepts (`.5` and `+1` arrive as `"0.5"` and `"1"`) and misses
  `1e20` (`"1e+20"`, rejected). KL1005 reports a selector `on: a` against a template
  label `"true": a`, which the server accepts. A null key (`~`) or a key in the unsigned
  64-bit range makes kubectl refuse the document, with no finding.
- A plain `.inf` or `.nan` anywhere makes kubectl refuse the file (`json: unsupported
  value: +Inf`), with no finding except where a rule reads the value as a number
  (KL1007, KL1009, KL1010).
- Because explicit tags are ignored, `!!str 12` as an env value gets KL1009 although
  kubectl sends a string, and `!!int "12"` or `!!bool "yes"` gets nothing although
  kubectl rejects it.
- A tab right after a block `-`, or after the `:` of a `? key` line, is accepted although
  kubectl refuses the file ("found character that cannot start any token").
- Each double-quoted scalar with a `\'` escape makes the builder parse the file again,
  so 5,000 such lines take about 7 seconds.

### Rules

- KL1008 disagrees with the server on two version dependent cases: it reports a Service
  name that starts with a digit, which is valid from 1.36, and it stays silent on a
  ReplicationController `generateName` with invalid characters when `name` is set, which
  1.34 rejects. Both need the `--kubernetes-version` flag.
- KL2003 reports only a container that may run as root, while its catalog row and the
  end to end CI item treat it as the `restricted` check `runAsNonRoot`. They differ on
  a non-zero `runAsUser` without `runAsNonRoot`, Windows pods, `hostUsers: false`
  (restricted accepts it from 1.35) and a pod-level `runAsUser: 0` that every container
  overrides. Decide which one the rule is.
- No rule covers several `baseline` checks (see Rule ideas), so the default security
  rules check only part of `baseline`.
- Some rule fixtures encode a verdict the API server gives differently, so the end to
  end CI item must mark or split them first. KL1001 and KL1002 pass a mapping that is
  not an object, which kubectl refuses (`apiVersion not set`). KL1005 passes a
  Deployment without a selector (`spec.selector: Required value`). KL1006 passes
  repeated empty or missing container names (`Duplicate value: ""`). KL1007 and KL1009
  pass a mapping or a list where a quantity or an env value goes, which the server
  rejects on that field as it rejects a bad value. The server never names the
  ReplicationController selector case of KL1010. It accepts the CronJob
  `jobTemplate.metadata.labels` case of KL1010, but the CronJob controller copies those
  labels into each Job, which the server rejects. KL2002 passes privileged and
  `CAP_SYS_ADMIN` containers, which `restricted` flags for `allowPrivilegeEscalation`.
- KL1011 passes a repeated env name made of spaces (`" "`), which the server warns about
  (`hides previous definition`).
- KL1010 checks workload and Service selectors only. PodDisruptionBudget, NetworkPolicy
  and PersistentVolumeClaim selectors, `matchExpressions`, `nodeSelector` and topology
  spread selectors are not checked, although the server rejects bad values there.
- KL1008 is silent on a `metadata.namespace` made of spaces, which the server answers
  with NotFound, and nothing reports a built-in kind written in the wrong case
  (`kind: configmap`), which the server rejects.
- KL3003 is silent on image references the server accepts but no registry serves
  (`nginx:`, `:1.0`, `Nginx:1.0`, `nginx::1`, a short digest) and on the ones the server
  rejects (an empty image, leading or trailing whitespace).
- `docs/rules/README.md` says Kubernetes expands `$(VAR)` only in `command`, `args` and
  `env`; the kubelet also expands it in `volumeMounts[].subPathExpr`.
- The "Use instead" examples of KL3001 and KL3002 each get the other rule's warning.
- KL1005 skips a pod template without `spec`. KL1007 misses PersistentVolume capacity,
  generic ephemeral volume claim templates and autoscaler targets. KL3005 misses `1.0`,
  `1.` and `!!int "1"`. KL3003 reports `nginx/`, `/nginx` and `a/b/` as unpinned. KL2008
  reports values that are file paths (`GOOGLE_APPLICATION_CREDENTIALS`) and misses the
  suffixes `PASS`, `PWD` and `PASSPHRASE`. KL2005 never reports a runtime socket, even
  when KL2007 is off.
- Some rule explanation examples are not complete objects. An explanation is right only
  once its examples ran against the API server or in a container.

### CLI and output

- The same file under two spellings is linted twice. `exclude` patterns with a leading
  `./` or `/` or a trailing `/` match nothing. Flag selectors are not trimmed.
  `[lint.severity]` errors suggest categories, which it rejects.
- `kubelint-disable-next-line` before a blank line silences nothing, a misspelled
  directive is a plain comment, and a Go template outside a chart gets a bare KL0001.
- The Actions runner shows at most 10 annotations per type in a step, which the docs do
  not say. Standard input appears as a file named `<stdin>` in SARIF and GitHub output.
  SARIF output has no cap at GitHub's 25,000 results and no `partialFingerprints`.
- Human output puts a position at the end of a file elsewhere than the other formats.
  On Linux, `\` in a path becomes `/` and a directory named `c:` reads as a drive. A
  closed pipe exits with code 2, or panics when stderr is closed too. An empty
  `NO_COLOR` disables color. YAML escapes put control characters into GitHub output.
- UTF-16 columns are counted from the line start for every finding, which is slow when
  thousands of findings share one long line.
- Human output renders every line between a finding and its related location, so its
  time grows with the square of the distance: 10,000 duplicate objects in one file
  (KL1004) take 6 seconds, against 0.15 seconds in JSON.
- Human output counts columns in characters, which the docs do not say.
- The error for a repeated `[lint.severity]` rule never names the other key.

### Releases and CI integration

- The Windows binary needs `VCRUNTIME140.dll`, so it is not self-contained.
- `cargo install --git` builds with the user's default toolchain, not the pinned one,
  and installs version `0.0.0`.
- The Action's SARIF mode prints nothing and fails the step when there are findings,
  fails on an output path in a missing directory, and leaves an empty file on exit code
  2. Its `args` take quotes and globs literally, a `version` without `v` gives a bare
  404, and CI never runs the path where a tag pins the version.
- The hook's `types: [yaml]` also matches `.clang-format` and similar files.
- The SARIF upload snippet may need `actions: read` in private repositories, as
  GitHub's docs say (not run).
- `CLAUDE.md` lists `cargo fmt --all` where CI runs it with `--check`, and CI runs the
  tests with `RUSTFLAGS=-D warnings`.
- `CONTRIBUTING.md` step 5 runs only the linter tests, and nothing ties the catalog in
  `docs/rules/README.md` to the registry.
- `e2e-apiserver.sh` reports a network failure as an unknown version.

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
