//! Diagnostics.
//!
//! Defines what a finding is: rule code, severity, message, labelled ranges, help text and
//! optional fixes. Contains no rendering code; renderers live in `kubelint_output`.
//!
//! Allowed internal dependencies: `kubelint_source`. See `docs/architecture.md`.
