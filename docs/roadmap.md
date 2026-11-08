# Roadmap

What comes next, in order. Within a milestone, items are listed so that each one only
needs the items above it. Work on the first unchecked item of the current milestone.

- Tick an item in the pull request that completes it.
- An item too big for one pull request is split in place into smaller items first.
- New items go at the end of their group, with a short reason when it is not obvious.
- When the current milestone has five or fewer open items, break the next outline into
  items.

## Current focus

**M2: Configuration, output formats and the rule set.** The rules, configuration file,
suppressions and machine-readable output that make kubelint useful in CI.

## Done

- **M0: Foundation** (2026-10-30). Vision, architecture, conventions, rule catalog,
  Cargo workspace with one crate per layer, CI on Linux, macOS and Windows.
- **M1: Walking skeleton** (2026-11-03). YAML tree with byte ranges and error recovery,
  Kubernetes object and workload views, the rule engine and its snapshot harness, KL0001,
  KL1001, KL1002, human output, `kubelint check` with Helm chart skipping.

## M2: Configuration, output formats and the rule set (target: 2026-11-17)

Engine and user experience:

- [x] KL0002 `duplicate-key`
- [x] Rule selection: selectors, default set, severity overrides, `fail-on`
- [x] `kubelint.toml` discovery and parsing with precise errors
- [x] `--config`, `--isolated`, `--select`, `--ignore`, `--fail-on`
- [x] `[files] exclude` globs
- [x] Suppression comments, and KL0003 `invalid-suppression`
- [x] `kubelint rules` and `kubelint rule <code|name>`
- [x] Parallel linting with deterministic output order
- [x] `--format json`, `--format sarif`, `--format github`
- [x] "Did you mean" suggestions for unknown rule codes and names

Correctness rules:

- [x] KL1003 `missing-object-name`
- [x] KL1004 `duplicate-object`
- [x] KL1005 `selector-mismatch`
- [x] KL1006 `duplicate-container-name`
- [x] Resource quantity parser matching Kubernetes' `resource.Quantity`
- [x] KL1007 `invalid-resource-quantity`
- [x] Validators for DNS-1123 names and label syntax
- [x] KL1008 `invalid-object-name`
- [x] KL1010 `invalid-label`
- [x] KL1009 `non-string-env-value`
- [x] KL1011 `duplicate-env-name`: the same env `name` twice in one container's `env`.
  Report the later one with a "first set here" label, like KL1006. `envFrom` is out of
  scope.

Security rules:

- [x] KL2001 `privileged-container`
- [x] KL2002 `privilege-escalation-allowed`
- [ ] KL2003 `run-as-root`: container settings override pod settings. Add a helper in
  `kubelint_k8s` that merges the pod and container `securityContext`.
- [ ] KL2004 `host-namespace`
- [ ] KL2005 `host-path-volume`
- [ ] KL2006 `dangerous-capability`
- [ ] KL2007 `docker-socket-mount`
- [ ] KL2008 `secret-in-env-literal`
- [ ] KL2009 `writable-root-filesystem` (off by default)

Reliability rules:

- [ ] KL3001 `missing-memory-limit`
- [ ] KL3002 `missing-resource-requests`
- [ ] KL3003 `unpinned-image-tag`, with an image reference parser that handles registry
  ports (`host:5000/app`) and digests
- [ ] KL3004 `missing-readiness-probe` (off by default)
- [ ] KL3005 `single-replica` (off by default)

## M3: Ready for real use (target: 2026-11-24)

- [x] Release workflow: every merge to `main` that changes code builds binaries for
  Linux, macOS and Windows and publishes a GitHub Release
- [ ] `kubelint/ignore` annotation to silence rules on one object (designed in
  [`user/configuration.md`](user/configuration.md#annotations-planned))
- [ ] `xtask` crate with `cargo xtask gen-rule-docs` writing `docs/rules/<CODE>.md` from
  each rule, and a CI check that fails when the pages are stale
- [ ] User docs: getting started, suppressions, CI integration; the README links to them
- [ ] Benchmark corpus generator (`cargo xtask gen-corpus`) and a measurement against
  the 10,000 objects in under a second budget; fix hotspots if over
- [ ] Real-world validation: run kubelint over public manifest collections and fix each
  false positive in its own pull request
- [ ] Diagnostic polish: review every rule's message, help and explanation against the
  "explains itself" bar in [`vision.md`](vision.md)
- [ ] `.pre-commit-hooks.yaml` and docs
- [ ] GitHub Action (`action.yml`, composite: install the release binary, run with
  `--format github` or upload SARIF) and docs

## M4: Schema validation (outline)

- Decide and document in `architecture.md`: the schema source (Kubernetes OpenAPI v3 in
  `kubernetes/kubernetes` `api/openapi-spec/v3`), the supported version window (latest
  4 to 5 minor versions), the storage format (generated, compact, embedded in the
  binary) and the validator design (span-aware, our own, not a generic JSON Schema
  crate).
- `kubelint_schema` crate and an `xtask` importer.
- `--kubernetes-version` flag and config key, defaulting to the latest supported
  version.
- Rules KL1101 to KL1106, with "did you mean" for unknown fields. KL1102
  `invalid-field-type` must not report again the values KL1009 and KL1010 already
  report.
- Custom resources: unknown kinds are skipped by schema rules. User-provided CRD schemas
  come later.

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
- `[files] exclude` filters files after walking excluded directories, and the CLI reads
  files one by one. Measure both in the M3 benchmark.
- The YAML 1.1 scalar tables in `kubelint_yaml::resolve` were written from memory of
  `gopkg.in/yaml.v2` `resolve.go` and are not yet compared with the upstream source.
- An empty selector (`selector: {}`) on an `apps/v1` workload is rejected by the API
  server but not reported (schema validation, M4).
- A namespaced object without `namespace` and the same object with
  `namespace: default` are not reported as KL1004 duplicates.
- The rule snapshot harness renders secondary labels with the primary file's text, so
  cross-file findings need unit tests.

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
