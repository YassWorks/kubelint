# Roadmap

This is the plan. Each unchecked task is sized to be **one pull request**. The autonomous
routine picks the **first unchecked task whose dependencies are done** in the current
milestone (see [`agent/playbook.md`](agent/playbook.md)).

Rules for editing this file:

- Tick a task (`[x]`) in the same PR that completes it, and append the PR number:
  `- [x] **M1-01** ... (#12)`.
- A task that turns out too big is split in place into `M1-04a`, `M1-04b`, ... in a docs
  PR before work starts. Never renumber existing tasks.
- New tasks go at the end of their milestone with the next free number.
- Milestones M4 and later are outlines. Break the next one into tasks (docs PR) when the
  current milestone has five or fewer open tasks.
- Changing milestone scope or order needs an entry in
  [`agent/owner-inbox.md`](agent/owner-inbox.md) explaining why.

## Current focus

**M1: Walking skeleton.** Goal: `kubelint check <path>` runs end to end and prints a
rustc-style diagnostic for a real rule.

---

## M0: Foundation (done, 2026-10-30)

- [x] **M0-01** Vision, architecture, ADRs, conventions, roadmap, rule catalog.
- [x] **M0-02** Cargo workspace with one crate per layer, pinned toolchain, workspace lints.
- [x] **M0-03** CI (fmt, clippy, tests on Linux/macOS/Windows, docs) and PR template.
- [x] **M0-04** Agent playbook, owner inbox, journal, concept glossary.

## M1: Walking skeleton (target: 2026-11-06)

- [x] **M1-01** (#2) `kubelint_source`: `SourceFile` (path + text), `FileId`, re-export
  `TextRange`/`TextSize` from `text-size`, and `LineIndex` (byte offset ↔ 1-based line and
  column, with columns in UTF-8 bytes and UTF-16 code units). Tests: ASCII, non-ASCII,
  CRLF, empty file, offset at end of file. _Depends on: none._
- [x] **M1-02a** (#3) `kubelint_yaml`: tree types (`Document`, `Node`, `NodeKind`, `Entry`,
  `Scalar`, `ScalarStyle`) and `Document::dump()` for snapshot tests. _Depends on: M1-01._
- [x] **M1-02b** (#6) `kubelint_yaml`: builder over `saphyr-parser` events. Converts character
  indices to byte offsets. Collection ranges from first and last child (collection start
  events have empty spans). Multi-document streams. Parse errors as
  `ParseError { message, range }`. Snapshot tests of the dump, including non-ASCII, flow
  style, block scalars and empty documents. _Depends on: M1-02a._
- [x] **M1-03** (#7) `kubelint_yaml`: navigation API: `get(key)`, `get_path(&[..])`,
  entry iteration, `as_str`/`as_mapping`/`as_sequence`, key ranges. Doc examples
  (doctests) on every public method. _Depends on: M1-02b._
- [ ] **M1-04** `kubelint_yaml`: Kubernetes scalar resolution (`null`/bool/int/float/string
  the way Kubernetes' YAML decoder sees them, YAML 1.1 style: `yes`/`no`/`on`/`off`/`y`/`n`
  are booleans). Table-driven tests. Cite the upstream source of the rules in the doc
  comment. _Depends on: M1-02b._
- [ ] **M1-05** `kubelint_yaml`: comment extraction (`Comment { range, text }`) using the
  rule in `architecture.md`. Tests with `#` inside quoted strings and block scalars.
  _Depends on: M1-02b._
- [x] **M1-06** (#4) `kubelint_diagnostics`: `Diagnostic`, `Severity`, `Label`, `Fix`, `Edit`,
  `Applicability`, with a small builder API. _Depends on: M1-01._
- [ ] **M1-07** `kubelint_k8s`: `Object` view (apiVersion, kind, metadata name/namespace/
  labels/annotations with ranges), `GroupVersionKind` parsing (`v1` is the core group),
  the "is this a Kubernetes object" rule, and `kind: List` expansion.
  _Depends on: M1-03._
- [ ] **M1-08** `kubelint_k8s`: workload helpers: `pod_spec()` for Pod, Deployment,
  StatefulSet, DaemonSet, ReplicaSet, ReplicationController, Job and CronJob;
  `containers()` over containers, initContainers and ephemeralContainers with
  `ContainerKind`; container accessors (name, image, securityContext, resources, env,
  probes). One fixture per workload kind. _Depends on: M1-07._
- [ ] **M1-09** `kubelint_linter`: `Rule` trait, `RuleMeta`, `Category`, static registry,
  `ObjectContext`/`RunContext`, a sequential engine, and the rule snapshot test harness
  (fixture path in, rendered diagnostics snapshot out). _Depends on: M1-06, M1-07._
- [ ] **M1-10** First rules: KL1001 `missing-api-version` and KL1002 `missing-kind`
  (one PR, they are mirror images). Establishes the rule file layout, fixture layout and
  snapshot style that every later rule copies. _Depends on: M1-09._
- [ ] **M1-11** KL0001 `invalid-yaml`: parse errors become diagnostics; a file with a YAML
  error still reports findings for its other valid documents where possible.
  _Depends on: M1-09._
- [ ] **M1-12** `kubelint_output`: human renderer with `annotate-snippets` (snippet,
  labels, help, rule code and name, summary line with counts and timing). Color on/off as a
  parameter. Snapshot tests without color. _Depends on: M1-06._
- [ ] **M1-13** `kubelint` CLI: `kubelint check [PATHS]...` with `clap`; file discovery
  via `ignore` (`*.yaml`, `*.yml`, honors `.gitignore`); `-` reads stdin;
  `--color auto|always|never`; exit codes 0/1/2. CLI snapshot tests with `assert_cmd`.
  _Depends on: M1-10, M1-11, M1-12._
- [ ] **M1-14** CLI: skip Helm chart templates (a directory with `Chart.yaml` has its
  `templates/` skipped with one friendly note suggesting `helm template ... | kubelint
  check -`); summary line reports files checked, objects checked, files skipped.
  _Depends on: M1-13._
- [ ] **M1-15** Docs: README quickstart with real output from the binary; verify
  `CONTRIBUTING.md` "Adding a rule" against the real code and fix any drift.
  _Depends on: M1-13._

## M2: Configuration, output formats and the v0.1 rule set (target: 2026-11-17)

Engine and user experience:

- [ ] **M2-01** KL0002 `duplicate-key`. _Depends on: M1-13._
- [ ] **M2-02** Rule selection: resolve selectors (code, code prefix like `KL2`, category,
  rule name) against the registry; default set; per-rule severity overrides; `fail-on`.
  Pure logic with exhaustive tests. _Depends on: M1-13._
- [ ] **M2-03** `kubelint_config`: discover `kubelint.toml`/`.kubelint.toml` walking up
  from the working directory; parse with precise errors for unknown keys; `--config
  <path>`, `--isolated`; CLI `--select`, `--ignore`, `--fail-on` override the file.
  _Depends on: M2-02._
- [ ] **M2-04** Suppression comments: `kubelint-disable-next-line`,
  `kubelint-disable-line`, `kubelint-disable-file`, codes or names, optional
  `-- reason`. Unknown codes in a suppression produce a warning. _Depends on: M1-05, M2-02._
- [ ] **M2-05** `kubelint rules` (table; `--format json`) and `kubelint rule <code|name>`
  (prints the explanation). _Depends on: M2-02._
- [ ] **M2-06** Parallel linting with `rayon` and deterministic output order (path, offset,
  code). Test that output is identical across runs. _Depends on: M1-13._
- [ ] **M2-07** `--format json` with a documented, versioned schema in
  `docs/user/output-formats.md`. _Depends on: M1-13._
- [ ] **M2-08** `--format sarif` (SARIF 2.1.0) with structural tests and docs for GitHub
  code scanning upload. _Depends on: M2-07._
- [ ] **M2-09** `--format github` (workflow commands) with docs. _Depends on: M2-07._
- [ ] **M2-10** "Did you mean" suggestions for unknown rule codes or names in config, CLI
  and suppressions (`strsim`). _Depends on: M2-03, M2-04._

Correctness rules (one PR each):

- [ ] **M2-11** KL1003 `missing-object-name`. _Depends on: M1-10._
- [ ] **M2-12** KL1004 `duplicate-object` (first cross-object rule, uses `check_all`).
  _Depends on: M1-10._
- [ ] **M2-13** KL1005 `selector-mismatch`. _Depends on: M1-10._
- [ ] **M2-14** KL1006 `duplicate-container-name`. _Depends on: M1-10._
- [ ] **M2-15** `kubelint_k8s`: resource quantity parser matching Kubernetes'
  `resource.Quantity` grammar, with exhaustive tests. _Depends on: M1-07._
- [ ] **M2-16** KL1007 `invalid-resource-quantity`. _Depends on: M2-15._
- [ ] **M2-17** `kubelint_k8s`: validators for DNS-1123 subdomain/label names and label
  keys/values, with tests mirroring Kubernetes' own validation messages.
  _Depends on: M1-07._
- [ ] **M2-18** KL1008 `invalid-object-name`. _Depends on: M2-17._
- [ ] **M2-19** KL1010 `invalid-label`. _Depends on: M2-17._
- [ ] **M2-20** KL1009 `non-string-env-value`. _Depends on: M1-04, M1-10._
- [ ] **M2-21** KL1011 `duplicate-env-name`. _Depends on: M1-10._

Security rules (one PR each):

- [ ] **M2-22** KL2001 `privileged-container`. _Depends on: M1-10._
- [ ] **M2-23** KL2002 `privilege-escalation-allowed`. _Depends on: M1-10._
- [ ] **M2-24** KL2003 `run-as-root` (container settings override pod settings).
  _Depends on: M1-10._
- [ ] **M2-25** KL2004 `host-namespace`. _Depends on: M1-10._
- [ ] **M2-26** KL2005 `host-path-volume`. _Depends on: M1-10._
- [ ] **M2-27** KL2006 `dangerous-capability`. _Depends on: M1-10._
- [ ] **M2-28** KL2007 `docker-socket-mount`. _Depends on: M2-26._
- [ ] **M2-29** KL2008 `secret-in-env-literal`. _Depends on: M1-10._
- [ ] **M2-30** KL2009 `writable-root-filesystem` (off by default). _Depends on: M1-10._

Reliability rules (one PR each):

- [ ] **M2-31** KL3001 `missing-memory-limit`. _Depends on: M1-10._
- [ ] **M2-32** KL3002 `missing-resource-requests`. _Depends on: M1-10._
- [ ] **M2-33** KL3003 `unpinned-image-tag`, with an image reference parser that handles
  registry ports (`host:5000/app`) and digests. _Depends on: M1-10._
- [ ] **M2-34** KL3004 `missing-readiness-probe` (off by default). _Depends on: M1-10._
- [ ] **M2-35** KL3005 `single-replica` (off by default). _Depends on: M1-10._

## M3: v0.1 release readiness (target: 2026-11-24)

- [ ] **M3-01** `xtask` crate with `cargo xtask gen-rule-docs` writing `docs/rules/<CODE>.md`
  from `RuleMeta`; CI job that fails if generated docs are stale. _Depends on: M2-05._
- [ ] **M3-02** User docs: `docs/user/getting-started.md`, `configuration.md`,
  `suppressions.md`, `ci-integration.md`; README links to them. _Depends on: M2-04, M2-08._
- [ ] **M3-03** Benchmark corpus generator (`cargo xtask gen-corpus`) and a documented
  measurement against the 10,000-objects-under-1-second budget; fix hotspots if over.
  _Depends on: M2-06._
- [ ] **M3-04** Real-world validation: run kubelint over public manifest collections,
  record false positives in `docs/agent/validation.md`, fix them (one PR per fix).
  _Depends on: M2-35._
- [ ] **M3-05** Diagnostic polish: review every rule's message, help and explanation
  against the "explains itself" bar in `vision.md`. _Depends on: M2-35._
- [ ] **M3-06** `.pre-commit-hooks.yaml` and docs. _Depends on: M1-13._
- [ ] **M3-07** Release automation: `dist` (cargo-dist) config and a release workflow
  triggered by a version tag; crate metadata complete; `cargo publish --dry-run` packaging
  check in CI. Publishing itself is an owner action. _Depends on: M1-13._
- [ ] **M3-08** GitHub Action (`action.yml`, composite: install release binary, run with
  `--format github` or upload SARIF) and docs. _Depends on: M3-07._
- [ ] **M3-09** `CHANGELOG.md` generated from Conventional Commits (`git-cliff` config)
  and `docs/release.md`, the owner's release checklist. _Depends on: M3-07._
- [ ] **M3-10** Owner: review, tag `v0.1.0`, publish. (Agent: prepare everything, then
  add a "ready to release" entry to the owner inbox.) _Depends on: all of M3._

## M4: v0.2 Schema validation (outline)

Break into tasks when M3 has five or fewer open tasks.

- ADR: schema source (Kubernetes OpenAPI v3 published in `kubernetes/kubernetes`
  `api/openapi-spec/v3`), supported version window (latest 4 to 5 minor versions), storage
  format (generated, compact, embedded in the binary), validator design (span-aware, our
  own, not a generic JSON Schema crate).
- `kubelint_schema` crate and an `xtask` importer.
- `--kubernetes-version` flag and config key; a sensible default (latest supported).
- Rules KL1101 to KL1106, with "did you mean" for unknown fields.
- Custom resources: unknown kinds are skipped by schema rules; user-provided CRD schemas
  are a later feature.

## M5: v0.3 Autofix and editor support (outline)

- Fix infrastructure: apply non-overlapping edits, `--fix`, `--diff`, `--unsafe-fixes`.
- Safe fixes for the rules where one exists (for example quoting a KL1009 value, adding
  `allowPrivilegeEscalation: false` for KL2002).
- ADR choosing the LSP library; `kubelint_lsp` crate; `kubelint server` subcommand.
- VS Code extension in `editors/vscode/` (publishing to the marketplace is an owner
  action).

## Backlog (not scheduled, needs owner approval)

Ideas outside the locked scope in [`vision.md`](vision.md). Add here, do not build.

- Native Helm chart and Kustomize input.
- Browser playground (WebAssembly).
- User-defined rules or policies.
- Distribution via Homebrew, Krew, container image.
