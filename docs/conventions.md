# Conventions

How code, tests, docs, commits and pull requests are written in this repository. These
apply to every contributor, human or agent. When a convention is missing, follow what the
surrounding code does, then add the convention here.

## Rust code

- **Formatting:** `rustfmt` defaults. Never hand-format against it.
- **Lints:** the workspace lint set (see [Toolchain and lints](#toolchain-and-lints)) with
  warnings denied in CI. To silence a lint, use
  `#[expect(clippy::name, reason = "why this is fine here")]` on the smallest possible
  item, so the exception disappears when it is no longer needed. Never `#[allow]`, never
  crate-wide.
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
- **Doc comments** only when they add what the name and signature do not: an invariant,
  a unit, a Kubernetes or YAML behavior, when to use the item. No doc comment that
  restates the name (`/// The key node.` on `key: Node`). Each crate keeps its `//!`
  crate doc. Public functions with non-obvious behavior get a doc example.
- **Comments** explain what the code cannot show: Kubernetes behavior, a YAML spec
  quirk, a `saphyr-parser` detail, why a rule skips something. Link the Kubernetes source
  when the behavior comes from there. Default to one line; 3 to 4 lines only for such an
  external contract. Never comment self-explanatory code.
- **Comments describe the code as it is,** never how it got there. No "deliberately",
  "on purpose", "an earlier version", "X was tried", "X is worse than Y". State the
  behavior or the constraint, then stop.
- **Multi-line text lives in its own file,** never in a Rust string: rule explanations
  in `<rule_name>.md` next to the rule (`include_str!`), test inputs as fixture files,
  expected output as `insta` file snapshots. A one-line message wrapped with `\` is fine.
- **Performance:** compile regexes once (`std::sync::LazyLock`), avoid cloning YAML
  subtrees, borrow from the tree.
- **No `unsafe`** (forbidden at the workspace level).

**Why:** a source file should read as code. Comments that restate names and long text
blocks bury the logic.

**Rejected:** the `missing_docs` lint (it forces a restating comment on every public
item); rule explanations as raw strings (40 lines of Markdown inside a Rust file).

## Style

Write the shortest clear version. If 200 lines could be 50, rewrite them.

- **No ceremony.** One expression where it reads well, not three statements with
  throwaway names: `cx.report(cx.diagnostic(range, message).with_help(help))`.
- **No repeated work.** Compute a value once and name it. Two near-identical helpers in
  different files become one shared helper.
- **Typed, not stringly typed.** Kubernetes kinds, groups and other closed sets get a
  small type or helper in `kubelint_k8s` instead of string literals repeated across
  rules.
- **No borrow-checker escape hatches.** A lifetime error means the design is wrong:
  restructure instead of reaching for `.clone()`, `Rc<RefCell<_>>` or `'static`.

## Toolchain and lints

- The toolchain is pinned in `rust-toolchain.toml`, and `rust-version` in `Cargo.toml`
  equals it. Bumping it is its own `chore` PR that also fixes any new lints.
- Edition 2024, Cargo resolver 3.
- Workspace lints: `unsafe_code = "forbid"`, clippy `pedantic` as warnings with a short
  allow list in `Cargo.toml`, plus `unwrap_used`, `dbg_macro` and `todo` (tests are
  exempt through `clippy.toml`). CI denies all warnings.

**Why pinned:** a new Rust release adds lints, and an unpinned toolchain would turn CI red
overnight with no code change.

**Rejected:**

- Tracking `stable`. Always current, but CI results change without a commit.
- Default clippy only. `pedantic` catches real issues (casts, needless clones, missing
  `#[must_use]` on builders) and keeps the codebase uniform.

## Kubernetes correctness

- A rule's behavior must match what Kubernetes actually does. When in doubt, read the
  Kubernetes source or official docs and cite it in a comment.
- A rule that cannot decide (fields missing, unknown kind, templated values) stays
  silent. False positives are worse than false negatives.
- Rules that look at containers use `pod_spec().containers()` from `kubelint_k8s` so every
  workload kind and every container type is covered consistently.

## Tests

- Every behavior change comes with tests. Every bug fix starts with a failing test.
- Anything whose output is text uses `insta` snapshot tests: rule results, renderers,
  CLI runs, YAML tree dumps. CLI tests run the real binary with `assert_cmd` and
  snapshot stdout, stderr and the exit code.
- Rule fixtures live in
  `crates/kubelint_linter/resources/test/fixtures/<category>/<CODE>.yaml`. The harness
  lints a fixture with only that rule enabled and snapshots the rendered findings.
- Pure logic (parsers, validators, selector resolution) gets table-driven unit tests.
- Tests live in their own files, never in a source file. Integration tests go in
  `crates/<crate>/tests/`. Tests that need private items go in `<module>/tests.rs`,
  declared with `#[cfg(test)] mod tests;`. Doc examples are the one exception.
- No inline snapshots (`@"..."`): expected output goes in `.snap` files.
- Rule fixtures contain both violating and passing cases, each preceded by a comment
  explaining the case.
- Fixtures include tricky inputs: non-ASCII text, flow-style YAML, multiple documents,
  every relevant workload kind.
- Update snapshots with `INSTA_UPDATE=always cargo test --workspace`, then read every
  changed `.snap` file before committing. Never commit `*.snap.new` files.

**Why snapshots:** a linter's output is "which findings, at which positions, with which
text". Hand-written assertions for that are tedious and brittle, and a rule's fixture
and snapshot double as documentation of its behavior.

**Why tests have their own files:** a source file holds only product code.

**Rejected:**

- Expected findings written inside fixtures, like rustc's `//~ ERROR`. Precise, but it
  needs a custom harness.
- Rust's usual inline `mod tests { ... }`. It puts test code below every module.

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

- **Where docs live:** contributor docs in `docs/`, user docs in `docs/user/`, the rule
  catalog in `docs/rules/`, the routine's manual in `docs/agent/playbook.md`. The root
  keeps only `README.md`, `CLAUDE.md` and `LICENSE`.
- **Same PR rule:** a PR that changes behavior updates the docs that describe it.
- **Decisions live in the doc they are about,** written as plain statements in the same
  PR as the change: a public contract in `docs/user/`, a design choice in
  `architecture.md`, a code or process rule in this file. Each decision gets a bold
  **Why:** line when the reason is not obvious and a bold **Rejected:** line naming the
  alternatives that lost, so nobody argues the question again. To reverse one, edit it
  in place and explain the reversal in the PR description. Write a decision down when it
  adds or replaces a core dependency, changes a public contract (CLI flags, config
  format, rule codes, output schemas), changes crate boundaries, or answers a question a
  newcomer would reasonably ask ("why is it like this?").
- **Prose style:**
  - Plain, direct sentences that make sense without prior context.
  - No dashes as punctuation: no em dash, no en dash, no hyphen standing in for one.
    End the sentence with a period and start a new one, or use a colon, a comma or
    parentheses. Compound words (`non-string`, `per-file`) are fine.
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
- The **Concepts** section explains 2 to 4 ideas the PR relies on, in plain language.
