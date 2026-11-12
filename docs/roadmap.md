# Roadmap

What comes next, in order. Within a milestone, items are listed so that each one only
needs the items above it. Work on the first unchecked item of the current milestone.

- Tick an item in the pull request that completes it.
- An item too big for one pull request is split in place into smaller items first.
- New items go at the end of their group, with a short reason when it is not obvious.
- When the current milestone has five or fewer open items, break the next outline into
  items.

## Current focus

**M3: Ready for real use.** Per-object suppression, generated rule docs, user docs,
a performance budget, real-world validation, diagnostic polish, and pre-commit and
GitHub Action integration.

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

## M3: Ready for real use (target: 2026-11-24)

- [x] Release workflow: every merge to `main` that changes code builds binaries for
  Linux, macOS and Windows and publishes a GitHub Release
- [x] `kubelint/ignore` annotation to silence rules on one object (designed in
  [`user/suppressions.md`](user/suppressions.md#annotations))
- [x] `xtask` crate with `cargo xtask gen-rule-docs` writing `docs/rules/<CODE>.md` from
  each rule, and a CI check that fails when the pages are stale
- [x] User docs: getting started, suppressions, CI integration; the README links to them
- [x] Benchmark corpus generator (`cargo xtask gen-corpus`) and a measurement against
  the 10,000 objects in under a second budget; fix hotspots if over
- Real-world validation: run kubelint over public manifest collections and fix each
  false positive in its own pull request. The first run (2026-11-12) covered
  `kubernetes/examples`, the examples of `kubernetes/website`,
  `GoogleCloudPlatform/microservices-demo` and `argoproj/argocd-example-apps`: 956
  objects in 785 files. Its false positives, one item each:
  - [x] KL1004 compared objects across files, and reported variants of one application
    kept in separate files or directories (209 findings, all across files)
  - [ ] KL0001 rejects `\'` in a double-quoted scalar, which the YAML parsers kubectl
    uses (libyaml and `go-yaml`) accept (`pods/inject/envars-file-container.yaml` in
    `kubernetes/website`)
  - [x] KL2008 reports a flag whose name contains a secret word, such as
    `MYSQL_ALLOW_EMPTY_PASSWORD: "1"`
  - [ ] KL1008 and KL1010 report placeholders of other template tools (`etcd-{{cell}}`,
    `process-item-$ITEM`), which a rule that cannot decide should leave alone
  - [ ] KL2002 and KL2003 noise: 415 and 426 findings in 956 objects. Decide whether
    either should be off by default (see the known gap)
- [ ] Diagnostic polish: review every rule's message, help and explanation against the
  "explains itself" bar in [`vision.md`](vision.md)
- [ ] `.pre-commit-hooks.yaml` and docs
- [ ] GitHub Action (`action.yml`, composite: install the release binary, run with
  `--format github` or upload SARIF) and docs

## M4: Schema validation (target: 2026-12-08)

- [ ] Design: a schema section in [`architecture.md`](architecture.md) that decides the
  schema source (Kubernetes OpenAPI v3 in `kubernetes/kubernetes` `api/openapi-spec/v3`),
  the supported version window (latest 4 to 5 minor versions), the storage format
  (generated, compact, embedded in the binary) and the validator design (span-aware, our
  own, not a generic JSON Schema crate)
- [ ] `kubelint_schema` crate: the in-memory schema model (object properties, required
  fields, scalar types, enums, lists and maps) and a lookup by `apiVersion` and `kind`,
  tested with small hand-written schemas
- [ ] `cargo xtask import-schemas`: turn the OpenAPI v3 files of one Kubernetes version,
  vendored under `crates/kubelint_schema/openapi/<version>/`, into the storage format,
  and a test that fails when the committed output is stale
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
- [ ] KL1106 `invalid-enum-value`
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
- KL2002 and KL2003 are on by default. A container that sets neither
  `allowPrivilegeEscalation: false` nor `runAsNonRoot: true` (in the container or the
  pod) gets both warnings, which is common in typical manifests. Measure their noise
  during the M3 real-world validation and decide whether either should be off by
  default.
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
- CronJob names longer than 52 characters.

## Backlog

Not scheduled. Items marked *non-goal* are excluded by [`vision.md`](vision.md) and are
never built. The others need an account or a token the routine does not have.

- Native Helm chart and Kustomize input (*non-goal*).
- Browser playground with WebAssembly (*non-goal*).
- User-defined rules or policies (*non-goal*).
- Publishing to crates.io (needs a crates.io token).
- Distribution through Homebrew, Krew or the VS Code marketplace (needs accounts).
