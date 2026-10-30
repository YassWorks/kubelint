# Glossary

Short, plain-language definitions of the concepts used in kubelint. Each entry says what
the thing is, why it matters here, and where it shows up.

Agents: add an entry the first time a PR relies on a concept that is not here yet. Keep
entries to a few sentences, keep each section alphabetical, and link to the code or doc
where the concept appears.

## Kubernetes basics

**Annotation.** A key/value note attached to an object under `metadata.annotations`.
Unlike labels, annotations are not used for selection, so tools use them to store extra
information. kubelint plans to read `kubelint/ignore` annotations to silence rules on one
object (owner inbox OI-002).

**API server.** The front door of a Kubernetes cluster. Every manifest you apply is sent
to it, and it rejects objects that are invalid. Many kubelint correctness rules
(`KL1xxx`) catch, before you apply, exactly what the API server would reject.

**`apiVersion` and `kind`.** The first two lines of almost every manifest. `kind` says what
the object is (`Deployment`, `Service`...). `apiVersion` says which version of that type's
definition to use, written `group/version` (`apps/v1`) or just `version` for the core
group (`v1`). kubelint uses these two fields to recognize Kubernetes YAML (see
`architecture.md`).

**Container.** One running program, started from an image. A pod holds one or more.
Most security and reliability rules check each container's settings.

**CRD (CustomResourceDefinition).** A way to teach a cluster new kinds of objects, such as
a database operator's `PostgresCluster`. kubelint does not know their fields, so schema
rules skip them.

**Deployment, StatefulSet, DaemonSet, Job, CronJob.** "Workload" kinds. Instead of
creating pods directly, you describe a pod template and the workload creates and manages
pods from it: Deployments for stateless apps, StatefulSets for apps with stable identity
and storage, DaemonSets for one pod per node, Jobs for run-to-completion tasks, CronJobs
for scheduled Jobs. kubelint finds the pod template inside each of them (`pod_spec()` in
`kubelint_k8s`) so rules work on all of them at once.

**Labels and selectors.** Labels are key/value tags in `metadata.labels`. A selector picks
objects by their labels: a Deployment finds its pods, a Service finds where to send
traffic. If a Deployment's selector does not match its own pod template's labels, the API
server rejects it. That is rule KL1005.

**Manifest.** A YAML (or JSON) file describing one or more Kubernetes objects. One file can
hold several objects separated by `---` lines.

**Metadata.** The `metadata` block every object has: `name` (required, unique per kind and
namespace), `namespace`, `labels` and `annotations`.

**Namespace.** A named group of objects inside a cluster, like a folder. Two objects can
share a name if they are in different namespaces. This matters for KL1004
(duplicate objects).

**Object.** One thing in the cluster (a Deployment, a Service, a ConfigMap), described by
one YAML document in a manifest.

**Pod.** The smallest thing Kubernetes runs: one or more containers that share a network
address and storage. You rarely write pods directly; workloads create them.

**Probes.** Health checks the cluster runs against a container. A readiness probe
decides whether the container should receive traffic; a liveness probe decides whether
to restart it. See KL3004.

**Resource requests and limits.** Under `resources`, a container declares what it needs
(`requests`, used to pick a node) and the most it may use (`limits`, enforced). Values are
"quantities" like `500m` (half a CPU) or `256Mi` (256 mebibytes). Missing limits let one
container starve its neighbors (KL3001); mistyped quantities like `256mb` are rejected
(KL1007).

**`securityContext`.** Security settings for a pod or a container: run as a non-root
user, forbid privilege escalation, drop Linux capabilities, read-only filesystem. Most
`KL2xxx` security rules inspect it. Container-level settings override pod-level ones.

**Spec.** The `spec` block: the desired state of the object. Its shape depends on the
`kind`.

## Rust basics

**Cargo, `Cargo.toml`, `Cargo.lock`.** Cargo is Rust's build tool and package manager.
`Cargo.toml` describes a crate and its dependencies; `Cargo.lock` records the exact
versions used so every build is identical. kubelint commits `Cargo.lock`, and CI builds
with `--locked` to enforce it.

**Crate.** A Rust package: either a library or a binary (a program). kubelint is split
into several crates, one per layer (see `architecture.md`).

**Doc comments and doctests.** Comments starting with `///` (for an item) or `//!` (for a
whole crate or module) become the generated documentation. Code examples inside them are
compiled and run as tests, so examples cannot go stale.

**Edition.** A Rust language "version" chosen per crate (2015, 2018, 2021, 2024). kubelint
uses edition 2024, the newest.

**Enum.** A type that is exactly one of several variants, each able to carry data. For
example a YAML node is a mapping, a sequence or a scalar. Rust forces code to handle
every variant, which prevents a whole class of bugs.

**`Option` and `Result`.** Rust has no `null` and no exceptions. A value that may be absent
is an `Option<T>` (`Some(value)` or `None`); an operation that may fail returns a
`Result<T, E>` (`Ok(value)` or `Err(error)`). The `?` operator passes an error up to the
caller.

**Ownership and borrowing.** Every value in Rust has one owner, and the value is freed when
the owner goes away. Other code can borrow it temporarily with a reference (`&T`, or
`&mut T` to modify it). The compiler checks these rules, which is how Rust is memory-safe
without a garbage collector.

**`pub` and visibility.** Items are private to their module by default. `pub` makes them
usable from other crates; `pub(crate)` from anywhere in the same crate only. kubelint
keeps things private unless another crate needs them.

**Struct.** A type that groups named fields, like `Diagnostic { code, severity, message,
... }`.

**Trait.** A set of methods a type promises to provide, similar to an interface. Every
kubelint rule is a type that implements the `Rule` trait, which is how the engine can run
any rule without knowing its details.

**`unwrap()` and panics.** `unwrap()` takes the value out of an `Option` or `Result` and
crashes the program (a "panic") if there is none. kubelint forbids it on anything that
comes from user input, because a linter must never crash on a bad file.

**Workspace.** Several crates developed together in one repository, sharing one
`Cargo.lock`, one `target/` build folder and common settings. kubelint's root `Cargo.toml`
defines the workspace.

## Tools and project terms

**ADR (Architecture Decision Record).** A short document recording one design decision
and why it was made. They live in `docs/decisions/`.

**CI (Continuous Integration).** Automated checks that run on every pull request: here,
formatting, lints, tests on three operating systems, and docs. The PR can only merge when
the `ci-success` check is green.

**Clippy.** Rust's official linter. kubelint runs it in "pedantic" mode with every warning
treated as an error.

**Conventional Commits.** A format for commit messages and PR titles: `type(scope):
summary`, like `feat(rules): add KL2001 privileged-container`. It makes history easy to
scan and lets a changelog be generated automatically.

**Diagnostic.** One finding reported by kubelint: rule code, severity, message, the exact
location in the file, and help on how to fix it.

**Linter.** A tool that reads source files without running them and reports likely
mistakes. kubelint is a linter for Kubernetes YAML.

**LSP (Language Server Protocol).** The standard way editors like VS Code talk to
language tools. A kubelint language server (planned for v0.3) will show findings while
you type.

**rustfmt.** Rust's official code formatter. CI fails if code is not formatted exactly the
way it would format it.

**SARIF.** A standard JSON format for static analysis results. GitHub reads it to show
findings in the "Code scanning" tab and inline on pull requests.

**Snapshot test.** A test that saves the output of some code to a file (a "snapshot") and
fails if the output ever changes. kubelint uses the `insta` crate for this. When output
changes on purpose, you review the new snapshot and accept it.

**Span / text range.** A start and end position in a file, measured in bytes. Every
diagnostic carries one, which is how kubelint can point at the exact characters that are
wrong.

**Squash merge.** Merging a pull request as a single commit on `main`, whatever number of
commits the PR had. Each roadmap task becomes exactly one commit in history.

**UTF-8 and UTF-16 columns.** Text is stored as UTF-8 bytes, where one character takes 1
to 4 bytes. The language server protocol counts columns in UTF-16 code units instead
(characters outside the basic plane take 2). `LineIndex` in `kubelint_source` reports
both so the terminal and, later, the editor point at the same character.

**YAML.** The text format Kubernetes manifests are written in. Indentation defines
structure, and unquoted values are guessed: `8080` becomes a number and `yes` becomes a
boolean, which is behind rule KL1009.
