# Conventions

How code, tests, docs, commits and pull requests are written in this repository. These
apply to every contributor, human or agent. When a convention is missing, follow what the
surrounding code does, then add the convention here.

## Rust code

- **Formatting:** `rustfmt` defaults. Never hand-format against it.
- **Lints:** the workspace lint set (ADR 0007) with warnings denied in CI. To silence a
  lint, use `#[expect(clippy::name, reason = "why this is fine here")]` on the smallest
  possible item. Never `#[allow]`, never crate-wide.
- **Naming:** types `UpperCamelCase`, functions and modules `snake_case`, rule structs
  named after the rule (`PrivilegedContainer` for `privileged-container`), rule files
  `snake_case` of the rule name (`privileged_container.rs`).
- **Errors:** library crates define error enums with `thiserror`. Only the `kubelint`
  binary uses `anyhow`. Errors that reach the user say what happened and what to do.
- **No panics on user input.** No `unwrap()`/`expect()` on anything derived from files,
  config or arguments. `expect("reason")` is acceptable only for true invariants, and the
  message states the invariant.
- **Visibility:** keep things private by default. `pub` is a promise to other crates;
  `pub(crate)` is the default for helpers.
- **Docs:** every public item has a doc comment (enforced by `missing_docs`). Explain
  *why* and *when to use*, not a restatement of the signature. Public functions with
  non-obvious behavior get a doc example (it runs as a test).
- **Comments:** comment the non-obvious: Kubernetes quirks, YAML edge cases, why a rule
  skips something. Link the Kubernetes source or docs when the behavior comes from there.
- **Performance:** compile regexes once (`std::sync::LazyLock`), avoid cloning YAML
  subtrees, borrow from the tree.
- **No `unsafe`** (forbidden at the workspace level).

## Kubernetes correctness

- A rule's behavior must match what Kubernetes actually does. When in doubt, read the
  Kubernetes source or official docs and cite it in a comment.
- A rule that cannot decide (fields missing, unknown kind, templated values) stays
  silent. False positives are worse than false negatives.
- Rules that look at containers use `pod_spec().containers()` from `kubelint_k8s` so every
  workload kind and every container type is covered consistently.

## Tests

See ADR 0006. In short:

- Every behavior change comes with tests. Every bug fix starts with a failing test.
- Rule fixtures contain both violating and passing cases, each preceded by a comment
  explaining the case.
- Fixtures include tricky inputs: non-ASCII text, flow-style YAML, multiple documents,
  every relevant workload kind.
- Update snapshots with `INSTA_UPDATE=always cargo test --workspace`, then read every
  changed `.snap` file before committing. Never commit `*.snap.new` files.

## Dependencies

- Pre-approved crates are listed in the root `Cargo.toml` (`[workspace.dependencies]`) and
  in `docs/architecture.md`. Use them through `{ workspace = true }`.
- A new dependency needs a justification line in the PR description and a row in the
  architecture dependency table. Prefer crates that are widely used, actively
  maintained, and licensed MIT, Apache-2.0, BSD, ISC, Zlib or Unicode.
- Do not add a dependency for something that takes under about 50 lines to write well.
- Never add a dependency with a copyleft license (GPL, AGPL, LGPL, MPL) without owner
  approval.

## Documentation

- **Where docs live:** user and contributor docs in `docs/`, rule catalog in
  `docs/rules/`, decisions in `docs/decisions/`, agent memory in `docs/agent/`. The root keeps only `README.md`, `CLAUDE.md` and
  `LICENSE`.
- **Same PR rule:** a PR that changes behavior updates the docs that describe it.
- **Prose style:**
  - Plain, direct sentences. Write for someone new to Rust and Kubernetes.
  - Do not use em dashes. End the sentence with a period and start a new one, or use a
    colon or parentheses.
  - Use `code formatting` for commands, file paths, field names, rule codes.
  - Prefer short sections with headings and tables over long paragraphs.
- **Links** are relative so they work on GitHub and in a checkout.

## Commits and branches

- **Conventional Commits** for commit messages and PR titles:
  `type(scope): summary` in the imperative mood, under 72 characters.
  - Types: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`, `ci`, `build`.
  - Scope: the crate without the prefix (`yaml`, `linter`, `cli`...), `rules` for rule
    work, `agent` for `docs/agent/` updates, or omitted.
  - Example: `feat(rules): add KL2001 privileged-container`.
- PRs are squash-merged, so the PR title becomes the commit on `main`. The changelog
  will be generated from these titles (M3-09).
- Branches: `claude/<topic>` for agent work.

## Pull requests

- Use `.github/pull_request_template.md`. Every required section is filled.
- **One concern per PR.** One roadmap task, or one fix, or one docs topic.
- **Size:** aim for under about 400 changed lines, excluding snapshots and fixtures. If
  larger, split it unless the change is mechanical.
- No drive-by changes. Unrelated improvements go to the roadmap or a follow-up PR.
- The **Concepts** section teaches the owner 2 to 4 ideas the PR relies on, in plain
  language.
