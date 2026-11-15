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
  would be friendlier, but needs a new rule code.
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

## Backlog

Not scheduled. Items marked *non-goal* are excluded by [`vision.md`](vision.md) and are
never built. The others need an account or a token the routine does not have.

- Native Helm chart and Kustomize input (*non-goal*).
- Browser playground with WebAssembly (*non-goal*).
- User-defined rules or policies (*non-goal*).
- Publishing to crates.io (needs a crates.io token).
- A PyPI wheel, so the pre-commit hook installs kubelint itself (needs a PyPI account).
- Distribution through Homebrew, Krew or the VS Code marketplace (needs accounts).
