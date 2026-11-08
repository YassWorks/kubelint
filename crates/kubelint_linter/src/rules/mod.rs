//! The built-in rules.
//!
//! Adding a rule: one file and its `.md` explanation under the category's module, one
//! fixture, one line in [`ALL`]. See `docs/CONTRIBUTING.md`.

pub mod correctness;
pub mod syntax;
#[cfg(test)]
mod tests;

use crate::Rule;

/// Every built-in rule, in code order. Tests snapshot each rule's fixture and check that
/// codes and names are unique and well-formed.
pub static ALL: &[&dyn Rule] = &[
    &syntax::InvalidYaml,
    &syntax::DuplicateKey,
    &syntax::InvalidSuppression,
    &correctness::MissingApiVersion,
    &correctness::MissingKind,
    &correctness::MissingObjectName,
    &correctness::DuplicateObject,
    &correctness::SelectorMismatch,
    &correctness::DuplicateContainerName,
    &correctness::InvalidResourceQuantity,
    &correctness::InvalidObjectName,
    &correctness::NonStringEnvValue,
    &correctness::InvalidLabel,
    &correctness::DuplicateEnvName,
];
