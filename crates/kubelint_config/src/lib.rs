//! Configuration.
//!
//! Finds and parses `kubelint.toml` / `.kubelint.toml`, merges it with command-line
//! overrides and exposes the resolved settings (ADR 0005).
//!
//! Allowed internal dependencies: `kubelint_source`. See `docs/architecture.md`.
