//! Configuration.
//!
//! Finds and parses `kubelint.toml` / `.kubelint.toml` (ADR 0005). Every value keeps the
//! byte range it was written at, so errors found later (an unknown rule selector) can
//! point at the exact spot in the file. Rule selectors are plain strings here: resolving
//! them needs the rule registry, which lives in `kubelint_linter` (ADR 0009).
//!
//! ```
//! let config = kubelint_config::parse(r#"
//! [lint]
//! select = ["correctness", "security"]
//! fail-on = "error"
//! "#).unwrap();
//! assert_eq!(config.lint.select.unwrap()[1].value, "security");
//! assert_eq!(config.lint.fail_on.unwrap().value, kubelint_config::Level::Error);
//! ```
//!
//! Allowed internal dependencies: `kubelint_source`. See `docs/architecture.md`.

mod discover;
mod parse;

pub use discover::{FILE_NAMES, find};
pub use parse::{Config, ConfigError, FilesConfig, Level, LintConfig, Spanned, parse};
