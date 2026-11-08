//! `KL2xxx`: settings that weaken the isolation between a container and its node.

mod privilege_escalation_allowed;
mod privileged_container;

pub use privilege_escalation_allowed::PrivilegeEscalationAllowed;
pub use privileged_container::PrivilegedContainer;

use kubelint_k8s::Container;
use kubelint_yaml::Node;

/// "container `web`", or "container" when it has no usable name.
fn container_subject(container: Container<'_>) -> String {
    match container.name().and_then(Node::as_str) {
        Some(name) => format!("container `{name}`"),
        None => "container".to_owned(),
    }
}
