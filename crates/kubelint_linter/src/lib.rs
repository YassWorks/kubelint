//! The linter.
//!
//! Defines the `Rule` trait and rule metadata, holds the registry of built-in rules, applies
//! configuration and suppression comments, and runs rules over Kubernetes objects to produce
//! diagnostics.
//!
//! Allowed internal dependencies: `kubelint_source`, `kubelint_yaml`, `kubelint_k8s`,
//! `kubelint_diagnostics`, `kubelint_config`. See `docs/architecture.md`.

mod context;
mod engine;
mod rule;
pub mod rules;
mod selection;
mod suppression;
#[cfg(test)]
mod testing;

pub use context::{LintObject, ObjectContext, RunContext};
pub use engine::{EnabledRule, LintResult, Linter};
pub use rule::{Category, Rule, RuleMeta};
pub use selection::{RuleSelection, SelectionError, SelectionErrorKind, SelectorList, fails};
