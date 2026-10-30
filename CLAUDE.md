# CLAUDE.md

kubelint is a fast, friendly linter for Kubernetes manifests, written in Rust. It is
developed mostly by an autonomous daily Claude Code routine. The repository is the
project's memory.

## Start here

- **Autonomous routine run:** read [`docs/agent/playbook.md`](docs/agent/playbook.md) in
  full and follow it. It defines the whole run.
- **Any other session:** read [`docs/vision.md`](docs/vision.md),
  [`docs/architecture.md`](docs/architecture.md) and
  [`docs/conventions.md`](docs/conventions.md) before changing code.

## Commands

CI runs exactly these. All must pass before a PR is opened.

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked
```

Accept changed `insta` snapshots, then review the `.snap` diffs:

```bash
INSTA_UPDATE=always cargo test --workspace
```

## Layout

- `crates/kubelint_*`: library crates, one per layer. Dependencies point downward only
  (table in `docs/architecture.md`).
- `crates/kubelint`: the CLI binary. Keep it thin.
- `docs/`: vision, roadmap, architecture, conventions, ADRs (`decisions/`), rule catalog
  (`rules/`), agent memory (`agent/`), learning notes for the owner (`learning/`).

## Rules of thumb

- One roadmap task per PR, small, using `.github/pull_request_template.md`.
- Rule codes are allocated only in `docs/rules/README.md`.
- Positions are byte offsets (`TextRange`). `saphyr-parser` gives character indices:
  convert them in `kubelint_yaml`.
- No `unwrap()` on user input. Silence lints only with
  `#[expect(clippy::..., reason = "...")]`.
- Prose: plain sentences, no em dashes.
