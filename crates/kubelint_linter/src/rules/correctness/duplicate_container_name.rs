//! KL1006 `duplicate-container-name`.

use std::collections::HashMap;

use kubelint_diagnostics::Severity;
use kubelint_k8s::Container;
use kubelint_yaml::Node;

use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a container whose name is already used by another container of the same
/// pod. Regular, init and ephemeral containers share one set of names, as in
/// Kubernetes' validation. Each repeat is reported once, pointing back at the first
/// container in the file with that name.
pub struct DuplicateContainerName;

static META: RuleMeta = RuleMeta {
    code: "KL1006",
    name: "duplicate-container-name",
    category: Category::Correctness,
    default_severity: Severity::Error,
    default_enabled: true,
    summary: "Two containers in one pod share a name.",
    explanation: include_str!("duplicate_container_name.md"),
};

impl Rule for DuplicateContainerName {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        let mut containers: Vec<(Container<'_>, &Node, &str)> = pod
            .containers()
            .filter_map(|container| {
                let node = container.name()?;
                let name = node.as_str()?;
                (!name.trim().is_empty()).then_some((container, node, name))
            })
            .collect();
        // `containers()` groups by list; "first" means first in the file.
        containers.sort_by_key(|(container, _, _)| container.node().range.start());
        let mut first: HashMap<&str, (Container<'_>, &Node)> = HashMap::new();
        for (container, name_node, name) in containers {
            let Some(&(original, original_name)) = first.get(name) else {
                first.insert(name, (container, name_node));
                continue;
            };
            let message = format!(
                "container name `{name}` is already used in `{}`",
                original.kind().field()
            );
            cx.report(
                cx.diagnostic(name_node.range, message)
                    .with_secondary(
                        cx.label(original_name.range)
                            .with_message("first used here"),
                    )
                    .with_help("give each container in the pod a unique name"),
            );
        }
    }
}
