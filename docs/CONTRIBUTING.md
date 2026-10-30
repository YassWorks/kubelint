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

> This walkthrough describes the intended pattern. It becomes exact once roadmap task
> M1-10 lands the first rule; M1-15 checks this section against the real code.

1. **Find or allocate the code.** Look the rule up in [`rules/README.md`](rules/README.md).
   If it is not there, add a row with the next free code in the right category and
   status `planned`.
2. **Write the fixture first:**
   `crates/kubelint_linter/resources/test/fixtures/<category>/<CODE>.yaml`. Put each case
   in its own YAML document (`---`) with a comment above it. Include cases that must be
   reported and cases that must not.
3. **Write the rule** in `crates/kubelint_linter/src/rules/<category>/<rule_name>.rs`:
   a unit struct implementing `Rule`, with a `RuleMeta` (code, name, category, default
   severity, default enabled, summary, explanation). Use the `kubelint_k8s` helpers
   (for example `pod_spec().containers()`) instead of walking YAML by hand.
4. **Register it** with one line in `crates/kubelint_linter/src/rules/mod.rs`.
5. **Add the snapshot test** in the rule's `tests` module, run
   `INSTA_UPDATE=always cargo test -p kubelint_linter`, and read the snapshot: every
   reported position and message should be exactly what you expect.
6. **Update the catalog row** status to `implemented` and tick the roadmap task.

A good diagnostic message names the object and the field, says what is wrong, and the
help text says how to fix it:

```text
error[KL2001]: container "api" runs in privileged mode (privileged-container)
  --> deploy.yaml:24:23
   |
24 |             privileged: true
   |                         ^^^^ this gives the container full access to the node
   |
   = help: remove `privileged: true`, or grant only the capabilities it needs
```

## Pull requests

- Fill in every section of the PR template.
- One concern per PR, ideally under 400 changed lines excluding snapshots and fixtures.
- PR titles follow Conventional Commits: `feat(rules): add KL2001 privileged-container`.

## Decisions

Significant design decisions are recorded as ADRs in [`decisions/`](decisions/). If your
change contradicts one, write a new ADR that supersedes it, in the same PR.
