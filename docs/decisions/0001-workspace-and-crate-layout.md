# 0001. Workspace and crate layout

- Status: Accepted
- Date: 2026-10-30

## Context

kubelint is meant to grow large: a CLI, CI integrations, schema validation, a language
server. Code that starts in one crate tends to become a tangle where everything depends on
everything, which slows compiles and makes the language server and future embeddings
hard to build.

## Decision

A Cargo workspace with one crate per layer under `crates/`, prefixed `kubelint_`:
`source`, `yaml`, `diagnostics`, `k8s`, `config`, `linter`, `output`, and the `kubelint`
binary. Dependencies point downward only, as listed in `docs/architecture.md`. Shared
metadata, dependency versions and lints are inherited from the workspace root.

New crates are created when their milestone starts (`kubelint_schema`, `kubelint_lsp`,
`xtask`), never speculatively.

## Alternatives considered

- **Single crate with modules.** Simplest to start, but module boundaries are not
  enforced, and the language server would pull in CLI-only dependencies.
- **Fewer, larger crates** (for example `core` + `cli`). Fewer `Cargo.toml` files, but
  `core` becomes the tangle. Ruff and rust-analyzer both use many small crates for this
  reason.

## Consequences

- Clear ownership: every type has one obvious home.
- Parallel compilation and smaller rebuilds.
- Slight overhead when a change spans several crates. That is a feature: it makes
  cross-layer changes visible in review.
- Publishing to crates.io requires publishing every internal crate (M3-07). They are
  documented as internal, with no stability guarantee.
