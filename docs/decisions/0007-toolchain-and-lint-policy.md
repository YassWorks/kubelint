# 0007. Toolchain and lint policy

- Status: Accepted
- Date: 2026-10-30

## Context

CI runs `clippy` with warnings as errors. New Rust releases add lints, so an unpinned
toolchain can turn CI red overnight with no code change, which would stall an autonomous
workflow.

## Decision

- **Toolchain pinned** in `rust-toolchain.toml` (1.97.0 at the time of writing), with
  `rust-version` in `Cargo.toml` equal to it. Bumping is its own `chore` PR that fixes any
  new lints in the same PR.
- **Edition 2024**, Cargo resolver 3.
- **Workspace lints:** `unsafe_code = "forbid"`, `missing_docs = "warn"`, clippy
  `pedantic` as warnings with a short, documented allow list, plus `unwrap_used`,
  `dbg_macro` and `todo` (tests exempted via `clippy.toml`). CI denies all warnings.
- **Allowing a lint locally** requires `#[expect(clippy::lint_name, reason = "...")]`
  (not `#[allow]`) so the exception disappears when it is no longer needed.

## Alternatives considered

- **Track `stable`.** Always current, but non-deterministic CI.
- **Default clippy only.** Less friction, but pedantic catches real issues (casts,
  needless clones, missing `#[must_use]` on builders) and keeps a growing codebase
  uniform.

## Consequences

- Occasional pedantic friction, handled with `#[expect(..., reason = "...")]`.
- Public items always have docs, which keeps the codebase self-explanatory for new
  contributors and the owner.
