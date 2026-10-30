<!--
Title format: Conventional Commits, e.g. `feat(linter): add KL2001 privileged-container`.
One PR = one roadmap task = one concern. Keep every section short.
Sections marked (required) are never deleted. Write "None" instead.
-->

## Summary (required)

<!-- 1-3 sentences: what this PR does and why, in plain language. -->

## Roadmap task (required)

<!-- The task ID from docs/roadmap.md, e.g. `M1-04`, or `docs` / `fix` / `chore` with a reason. -->

## Changes (required)

<!-- Bullet list of the concrete changes. If a bullet is unrelated to the task, it belongs in another PR. -->

-

## How it was verified (required)

<!-- Commands run locally and what they showed. Mention new tests and snapshots. -->

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- [ ] `cargo test --workspace --all-features --locked`
- [ ] `cargo doc --workspace --no-deps --all-features --locked`

## Docs and memory (required)

- [ ] Roadmap checkbox ticked (or task split) in `docs/roadmap.md`
- [ ] User-facing behavior change reflected in docs (README, `docs/`), or none
- [ ] New architectural decision recorded as an ADR in `docs/decisions/`, or none
- [ ] New Rust/Kubernetes concept added to `docs/learning/glossary.md`, or none

## Size check (required)

<!-- Rough count of changed lines, excluding snapshots and YAML fixtures.
Over ~400 lines? Split the PR unless it is mechanical (renames, generated docs). -->

## Out of scope / follow-ups (required)

<!-- Things noticed but deliberately not done here, and where they were recorded (roadmap, owner inbox). -->

## Concepts (required)

<!-- 2-4 plain-language bullets on the Rust or Kubernetes ideas this PR relies on, not the diff.
Link the glossary entry when one exists. -->

-
