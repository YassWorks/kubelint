//! `KL2xxx`: settings that weaken the isolation between a container and its node.

mod host_namespace;
mod privilege_escalation_allowed;
mod privileged_container;
mod run_as_root;

pub use host_namespace::HostNamespace;
pub use privilege_escalation_allowed::PrivilegeEscalationAllowed;
pub use privileged_container::PrivilegedContainer;
pub use run_as_root::RunAsRoot;

use kubelint_k8s::Container;
use kubelint_source::TextRange;
use kubelint_yaml::Node;

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
