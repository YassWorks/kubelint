# Contributing to kubelint

Thanks for helping. kubelint is early, and the fastest way to get oriented is:

1. [`vision.md`](vision.md): what we are building and what we are not.
2. [`architecture.md`](architecture.md): how the code is organized.
3. [`conventions.md`](conventions.md): how code, tests, docs and PRs are written.
4. [`roadmap.md`](roadmap.md): what is being worked on now.

## Setup

You need [rustup](https://rustup.rs). The toolchain version is pinned in
`rust-toolchain.toml` and installs automatically on first build.

```bash
git clone https://github.com/YassWorks/kubelint
```

```bash
cd kubelint && cargo build
```

## The checks CI runs

Run these before opening a PR. CI runs exactly the same commands.

```bash
cargo fmt --all
```

```bash
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

```bash
cargo test --workspace --all-features --locked
```

```bash
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked
```

To accept changed snapshots, run the tests with `INSTA_UPDATE=always`, then review the
`.snap` diffs with `git diff` before committing:

```bash
INSTA_UPDATE=always cargo test --workspace
```

## Adding a rule

KL1001 `missing-api-version`
(`crates/kubelint_linter/src/rules/correctness/missing_api_version.rs`) is a complete,
small example to copy.

1. **Find or allocate the code.** Look the rule up in [`rules/README.md`](rules/README.md).
   If it is not there, add a row with the next free code in the right category and
   status `planned`.
2. **Write the fixture first:**
   `crates/kubelint_linter/resources/test/fixtures/<category>/<CODE>.yaml`, where
   `<category>` is `syntax`, `correctness`, `security` or `reliability`. Put each case in
   its own YAML document (`---`) with a comment above it saying `Reported: ...` or
   `Not reported: ...` and why. Include cases that must be reported and cases that must
   not.
3. **Write the rule** in `crates/kubelint_linter/src/rules/<category>/<rule_name>.rs`
   (the name in snake case): a unit struct implementing `Rule`, with a `static META:
   RuleMeta` (code, name, category, default severity, default enabled, summary,
   explanation). The explanation is Markdown in `<rule_name>.md` next to the rule
   (what it does, why it matters, an example and its fix), loaded with
   `explanation: include_str!("<rule_name>.md")`. `check_object` gets an
   `ObjectContext`: read the object with `cx.object()`, build a finding with
   `cx.diagnostic(range, message)` plus `.with_help(...)`, and hand it to
   `cx.report(...)`. The context fills in the code and the configured severity. Use the
   `kubelint_k8s` helpers (for example `pod_spec().containers()`) instead of walking
   YAML by hand.
4. **Register it** in two places: `mod <rule_name>;` and `pub use` of the struct in the
   category module (`src/rules/<category>/mod.rs`), and one line in `ALL` in
   `src/rules/mod.rs`, which is kept in code order (a test checks it).
5. **Review the snapshot.** There is no test code to write: `src/rules/tests.rs` lints
   the fixture of every rule in `ALL`. Run
   `INSTA_UPDATE=always cargo test -p kubelint_linter` and read the new
   `src/rules/snapshots/kubelint_linter__rules__tests__<CODE>.snap`: every reported
   position and message should be exactly what you expect, and every `Not reported`
   case should be absent.
6. **Update the catalog row** status to `implemented` and tick the roadmap item.

A good diagnostic message names the object and the field and says what is wrong; the
help says how to fix it. This is how `kubelint check` prints one:

```text
error[KL1001]: `Deployment` object has no `apiVersion`
 --> k8s/web.yaml:1:1
  |
1 | kind: Deployment
  | ^^^^^^^^^^^^^^^^
  |
  = help: add `apiVersion: apps/v1`
  = note: rule `missing-api-version`
```

## Pull requests

- Fill in every section of the PR template.
- One concern per PR, ideally under 400 changed lines excluding snapshots and fixtures.
- PR titles follow Conventional Commits: `feat(rules): add KL2001 privileged-container`.

## Decisions

Decisions are written into the doc they are about, each with a **Why** and a
**Rejected** line ([`conventions.md`](conventions.md#documentation)). If your change
contradicts one, update it in the same PR and explain why in the PR description.
