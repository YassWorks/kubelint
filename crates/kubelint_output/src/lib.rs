//! Output formats.
//!
//! Renders diagnostics for people (rustc-style snippets via `annotate-snippets`) and for
//! machines (JSON, SARIF, GitHub Actions workflow commands).
//!
//! Allowed internal dependencies: `kubelint_source`, `kubelint_diagnostics`.
//! See `docs/architecture.md`.

mod human;
mod json;

pub use human::{HumanRenderer, Summary};
pub use json::{JSON_SCHEMA_VERSION, JsonRenderer};
