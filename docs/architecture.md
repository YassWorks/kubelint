# Architecture

This document describes how kubelint is built. It is the map every contributor reads
before touching code. Each decision says why it was made and what was rejected. When the
code and this document disagree, fix one of them in the same PR.

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
run in parallel per file (`rayon`). Output is always sorted (files in argument order, the
files found in a directory by path, then byte offset, then rule code) so results are
deterministic regardless of thread scheduling.

## Crates

All crates live in `crates/` and are named `kubelint_*` (the CLI binary is just
`kubelint`, and the developer tasks are `xtask`). Every crate inherits version, edition and lints from the workspace.

| Crate | Responsibility | May depend on (internal) |
| --- | --- | --- |
| `kubelint_source` | `SourceFile`, `FileId`, `TextRange`/`TextSize` (re-exported from `text-size`), `LineIndex` (byte offset ↔ line/column, UTF-8 and UTF-16 columns for the future LSP). | none |
| `kubelint_yaml` | Span-preserving YAML tree built from `saphyr-parser` events. Comment extraction. Kubernetes-compatible scalar typing. | source |
| `kubelint_diagnostics` | `Diagnostic`, `Severity`, `Label`, `Fix`, `Edit`, `Applicability`. Data only, no rendering. | source |
| `kubelint_k8s` | `Object` view over a YAML document, `GroupVersionKind`, workload helpers (`pod_spec()`, `containers()`), Kubernetes value parsers (quantities, image references, DNS-1123 names, label syntax). | source, yaml |
| `kubelint_schema` | The Kubernetes API schema of one version (`Schema`, `Shape`), read from the compact JSON form, and a lookup by group, version and kind ([schema validation](#schema-validation)). | source, yaml, k8s |
| `kubelint_config` | Config file discovery and parsing (`kubelint.toml`, `.kubelint.toml`), CLI overrides, resolved `Settings`. Knows rule *selectors* as strings, not the rule registry. | source |
| `kubelint_linter` | `Rule` trait, `RuleMeta`, `Category`, the static rule registry, selector resolution, suppression comments and annotations, the lint engine (with the Kubernetes version it checks against), all built-in rules, and the rule test harness. | source, yaml, k8s, diagnostics, schema, config |
| `kubelint_output` | Renderers: `human` (`annotate-snippets`), `json`, `sarif`, `github`. | source, diagnostics |
| `kubelint` | CLI: `clap` argument parsing, file discovery (`ignore` crate, Helm chart detection), stdin, orchestration, exit codes, and [`kubelint self update`](#self-update), the only code that uses the network. Thin. | all of the above |
| `xtask` | Developer automation run with `cargo xtask <task>`: `gen-rule-docs` writes `docs/rules/<CODE>.md`, `gen-corpus` writes the [benchmark corpus](#measuring-it), `import-schemas` writes `crates/kubelint_schema/schemas/` ([importing a version](#importing-a-version)), `e2e` checks rule fixtures against a real API server ([the fixture check](#the-fixture-check)). Not part of the release binary. | source, yaml, linter, schema |

**Layering rule:** dependencies point downward in the table only. Lower crates never know
about higher ones. If a lower crate seems to need something from a higher one, the type
belongs lower, or the layering changes here first.

Planned crates (create them when their milestone starts, not before):

- `kubelint_lsp` (M5): language server.

Publishing the CLI to crates.io means publishing every internal crate too. They are
internal, with no stability guarantee.

**Why one crate per layer:** boundaries are enforced by the compiler, crates compile in
parallel, and the language server will not pull in CLI-only dependencies.

**Rejected:**

- A single crate with modules. Module boundaries are not enforced.
- A few large crates such as `core` and `cli`. `core` becomes the tangle. Ruff and
  rust-analyzer use many small crates for this reason.

## Key design decisions

### Positions are byte offsets

Every location is a `TextRange` of byte offsets into the original file text. Line and
column numbers are computed only at the edges (rendering, SARIF, LSP) through `LineIndex`.

`saphyr-parser` reports positions as **character** indices, not bytes (verified on
2026-10-30 against version 0.3.3: a 2-byte `é` shifts later indices by one). The YAML
layer converts them to byte offsets. Every test file in `kubelint_yaml` must include
non-ASCII content to keep that conversion honest.

### Our own YAML tree

`kubelint_yaml` parses with `saphyr-parser`, the maintained successor of `yaml-rust`,
using its event stream, which carries a span for every scalar and collection boundary.
It builds its own tree and never exposes `saphyr` types, so the parser can be replaced
without touching rules:

```rust
pub struct Document { pub root: Option<Node>, pub range: TextRange }
pub struct Node { pub kind: NodeKind, pub range: TextRange }
pub enum NodeKind {
    Mapping(Vec<Entry>),   // entries in source order, duplicates kept (KL0002 needs them)
    Sequence(Vec<Node>),
    Scalar(Scalar),
    Null,                  // a missing value (`key:`); an explicit `~` is a plain scalar
}
pub struct Entry { pub key: Node, pub value: Node }
pub struct Scalar { pub value: String, pub style: ScalarStyle /* plain, quoted, block */ }
```

**Why:** a linter must point at the exact key or value that is wrong, and must see the
file as written (duplicate keys, quoting style, comments). `serde_yaml` is archived and
drops positions. Typed Kubernetes structs drop positions and fail on exactly the broken input
a linter exists to explain.

**Rejected:**

- `saphyr`'s higher-level `MarkedYaml`. It has spans, but less control over duplicate
  keys, key spans and styles, and it would tie the public API to its types.
- `yaml-rust2`. Maintained, but its maintainers now focus on `saphyr`.
- Writing our own YAML parser. Maximum control, very high cost. Worth revisiting only if
  autofix or the language server proves the event stream insufficient.

Principles:

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
  rarely use anchors, so this is simple and good enough until proven otherwise. An alias
  is expanded only within the limits below.
- **Comments are extracted separately.** The parser does not emit comments. A `#` that
  starts a line or follows whitespace, and is not inside any scalar's range, starts a
  comment to the end of the line. Suppression comments are read from that list.
- **Recovery after a syntax error.** `saphyr-parser` cannot continue after an error, so
  the document containing it is dropped and parsing restarts at the next document marker
  (`---` at the start of a line, which YAML never allows inside a scalar). Every other
  document of the file is still linted. Comments between the error and that marker are
  not reported, because scalar ranges there are unknown.
- **Quoted scalar ranges end at the closing quote.** `saphyr-parser` reports a quoted
  scalar's span running on over trailing spaces and a trailing comment, so the builder
  finds the closing quote itself.
- **`\'` in a double-quoted scalar reads as `'`,** as in libyaml and `go-yaml`, which
  kubectl and the API server use. `saphyr-parser` rejects it as an unknown escape, so
  when it reports an error at a double-quoted scalar, the builder rewrites each `\'` in
  that scalar to `\a` and parses again. Both escapes are two bytes, so no range moves.
  The builder then turns each BEL character that came from a rewritten escape back into
  `'`.

  **Why:** real manifests use it (the Kubernetes website has one), and kubectl accepts
  it, so reporting KL0001 there is a false positive.

  **Rejected:** forking or patching `saphyr-parser` (a maintenance burden for one
  escape); rewriting every `\'` in the file before parsing (outside double quotes it is
  two literal characters, so only the scalars the parser rejects are touched).
- **Tabs after a mapping's `:` separate the value,** as YAML 1.2 says and kubectl
  accepts (`name:<tab>web`). `saphyr-parser` rejects tabs alone before a value that
  starts with an ASCII letter, a digit, `-` or `_`, so when it reports that error, the
  builder rewrites the rejected run to spaces, one byte each, and parses again. To avoid
  one parse per run, it also parses a probe copy with every later candidate run rewritten
  and rewrites, in the real text, the candidates that lie before the probe's first error
  and outside every scalar and comment of the probe. A tab after a `:` inside a quoted or
  block scalar, a quoted key, a comment or a later document keeps its tab.

  **Why:** kubectl accepts these files, so KL0001 there is a false positive that also
  drops the document from every other rule. The text of a scalar is read by rules and
  echoed in messages, so it must stay as written.

  **Rejected:** rewriting every such run from the error on (it changes the text of
  scalars, keys and later documents); rewriting one run per parse (one reparse of the
  file per run, quadratic in a file that uses tabs throughout).
- **A document stops at the alias and nesting limits kubectl enforces.** The builder
  counts, per document, the nodes the decoder of `go-yaml` v2 (what Kubernetes 1.34 to
  1.37 use) visits, including every node of each alias expansion. Each finished node
  stores that count and its nesting, so an alias is checked before the anchored subtree is
  cloned. The document is over the budget under the same rule as `decode.go`: more than
  100 alias visits, more than 1,000 visits in all, and an alias share above a ratio that
  falls from 0.99 (up to 400,000 visits) to 0.10 (from 4,000,000). A `<<` key is not
  counted, as in `go-yaml`, and a merge adds no nesting level, as in the JSON. The
  document is also over the limit when its collections nest deeper than 10,000 levels,
  an alias expansion included, which is what the JSON decoder kubectl runs next refuses.
  In both cases the builder reports a KL0001 error at the
  alias or collection that crossed the limit, drops the document and resumes at the next
  document marker.

  **Why:** kubectl refuses these files ("excessive aliasing", "exceeded max depth"), so
  they cannot be applied anyway, and a few hundred bytes of nested aliases or a 100 KB
  line of `- ` otherwise exhaust memory or the stack and abort the whole run, every other
  file included. The same numbers as kubectl mean kubelint neither reports a manifest
  kubectl accepts nor accepts one it refuses.

  **Rejected:** a fixed node count unrelated to kubectl (it would refuse or accept
  different files than kubectl); a time limit (it depends on the machine and cannot stop
  the memory growth before it happens); counting the nodes of an expansion by walking it
  (that is the cost the limit exists to avoid).
- **Merge keys (`<<`) are expanded when their mapping closes,** with the precedence
  `go-yaml` v2 gives them, which is what `kubectl` and the API server use. Only a plain
  `<<`, one tagged `!!merge` or one tagged with the verbatim `!<tag:yaml.org,2002:merge>`
  is a merge key; a quoted `"<<"` or a local `!merge <<` is an ordinary key. The entries
  are written in document order and the last write of a key wins, so the builder replaces
  each `<<` entry in place by the entries of its source mapping (they keep the source's
  ranges, as an alias does). A key written before a `<<` that repeats it is overridden by
  the merge, and a key written after it overrides the merged one. In a sequence of
  sources the earlier source wins, so the builder lists the sources backwards. It then
  drops each entry that a later entry of another source overrides, so rules that read a
  key see the value the server gets, unless its own source repeats the key. Every entry
  has an `origin`: 0 when it is written in its mapping, otherwise a number shared by the
  entries of one merge source. KL0002 compares keys only within an origin, so it reports
  every key written twice (inside an inline source too, and around a merge) and never a
  key that only a merge repeats. A duplicate inside the value of an entry that a merge or
  a later key overrides is not reported, because the server never reads it. A mapping's
  range covers what it writes: the merged entries and a `<<` value written as an alias do
  not widen it. A `<<` value that is not a mapping or a sequence of mappings is refused
  by `kubectl`, so the builder reports a KL0001 error at the `<<` and drops the document.

  **Why:** every rule reads the tree as the server reads the object. Without the
  expansion, rules miss merged fields (a selector that `<<: *labels` fills) and report the
  `<<` key itself (KL1010) on files the server accepts.

  **Rejected:** the YAML 1.1 merge specification, where an explicit key always beats a
  merged one (`kubectl` lets a `<<` override a key written before it, so the tree would
  differ from what the server gets); keeping the `<<` entry and resolving it in each rule
  (every rule would repeat the precedence); expanding in `kubelint_k8s` views (KL0002 and
  the tree dump would still see `<<`); reporting duplicates on the expanded mapping alone
  (it cannot tell a key written twice from a key a merge repeats); keeping every
  overridden entry (rules would check values the server never reads).
- **Edits apply to the original text.** Autofix (M5) changes the file by byte range,
  never by re-serializing the tree, so formatting and comments survive.

### Kubernetes object views, not typed structs

kubelint does **not** deserialize manifests into typed Rust structs (`k8s-openapi`
style). Typed structs throw away positions and fail on the very mistakes a linter must
report. Instead `kubelint_k8s` offers thin views over the YAML tree:

```rust
for obj in kubelint_k8s::objects(&document) { ... }   // expands `kind: List`
obj.kind()            // Option<&Node>: the value node, with its range
obj.gvk()             // Option<GroupVersionKind>, `v1` is the core group
obj.name()            // metadata.name
obj.pod_spec()        // Some(PodSpec) for Pod, PodTemplate, Deployment, StatefulSet,
                      // DaemonSet, ReplicaSet, ReplicationController, Job, CronJob
for c in pod_spec.containers() { c.kind(); c.name(); c.security_context(); }
                      // containers + initContainers + ephemeralContainers
c.security_setting(SecurityField::RunAsUser)  // the container's, else the pod's
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
    pub explanation: &'static str,   // markdown from <rule_name>.md: why, examples, fix
}
```

- Rules are unit structs, one per file:
  `crates/kubelint_linter/src/rules/<category>/<rule_name>.rs`, with the explanation in
  `<rule_name>.md` next to it.
- The registry is a single static slice in `rules/mod.rs`. Adding a rule takes the rule
  file and its `.md`, one line in the registry, one fixture, its snapshot and a row in
  [`rules/README.md`](rules/README.md). No test code: `rules/tests.rs` snapshots the
  fixture of every rule in the registry. [`CONTRIBUTING.md`](CONTRIBUTING.md) walks
  through it.
- Rules report through the context (`cx.report(...)`), never by returning errors. A rule
  that cannot evaluate (missing fields) reports nothing; missing required fields are a
  different rule's job.
- Rules are pure: no I/O, no global state, no config reading beyond what the context
  passes in. That keeps them parallel-safe and trivially testable.
- Codes, names and categories follow the policy in
  [`rules/README.md`](rules/README.md#codes-and-names). Both are permanent once released.

### Rule documentation pages

A rule's page is `RuleMeta::documentation`, the same text `kubelint rule` prints. The
test `crates/xtask/tests/rule_docs.rs` regenerates every page in memory and fails when a
committed page differs, or when
`docs/rules/` holds a page for a code that no rule has.

**Why a test:** CI already runs the tests on every platform, so a stale page fails CI
with no workflow change, and the check also runs locally with `cargo test`.

**Rejected:** a separate CI job that runs the generator and then `git diff --exit-code`
(one more job to keep in `ci-success`, and it does not run locally); writing the pages
from a `build.rs` (a build that writes into the source tree).

### Diagnostics and exit codes

A `Diagnostic` has a rule code, severity, message, a primary label (range + optional
short text), optional secondary labels, optional help text and an optional `Fix` (a set
of `Edit { range, replacement }` with an `Applicability`, `Safe` or `Unsafe`, unused
until M5 autofix). Diagnostics are plain data in `kubelint_diagnostics`; rendering lives
in `kubelint_output`. The human renderer uses `annotate-snippets`, the renderer rustc
uses, for the familiar `error[KL2001]: ... --> file:line:col` layout.

Severities are `error`, `warning`, `info`. Each rule has a default, and users can
override it per rule. The run fails when any diagnostic is at or above `--fail-on`
(default `warning`).

**Why severities:** Kubernetes users expect errors and warnings, and some rules are
advisory by nature.

**Why warnings fail by default:** a linter that does not fail CI rarely gets acted on.

**Rejected:**

- No severities, every finding fails (Ruff, kube-linter).
- `miette` for rendering. More opinionated; `annotate-snippets` matches rustc exactly
  and is what Ruff uses.
- Warnings never fail by default (ESLint). It makes kubelint easy to ignore in CI.

| Exit code | Meaning |
| --- | --- |
| 0 | No diagnostics at or above the fail threshold. |
| 1 | Lint findings at or above the fail threshold. |
| 2 | kubelint itself failed: bad arguments, unreadable config, I/O error. |

Invalid YAML is a finding (KL0001), not an exit-code-2 error, because it is a problem in
the user's files rather than in kubelint.

### Configuration and suppressions

The file format, selector rules, flags, `exclude` patterns and suppressions are a public
contract, documented in [`user/configuration.md`](user/configuration.md) and
[`user/suppressions.md`](user/suppressions.md). Where the code
lives:

- `kubelint_config` finds and parses the file. Every value keeps its byte range, so
  errors are rendered as snippets of the file, with the same renderer as lint findings.
  It knows selectors only as strings.
- `kubelint_linter::RuleSelection::resolve` is the only place selectors are resolved,
  because that needs the rule registry. Configuration, flags and suppression comments
  all go through it. Its errors carry the list and index of the bad selector.
- `crates/kubelint/src/settings.rs` merges the file with the flags and remembers which
  list came from the file, so an error points back at the file only when the selector
  was written there.
- `kubelint_linter`'s `suppression` module reads the suppression comments and the
  `kubelint/ignore` annotations of each file. The engine drops the findings they cover,
  per-object and cross-object alike, before sorting.
- `crates/kubelint/src/exclude.rs` normalizes paths lexically (no symlink resolution)
  before matching. Excluded directories are still walked and their files filtered
  afterwards.

### File discovery

`crates/kubelint/src/discover.rs` searches each directory named on the command line with
the `ignore` crate: `.gitignore`, `.ignore`, `.git/info/exclude`, the global git excludes
(also outside a git repository) and hidden-file skipping apply, and so do the ignore files
in parent directories. If that search finds no YAML file and the parent rules would ignore
a YAML file placed directly in the directory (`gen/*`, `gen/**`, `/gen/*`, a root
`*.yaml`), it is searched again without the parent rules: the `.gitignore` and `.ignore`
files above it and the repository's `.git/info/exclude`. The ignore files inside the
directory, the global git excludes, hidden-file skipping, `[files] exclude` and Helm
chart skipping still apply. The test is a probe path in the directory matched against the
parent ignore files, so a directory whose only YAML sits under an ignored subdirectory
(`node_modules/` above `app/node_modules/pkg/x.yaml`) is not searched again. A directory
with at least one visible YAML file is searched once with every rule, so hidden files
next to visible ones stay skipped.

**Why:** a root `.gitignore` with `gen/*` hides `gen/a.yaml`, yet `kubelint check gen`
after `helm template > gen/all.yaml` is a request to lint that directory. Without the
exception it checked 0 files and exited 0, so a CI step could pass while checking
nothing. The probe keeps the other parent rules working: `kubelint check .` inside a
subdirectory still skips the `node_modules/` the repository root ignores, and a root
`*.tmp.yaml` still hides one file among others. The probe reads the parent files itself
because the `ignore` crate offers no way to ask whether its parent rules hide a path.

**Rejected:**

- Never applying parent ignore files. `kubelint check .` inside a subdirectory would lint
  a `node_modules/` that the root ignores.
- Searching again whenever the first search finds nothing. A directory whose only YAML is
  under an ignored subdirectory would then lint that subdirectory.
- An error when a named directory yields no file. It fails CI for a legitimately empty
  directory and still lints nothing.
- Keeping the behavior. A CI step can pass while checking nothing.
- Keeping `.git/info/exclude` in the second search. The `ignore` crate loads it together
  with the parent ignore files, so it cannot be separated from them.

### Output formats

| `--format` | For | Notes |
| --- | --- | --- |
| `human` (default) | terminals | rustc-style snippets via `annotate-snippets`, colors when stdout is a TTY, summary line with timing. |
| `json` | scripts | Stable, versioned schema. Breaking it is a breaking change. |
| `sarif` | GitHub code scanning and other security dashboards | SARIF 2.1.0, hand-written `serde` structs (no SARIF crate dependency). |
| `github` | GitHub Actions logs | `::error file=...,line=...,col=...::message` workflow commands. |

The exact contracts are in [`user/output-formats.md`](user/output-formats.md).
`kubelint_output` cannot see the rule registry, so the CLI passes rule metadata in (for
example `SarifRule` for the SARIF rule list).

### Schema validation

M4 checks every built-in object against the real Kubernetes API schema of one chosen
Kubernetes version. Four decisions shape it.

**Source: the OpenAPI v3 documents of `kubernetes/kubernetes`.** They live in
`api/openapi-spec/v3/` at each release tag: one JSON file per group version, plus
discovery and non-resource documents (about 64 files and 12 MB in all). They are what
the API server itself serves. A root definition names its kinds in
`x-kubernetes-group-version-kind`, fields mark `required`, a referenced type is a
single-element `allOf` around a `$ref` (with or without a `default`), and a field that
takes a number or a string (`IntOrString`, `Quantity`) is a `oneOf`. The importer skips
the files that define no kind. The published documents carry no `enum`: allowed values
appear only in descriptions ("One of Always, Never, IfNotPresent"), so KL1106 reads a
hand-written table, like the API version lifecycle table of KL1104.

**Why:** the schema comes from the project that defines the API, at an exact tag, so a
finding can always be traced to upstream.

**Rejected:** the single OpenAPI v2 `swagger.json` (one 4.4 MB file, with no `default`
and no `oneOf`, so `IntOrString` reads as a plain string); third-party JSON Schema
collections such as `kubernetes-json-schema` (derived, sometimes late, and their
conversion loses details); the `k8s-openapi` crate (typed structs, not schema data, see
[Kubernetes object views](#kubernetes-object-views-not-typed-structs)); parsing the
allowed values out of descriptions (prose, with no fixed shape).

**Version window: the latest 4 minor releases.** On 2026-11-15 that is 1.34 to 1.37,
and the default is the newest. Each version is imported from its latest patch tag at
import time, because patch releases do not change the API: a new minor release is a
PR that imports it, drops the oldest and moves the default. `--kubernetes-version` and
the `kubernetes-version` config key take `1.37` or `v1.37`; a patch number (`1.37.2`)
is accepted and ignored. A version outside the window is an error that lists the
supported ones.

**Why:** upstream supports the latest 3 minor releases, and managed clusters often run
one more. Four versions cover nearly every cluster still getting updates while keeping
the binary small.

**Rejected:** only the latest version (a team one release behind would get findings
for fields it cannot use yet); every version since 1.16 (a much larger binary for
clusters that no longer get security fixes).

**Storage: a compact form generated by `cargo xtask import-schemas`, committed and
embedded in the binary.** The importer reads the files of one version from a local
directory (a sparse checkout of the release tag, see
[Importing a version](#importing-a-version)). It merges them
into one table of definitions keyed by definition name, since every file repeats the
shared ones (`ObjectMeta`), and builds a lookup from each group version kind to its
definition (a definition such as `DeleteOptions` lists several). It keeps only what
validation uses (types, properties, required fields, `additionalProperties`, array
items, `oneOf` and `$ref`), drops descriptions, formats, defaults and the list, map and
patch extensions, and writes `crates/kubelint_schema/schemas/<version>.json` with
sorted keys, one definition per line. `schemas/sources.toml` records the tag and commit
of each version. `kubelint_schema` embeds the files with `include_str!` and parses only
the selected version, once, on first use. The format version is a constant in
`kubelint_schema`, written into each file's header, and a test fails when a committed
file has an older one. `xtask` depends on `kubelint_schema` for it.

**Why:** about 12 MB per version is too much to vendor in Git for 4 versions and every
future release, while the stripped form is a fraction of that and its diffs show what
changed in the API between versions. JSON reuses `serde_json`, which the workspace
already has. Parsing one version takes 3 to 5 ms against the
[performance budget](#performance-budget), and the four versions 1.34 to 1.37 (1.1 MB
of JSON) grow the release binary from 3.4 MB to 4.6 MB once the CLI calls
`Schema::embedded` (both measured on Linux x86-64). Without the upstream files in
Git, CI cannot regenerate the output, so the test checks the format version, which
catches an importer change that was not followed by a new import.

**Rejected:** vendoring the upstream files so CI can regenerate the output (repository
size); generated Rust tables (slow builds and huge source files); a binary format such
as `postcard` or compression such as `zstd` (new dependencies and unreadable diffs,
worth revisiting only if the binary grows too much); a SHA-256 of each upstream file in
`sources.toml` (a hash crate for a check that cannot run offline, while the tag and
commit already pin the input); downloading schemas at run time (kubelint never uses
the network).

#### Importing a version

```bash
git clone --depth 1 --branch v1.37.2 --filter=blob:none --sparse \
  https://github.com/kubernetes/kubernetes /tmp/kubernetes
git -C /tmp/kubernetes sparse-checkout set api/openapi-spec/v3
cargo xtask import-schemas /tmp/kubernetes
```

The importer reads the tag and commit from the checkout with `git`, takes the minor
version from the tag (`v1.37.2` gives `1.37`, a pre-release tag is refused), refuses a
checkout whose `api/openapi-spec/v3` differs from the tag, and replaces that version's
file and its `sources.toml` entry. A document that lists no
`x-kubernetes-group-version-kind` (`version_openapi.json`, `logs_openapi.json` and the
OpenID documents) is skipped without checking its keywords, and an input where no
document lists one is an error. A definition repeated across files must have the same
stripped form in each (only its `x-kubernetes-group-version-kind` list differs), and a
group version kind must belong to one definition. A keyword the importer does not know
(`enum`, `anyOf`, `x-kubernetes-preserve-unknown-fields`) stops the import with the
definition and the keyword, and so does a `$ref` or an `allOf` next to a keyword the
importer keeps (`kubelint_schema` reads only the reference) and an output that
`kubelint_schema` cannot load.

**Why:** a new upstream keyword can change what a field accepts, so dropping it unseen
would make the validator wrong without a trace. Reading the version from a clean
checkout of the tag means a file is never labeled with another version than the one it
came from.

**Rejected:** dropping every unknown keyword (silent drift); passing the version and
commit as arguments (they can disagree with the checkout).

`kubelint_schema` lists the embedded versions once, in the `embed!` call of
`src/embedded.rs`. Because `xtask` builds `kubelint_schema`, a listed file must exist
before the importer can run: import a new version first, then add it to `embed!`, and
remove a dropped version from `embed!` before deleting its file and its `sources.toml`
entry.

**Why:** the list stays visible in code and the crate needs no build script.

**Rejected:** a `build.rs` that generates the list from `schemas/` or `sources.toml`
(one more build step, for a list that changes once per Kubernetes release).

**Validator: our own, span-aware, in `kubelint_schema`.** It walks the `kubelint_yaml`
tree of an object against the definition of its group version kind and reports each
problem with the `TextRange` of the offending key or value. Its rules:

- `$ref` and the single-element `allOf` around it are followed. A `oneOf` accepts a
  value that matches any branch.
- An object with `properties` and no `additionalProperties` is closed: another key is
  an unknown field. An object with `additionalProperties` is a map of that schema. An
  object with neither (`RawExtension`, the `JSON` of a CRD) accepts anything.
- An explicit null (`key:`, `~`) is accepted for every field that is not required, as
  the API server treats it as unset. A required field that is null or missing is a
  missing required field.
- Scalars are typed with `Node::resolve_kubernetes`, the YAML 1.1 rules `kubectl` uses,
  so the validator agrees with what would be sent.
- An object whose group version kind is not in the selected version is skipped without
  a finding. That covers custom resources, removed versions (KL1104 reports those) and
  group versions added after the selected version.

**Why:** the findings need exact positions and plain words, which a generic validator
cannot give.

**Rejected:** a JSON Schema crate such as `jsonschema` (it validates a
`serde_json::Value`, which has no positions, its messages are generic, and Kubernetes
OpenAPI is not plain JSON Schema).

The schema rules KL1101 to KL1103 are rules like any other: the validator returns typed
problems (unknown field, wrong type, missing required field) and each rule turns its
kind into a diagnostic. KL1102 never reports a value that KL1009 or KL1010 checks,
whether or not those rules are enabled, so one rule's output never depends on another
rule's configuration. User-provided CRD schemas come after M4, through the same
validator.

### Self update

`kubelint self update` runs the latest release's installer again, the way a user
installs:

1. Read the receipt the installer wrote
   ([what the installer writes](user/getting-started.md#what-the-installer-writes)).
   Stop unless the running binary is in the folder it names.
2. Read the latest version from `dist-manifest.json`, which every release carries, at
   `releases/latest/download/`.
3. When that version is newer, run its installer with `KUBELINT_INSTALL_DIR` set to the
   receipt's folder and `KUBELINT_NO_MODIFY_PATH=1`. The installer replaces the binary
   and rewrites the receipt, and leaves the shell profiles and the `PATH` as the first
   install set them up, even if the user changed them since.

Downloads use `curl`, or `wget` when `curl` is missing (Windows 10 and 11 include `curl`).
On Windows the installer runs from the downloaded file with `powershell -File`. Windows
cannot replace a running exe but can rename it, so `kubelint.exe` is renamed to
`kubelint.previous.exe` and a copy takes its place until the installer overwrites it. A
failed install renames it back, and a hidden PowerShell deletes it after kubelint exits.

**Why:** no new dependency. The binary grows by about 160 KB (the JSON parser) and the
build time does not change.

**Rejected:**

- The `axoupdater` crate, which does the same steps in process. It adds about 110 crates
  (an HTTP and TLS stack and `tokio`), more than doubles the binary (3.4 MB to 7.6 MB on
  Windows, to about 8.5 MB on Linux) and the release build time. Version 0.10.3 also
  fails on Windows when the temp folder path has a space, leaves no `kubelint.exe` when
  the update is interrupted, and adds the `PATH` setup back when the user removed it.
- dist's `install-updater`, which ships a separate `kubelint-update` binary instead of a
  `kubelint self` command.
- Running the Windows installer the way the install command does,
  `powershell -c "irm <url> | iex"`. Microsoft Defender's machine learning can flag that
  command shape as `Trojan:Win32/Commando`, and blocked it on 2026-11-17.

## Testing

How tests are written is in [`conventions.md`](conventions.md#tests).

- **Unit tests** live in their own files: `crates/<crate>/tests/`, or `<module>/tests.rs`
  when they need private items ([conventions](conventions.md#tests)).
- **Rule tests** are snapshot tests: a fixture YAML file per rule in
  `crates/kubelint_linter/resources/test/fixtures/<category>/<CODE>.yaml` containing both
  violating and passing cases, linted with only that rule enabled, rendered as text and
  compared with `insta`. One loop in `crates/kubelint_linter/src/rules/tests.rs` covers
  every rule of the registry, with one snapshot per rule code.
- **CLI tests** in `crates/kubelint/tests/` run the real binary (`assert_cmd`) and
  snapshot stdout, stderr and the exit code.
- Snapshots are reviewed like code. Update with `INSTA_UPDATE=always cargo test`, then
  read the diff.

### End to end checks

A rule is right only if Kubernetes agrees with it. `.github/scripts/e2e-apiserver.sh
[VERSION]` starts a real API server (etcd and `kube-apiserver`, any version, downloaded
when missing) and the Docker daemon, and prints the `KUBECONFIG` and `PATH` to use. One
server runs per checkout: a second call for the same version reuses it, a call for
another version replaces it. Each server takes the first free ports from 2379 (etcd)
and 6443 (API) and its own token, so the servers of several checkouts run side by side.
Starts on one machine run one at a time, so two never pick the same ports. Then:

- `kubectl apply --dry-run=server -f <file>` gives the API server's verdict on an
  object: decoding, validation and admission, including Pod Security once the namespace
  has a `pod-security.kubernetes.io/enforce` label. A dry run creates nothing, so create
  the namespaces a file uses first with `kubectl create namespace`.
- `docker run` with the flags that match a pod's `securityContext` shows whether an
  example container starts: `--user`, `--read-only`, `--cap-add`, `--cap-drop`,
  `--privileged`, `--security-opt no-new-privileges` for
  `allowPrivilegeEscalation: false`, and `--tmpfs <path>:exec` for an `emptyDir`. Docker
  applies its own seccomp profile, so add `--security-opt seccomp=unconfined` unless the
  pod sets `seccompProfile: RuntimeDefault`.

The server runs with `--allow-privileged=true`, as real clusters do, and without the
`ServiceAccount` admission plugin, because no controller creates service accounts. It
reports only the first decoding error of an object, so one bad value can hide another.

**Why:** the routine's VM has Docker but withholds the permission a kubelet needs to
start pods (lowering a process's OOM score), so a full cluster cannot run there. The
claims rules make are about what the API server accepts, which the bare server answers
exactly and starts in about 4 seconds for any version of the window, or about what a
container does with a given `securityContext`, which `docker run` shows. Kubelet
behavior (probes, the `runAsNonRoot` check, evictions) is not covered and is checked
against the kubelet source instead.

**Rejected:** `kind` or `k3s` in the VM (pod sandboxes fail with "failed to update
/proc/self/oom_score_adj: Permission denied", tested on 2026-11-15); a managed cluster
(an account and a secret, and network calls from CI to a shared cluster).

#### The fixture check

`cargo xtask e2e [<CODE>...]` checks rule fixtures against the API server that
`e2e-apiserver.sh` started, at the server's own version. It labels namespaces, so it
refuses to run with any other `KUBECONFIG`. The CI job `e2e` runs it through
`.github/scripts/e2e.sh` on every version kubelint has a schema for, and any
disagreement fails CI.

- It splits a fixture at its `---` lines into cases, each with the comment above it, and
  lints the whole file with only that rule, at the server's version. A case is reported
  when a finding starts inside it.
- It creates every namespace the cases use and sends each case alone as a dry run, with
  the check's `kubectl` command: `apply --dry-run=server` by default, and
  `create --dry-run=server` for KL1003 and KL1008, because `kubectl apply` refuses an
  object without a `name`, including one with only a `generateName`, before it sends it. A
  namespace whose name the server rejects is not created, so a case in it gets
  `NotFound`, which is the answer a cluster gives for it.
- Each checked rule (`CHECKS` in `crates/xtask/src/e2e.rs`) has the answers that mean
  its field is wrong (its needles) and the stage where the server gives them. A needle
  matches within one line, with list indices written `[]` and `*` for any text, so it
  names the field and not only the error (`containers[].name: Duplicate value`, not a
  duplicate volume name). A case agrees when the rule reports it exactly when the
  server gives one of them.

  | Stage | Rules | The server judged the case when |
  | --- | --- | --- |
  | Decoding | KL1007, KL1009 | it accepted the case, gave a needle, or reported a validation error (decoding reports one error, and validation runs only after it passed) |
  | Validation | KL1003, KL1005, KL1006, KL1008, KL1010 | it accepted the case, gave a needle, or reported validation errors other than `invalid label selector` (validation reports every invalid field, but an invalid workload selector skips the checks of the pod template) |
  | Warning | KL1011 | it accepted the case (a validation error comes without warnings) |
  | Pod Security | KL2001 to KL2007 | it accepted the case or named a Pod Security check, in namespaces that enforce the rule's level (`baseline` or `restricted`) |

  A Pod Security needle is a check name (`privileged`, `runAsNonRoot != true`...),
  given in a rejection for a Pod and in a warning for a workload. A case the server
  rejects before the rule's stage is a disagreement too, because it proves nothing.
- A case the server cannot judge carries a marker comment with the reason:
  `# e2e-skip: <reason>` on every version, `# e2e-skip 1.34: <reason>` on one (a marker
  with no reason or an unsupported version stops the run). Typical
  reasons are custom resources, schema problems, invalid names and fields that are
  rejected first, cases where `kubectl apply` and `kubectl create` disagree, and
  deliberate differences between the rule and the server. The
  run prints every skipped case with its reason.

**Why:** the fixtures already hold every case a rule decides, each with its intended
verdict, so checking them needs no second set of examples. Markers live in the fixture
next to the case they explain, so a new case is checked by default and leaving it out
takes a written reason. Judging by the rule's own answer, not by acceptance, keeps cases
testable that the server rejects for something else too: a `restricted` namespace flags
several checks at once, and most fixture workloads have no containers. `kubectl apply`
stays the default because it is what users run, and `kubectl create` is used only where
`apply` sends nothing.

**Rejected:** a separate set of end to end examples (it drifts from the fixtures); a
list of skipped cases outside the fixtures (line numbers move with every edit); running
only the newest version (Pod Security changed between 1.34 and 1.35, and KL1008 knows
version dependent names); `kubectl create` for every rule (it keeps the null labels that
`apply` drops, so KL1005 cases would change verdict); skipping the KL1003 and KL1008 cases
without a `name` (every case KL1003 reports, and every KL1008 case with a `generateName`
and no `name`).

## Performance budget

- Lint 10,000 objects in under 1 second on a laptop, release build.
- No regex compilation per object (compile once, `std::sync::LazyLock`).
- No per-object allocation of the whole document. Views borrow from the tree.

### Measuring it

```bash
cargo xtask gen-corpus
cargo build --release
time ./target/release/kubelint check target/bench-corpus --format json > /dev/null
```

`cargo xtask gen-corpus [--objects <N>] [<DIR>]` writes a deterministic corpus of
exactly `N` objects (default 10,000) into `DIR` (default `target/bench-corpus`). Each
group of five files is one application in its own namespace (its DaemonSet runs in
`kube-system`), built from the templates
in `crates/xtask/resources/corpus/`: Deployments, a StatefulSet, a DaemonSet, a Job and
a CronJob with Services, ConfigMaps, a Secret, RBAC, an Ingress, a network policy and
an autoscaler. Some parts are clean and some have typical security and reliability
findings. The corpus has no syntax or correctness findings, which
`crates/xtask/tests/corpus.rs` checks on 160 objects, along with the exact object count.
Files hold at most four objects, so the corpus does not measure one large file.

Measured on 2026-11-16 (release build of `78b2116`, 4 cores, Linux cloud container,
five runs each, 10,000 objects in 3,125 files, 11,250 findings with the default rules):

| Output | Wall time |
| --- | --- |
| `--format json` | 0.14 to 0.15 s |
| `--format json`, one thread (`RAYON_NUM_THREADS=1`) | 0.29 to 0.34 s |
| `--format human` | 0.20 to 0.23 s |
| `--format json`, the same 10,000 objects in one file | 0.26 s |
| `--format human`, the same 10,000 objects in one file | 60 s |
| `--format json`, 20,000 objects with half excluded by `[files] exclude` (2026-11-12) | 0.19 s |

The corpus holds the budget with room to spare. One large file, which is what
`helm template | kubelint check -` gives, breaks it in human output only: each finding
rendered the whole file. The 2026-11-12 measurement counted 19,375 findings because
KL2002 and KL2003 were still on by default. Reading files one by one and walking
excluded directories are not measurable at this size.

**Why a generated corpus:** it is reproducible on any machine with no download, scales
to any size, and mixes the kinds and findings of a real repository.

**Why no timing check in CI:** shared CI runners vary by more than the budget's margin,
so a time limit would fail at random. The budget is measured by hand when the lint
path changes (for example after M4 schema validation).

**Rejected:** a public manifest collection as the corpus (needs the network and a
license check, and its size is fixed); a `criterion` benchmark (a new dependency, and it
measures library calls, not the binary users run); a CI time limit (flaky, see above).

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
| `serde`, `serde_json`, `toml` | config parsing, JSON/SARIF output and schema files |
| `anyhow` | error handling in the CLI binary only |
| `insta` | snapshot tests (dev-dependency) |
| `assert_cmd` | CLI integration tests (dev-dependency) |
