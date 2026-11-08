<!--
Title format: Conventional Commits, e.g. `feat(linter): add KL2001 privileged-container`.
One PR = one roadmap item = one concern. Keep every section short.
-->

## Summary

<!-- 1-3 sentences: what this PR does and why, in plain language. -->

## Roadmap item

<!-- The item from docs/roadmap.md, e.g. `KL2001 privileged-container`, or `docs` / `fix` / `chore` with a reason. -->

## Changes

<!-- Bullet list of the concrete changes. If a bullet is unrelated to the item, it belongs in another PR. -->

-

## How it was verified

<!-- Commands run locally and what they showed. Mention new tests and snapshots. -->

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- [ ] `cargo test --workspace --all-features --locked`
- [ ] `cargo doc --workspace --no-deps --all-features --locked`

## Docs

- [ ] Roadmap item ticked (or split) in `docs/roadmap.md`
- [ ] User-facing behavior change reflected in docs (README, `docs/`), or none
- [ ] New decision written into the doc it concerns, with **Why** and **Rejected**, or none

## Size check

<!-- Rough count of changed lines, excluding snapshots and YAML fixtures.
Over ~400 lines? Split the PR unless it is mechanical (renames, generated docs). -->

## Out of scope / follow-ups

<!-- Things noticed but not done here, and where they were recorded in docs/roadmap.md. -->
