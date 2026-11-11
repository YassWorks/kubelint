//! The built-in rules.
//!
//! Adding a rule: one file and its `.md` explanation under the category's module, one
//! fixture, one line in [`ALL`]. See `docs/CONTRIBUTING.md`.

pub mod correctness;
pub mod reliability;
pub mod security;
pub mod syntax;
#[cfg(test)]
mod tests;

use kubelint_k8s::Container;
use kubelint_source::TextRange;
use kubelint_yaml::Node;

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
    &security::PrivilegedContainer,
    &security::PrivilegeEscalationAllowed,
    &security::RunAsRoot,
    &security::HostNamespace,
    &security::HostPathVolume,
    &security::DangerousCapability,
    &security::DockerSocketMount,
    &security::SecretInEnvLiteral,
    &security::WritableRootFilesystem,
    &reliability::MissingMemoryLimit,
    &reliability::MissingResourceRequests,
    &reliability::UnpinnedImageTag,
    &reliability::MissingReadinessProbe,
    &reliability::SingleReplica,
];

/// "container `web`", or "container" when it has no usable name.
fn container_subject(container: Container<'_>) -> String {
    match container.name().and_then(Node::as_str) {
        Some(name) => format!("container `{name}`"),
        None => "container".to_owned(),
    }
}

/// Where a finding about a missing setting of `container` points: its `name`, or its
/// first key when it has none.
fn container_range(container: Container<'_>) -> TextRange {
    container
        .name()
        .or_else(|| container.node().entries().next().map(|entry| &entry.key))
        .map_or(container.node().range, |node| node.range)
}
