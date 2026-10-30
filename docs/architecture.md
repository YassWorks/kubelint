# Architecture

This document describes how kubelint is built. It is the map every contributor (human or
agent) reads before touching code. The reasoning behind each choice lives in the ADRs in
[`decisions/`](decisions/). When the code and this document disagree, fix one of them in
the same PR.

## The pipeline

```text
 paths / stdin
      │
      ▼
 ┌─────────────┐  discover files, skip non-YAML, skip Helm templates      (kubelint CLI)
 │  discovery  │
 └─────┬───────┘
       ▼
 ┌─────────────┐  SourceFile { path, text }, LineIndex                    (kubelint_source)
 │   source    │
 └─────┬───────┘
       ▼
 ┌─────────────┐  YAML text ──► Vec<Document>, every node has a TextRange (kubelint_yaml)
 │    yaml     │  + comments (for suppressions) + parse errors
 └─────┬───────┘
       ▼
 ┌─────────────┐  Document ──► Object views (apiVersion, kind, metadata,  (kubelint_k8s)
 │     k8s     │  pod specs, containers). Non-Kubernetes docs are skipped.
 └─────┬───────┘
       ▼
 ┌─────────────┐  run enabled rules per object and across all objects,    (kubelint_linter)
 │   linter    │  apply config + suppression comments ──► Vec<Diagnostic>
 └─────┬───────┘
       ▼
 ┌─────────────┐  human (rustc-style) | json | sarif | github             (kubelint_output)
 │   output    │
 └─────┬───────┘
       ▼
   exit code 0 / 1 / 2                                                    (kubelint CLI)
```

Files are independent until the cross-object rules run, so parsing and per-object rules
run in parallel per file (`rayon`). Output is always sorted (file path, then byte offset,
then rule code) so results are deterministic regardless of thread scheduling.

## Crates

All crates live in `crates/` and are named `kubelint_*` (the CLI binary is just
`kubelint`). Every crate inherits version, edition and lints from the workspace.

| Crate | Responsibility | May depend on (internal) |
| --- | --- | --- |
| `kubelint_source` | `SourceFile`, `FileId`, `TextRange`/`TextSize` (re-exported from `text-size`), `LineIndex` (byte offset ↔ line/column, UTF-8 and UTF-16 columns for the future LSP). | none |
| `kubelint_yaml` | Span-preserving YAML tree built from `saphyr-parser` events. Comment extraction. Kubernetes-compatible scalar typing. | source |
| `kubelint_diagnostics` | `Diagnostic`, `Severity`, `Label`, `Fix`, `Edit`, `Applicability`. Data only, no rendering. | source |
| `kubelint_k8s` | `Object` view over a YAML document, `GroupVersionKind`, workload helpers (`pod_spec()`, `containers()`), Kubernetes value parsers (quantities, DNS-1123 names, label syntax). Later: schema validation data. | source, yaml |
| `kubelint_config` | Config file discovery and parsing (`kubelint.toml`, `.kubelint.toml`), CLI overrides, resolved `Settings`. Knows rule *selectors* as strings, not the rule registry. | source |
| `kubelint_linter` | `Rule` trait, `RuleMeta`, `Category`, the static rule registry, selector resolution, suppression comments, the lint engine, all built-in rules, and the rule test harness. | source, yaml, k8s, diagnostics, config |
| `kubelint_output` | Renderers: `human` (`annotate-snippets`), `json`, `sarif`, `github`. | source, diagnostics |
| `kubelint` | CLI: `clap` argument parsing, file discovery (`ignore` crate), stdin, orchestration, exit codes. Thin. | all of the above |

**Layering rule:** dependencies point downward in the table only. Lower crates never know
about higher ones. If a lower crate seems to need something from a higher one, the type
belongs lower, or the design needs an ADR.

Planned crates (create them when their milestone starts, not before):

- `kubelint_schema` (v0.2): bundled Kubernetes API schemas and the span-aware validator.
- `kubelint_lsp` (v0.3): language server.
- `xtask` (v0.1, M3): developer automation, starting with rule docs generation.

## Key design decisions

### Positions are byte offsets

Every location is a `TextRange` of byte offsets into the original file text. Line and
column numbers are computed only at the edges (rendering, SARIF, LSP) through `LineIndex`.

`saphyr-parser` reports positions as **character** indices, not bytes (verified on
2026-10-30 against version 0.3.3: a 2-byte `é` shifts later indices by one). The YAML
layer converts them to byte offsets. Every test file in `kubelint_yaml` must include
non-ASCII content to keep that conversion honest.

### Our own YAML tree

`kubelint_yaml` does not expose `saphyr` types. It builds its own tree:

```rust
pub struct Document { pub root: Option<Node>, pub range: TextRange }
pub struct Node { pub kind: NodeKind, pub range: TextRange }
pub enum NodeKind {
    Mapping(Vec<Entry>),   // entries in source order, duplicates kept (KL0002 needs them)
    Sequence(Vec<Node>),
    Scalar(Scalar),
    Null,                  // explicit `~`/`null` or an empty value
}
pub struct Entry { pub key: Node, pub value: Node }
pub struct Scalar { pub value: String, pub style: ScalarStyle /* plain, quoted, block */ }
```

The exact shape will evolve in M1, but the principles are fixed:

- **Every node knows its range,** including mapping keys, so a diagnostic can point at the
  key (`privileged`) or the value (`true`) as appropriate.
- **Collection ranges come from start and end events.** `saphyr-parser` gives
  `MappingStart`/`SequenceStart` an empty span, so the builder computes a collection's
  range from its first and last child.
- **Scalar typing follows Kubernetes, not YAML 1.2.** Kubernetes decodes YAML with a
  YAML 1.1 style library, where `yes`, `no`, `on`, `off`, `y` and `n` are booleans. The
  tree keeps the raw text and style, and offers `resolve_kubernetes()` helpers. This is
  what makes rules like KL1009 (`value: yes` in an env var) possible.
- **Aliases are resolved** to the anchored node, keeping the anchor's range. Manifests
  rarely use anchors, so this is simple and good enough until proven otherwise.
- **Comments are extracted separately.** The parser does not emit comments. A `#` that
  starts a line or follows whitespace, and is not inside any scalar's range, starts a
  comment to the end of the line. Suppression comments are read from that list.

### Kubernetes object views, not typed structs

kubelint does **not** deserialize manifests into typed Rust structs (`k8s-openapi`
style). Typed structs throw away positions and fail on the very mistakes a linter must
report. Instead `kubelint_k8s` offers thin views over the YAML tree:

```rust
let obj: Object = ...;
obj.kind()            // Option<&Scalar> with its range
obj.metadata_name()
obj.pod_spec()        // Some(PodSpec) for Pod, Deployment, StatefulSet, DaemonSet,
                      // ReplicaSet, ReplicationController, Job, CronJob
for c in pod_spec.containers() { c.kind(); c.name(); c.security_context(); }
                      // containers + initContainers + ephemeralContainers
```

Most security and reliability rules are "for every container, check X", so the workload
helpers are where the leverage is. A rule never hand-walks `spec.template.spec`.

**What counts as a Kubernetes object:** a document whose root is a mapping containing
`apiVersion` **or** `kind`. Documents with neither (GitHub workflow files, Helm
`values.yaml`, random config) are skipped silently, which is what makes zero-config
`kubelint check .` work. `kind: List` documents are expanded into their `items`.

### Rules

```rust
pub trait Rule: Sync {
    fn meta(&self) -> &'static RuleMeta;
    /// Called once per Kubernetes object.
    fn check_object(&self, cx: &mut ObjectContext<'_>) {}
    /// Called once per run with every object (cross-object rules like KL1004).
    fn check_all(&self, cx: &mut RunContext<'_>) {}
}

pub struct RuleMeta {
    pub code: &'static str,          // "KL2001"
    pub name: &'static str,          // "privileged-container"
    pub category: Category,          // Syntax | Correctness | Security | Reliability
    pub default_severity: Severity,
    pub default_enabled: bool,
    pub summary: &'static str,       // one line, shown in `kubelint rules`
    pub explanation: &'static str,   // markdown: why it matters, examples, how to fix
}
```

- Rules are unit structs, one per file: `crates/kubelint_linter/src/rules/<category>/<rule_name>.rs`.
- The registry is a single static slice in `rules/mod.rs`. Adding a rule = one new file
  + one line in the registry + one fixture + one snapshot + a row in
  [`rules/README.md`](rules/README.md). [`CONTRIBUTING.md`](CONTRIBUTING.md) walks through it.
- Rules report through the context (`cx.report(...)`), never by returning errors. A rule
  that cannot evaluate (missing fields) reports nothing; missing required fields are a
  different rule's job.
- Rules are pure: no I/O, no global state, no config reading beyond what the context
  passes in. That keeps them parallel-safe and trivially testable.
- Codes and names are permanent once released. See
  [ADR 0004](decisions/0004-rule-codes-and-categories.md).

### Diagnostics and exit codes

A `Diagnostic` has a rule code, severity, message, a primary label (range + optional
short text), optional secondary labels, optional help text and an optional `Fix` (a set
of text edits with an applicability level, unused until v0.3 autofix).

Severities are `error`, `warning`, `info`. The run fails when any diagnostic is at or
above `--fail-on` (default `warning`).

| Exit code | Meaning |
| --- | --- |
| 0 | No diagnostics at or above the fail threshold. |
| 1 | Lint findings at or above the fail threshold. |
| 2 | kubelint itself failed: bad arguments, unreadable config, I/O error. |

Invalid YAML is a finding (KL0001), not an exit-code-2 error, because it is a problem in
the user's files rather than in kubelint.

### Configuration and suppressions

See [ADR 0005](decisions/0005-configuration-and-suppressions.md). In short:

```toml
# kubelint.toml (or .kubelint.toml), found by walking up from the current directory
[lint]
select = ["correctness", "security", "KL3001"]   # categories, code prefixes, codes, names
ignore = ["KL2004"]
fail-on = "warning"

[lint.severity]
KL3003 = "error"

[files]
exclude = ["vendor/**"]
```

```yaml
securityContext:
  # kubelint-disable-next-line KL2001 -- the CNI plugin needs this
  privileged: true
  allowPrivilegeEscalation: true  # kubelint-disable-line privilege-escalation-allowed
```

`# kubelint-disable-file KL3001` anywhere in a file disables a rule for the whole file.

### Output formats

| `--format` | For | Notes |
| --- | --- | --- |
| `human` (default) | terminals | rustc-style snippets via `annotate-snippets`, colors when stdout is a TTY, summary line with timing. |
| `json` | scripts | Stable, documented schema. Breaking it is a breaking change. |
| `sarif` | GitHub code scanning and other security dashboards | SARIF 2.1.0, hand-written `serde` structs (no SARIF crate dependency). |
| `github` | GitHub Actions logs | `::error file=...,line=...,col=...::message` workflow commands. |

## Testing

See [ADR 0006](decisions/0006-testing-strategy.md).

- **Unit tests** live next to the code in `#[cfg(test)] mod tests`.
- **Rule tests** are snapshot tests: a fixture YAML file per rule in
  `crates/kubelint_linter/resources/test/fixtures/<category>/<CODE>.yaml` containing both
  violating and passing cases, linted with only that rule enabled, rendered as text and
  compared with `insta`.
- **CLI tests** in `crates/kubelint/tests/` run the real binary (`assert_cmd`) and
  snapshot stdout, stderr and the exit code.
- Snapshots are reviewed like code. Update with `INSTA_UPDATE=always cargo test`, then
  read the diff.

## Performance budget

- Lint 10,000 objects in under 1 second on a laptop, release build.
- No regex compilation per object (compile once, `std::sync::LazyLock`).
- No per-object allocation of the whole document. Views borrow from the tree.

A benchmark corpus and a check against this budget are part of the v0.1 roadmap.

## Dependencies

Pre-approved crates are declared in the root `Cargo.toml` under
`[workspace.dependencies]`. The policy for adding new ones is in
[`conventions.md`](conventions.md#dependencies).

| Crate | Used for |
| --- | --- |
| `saphyr-parser` | YAML event parsing with positions |
| `text-size` | `TextRange`/`TextSize` byte offsets (same types as rust-analyzer and Ruff) |
| `annotate-snippets` | rustc-style human output (the renderer rustc itself uses) |
| `clap` | CLI argument parsing (derive API) |
| `ignore` | fast, `.gitignore`-aware file discovery (from ripgrep) |
| `globset` | include/exclude globs in config |
| `rayon` | data parallelism over files |
| `serde`, `serde_json`, `toml` | config parsing and JSON/SARIF output |
| `thiserror` | error types in library crates |
| `anyhow` | error handling in the CLI binary only |
| `insta` | snapshot tests (dev-dependency) |
| `assert_cmd` | CLI integration tests (dev-dependency) |
