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
    explanation: r#"## What it does

Reports a container whose `name` is already used by another container in the same pod,
counting `containers`, `initContainers` and `ephemeralContainers` together.

## Why it matters

Kubernetes refers to containers by name: in `kubectl logs`, `kubectl exec`, status
reports and probes. Names must be unique within a pod, so the API server rejects the
object with "Duplicate value". This usually happens when a container block is copied to
add a sidecar and the name is not changed.

## Example

```yaml
spec:
  containers:
  - name: web
    image: nginx:1.27
  - name: web
    image: envoyproxy/envoy:v1.31.0
```

Use instead:

```yaml
spec:
  containers:
  - name: web
    image: nginx:1.27
  - name: proxy
    image: envoyproxy/envoy:v1.31.0
```
"#,
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
            let label = cx
                .label(original_name.range)
                .with_message("first used here");
            let diagnostic = cx
                .diagnostic(name_node.range, message)
                .with_secondary(label)
                .with_help("give each container in the pod a unique name");
            cx.report(diagnostic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DuplicateContainerName;

    #[test]
    fn fixture() {
        insta::assert_snapshot!(crate::testing::check_fixture(&DuplicateContainerName));
    }
}
