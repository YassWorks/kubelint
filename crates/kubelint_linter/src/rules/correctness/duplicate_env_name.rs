//! KL1011 `duplicate-env-name`.

use std::collections::HashMap;

use kubelint_diagnostics::Severity;
use kubelint_yaml::Node;

use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports an env var whose `name` is already set earlier in the same container's `env`.
///
/// The API server accepts the object with a "hides previous definition" warning
/// (`pkg/api/pod/warnings.go`) and the kubelet keeps the last value. Each repeat is
/// reported once, pointing back at the first entry with that name.
pub struct DuplicateEnvName;

static META: RuleMeta = RuleMeta {
    code: "KL1011",
    name: "duplicate-env-name",
    category: Category::Correctness,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "The same env var name is set twice in one container.",
    explanation: include_str!("duplicate_env_name.md"),
};

impl Rule for DuplicateEnvName {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        for container in pod.containers() {
            let mut first: HashMap<&str, &Node> = HashMap::new();
            for var in container.env().into_iter().flat_map(Node::items) {
                let Some(node) = var.get("name") else {
                    continue;
                };
                let Some(name) = node.as_str().filter(|name| !name.trim().is_empty()) else {
                    continue;
                };
                let Some(original) = first.get(name) else {
                    first.insert(name, node);
                    continue;
                };
                cx.report(
                    cx.diagnostic(
                        node.range,
                        format!("env var `{name}` is already set in this container"),
                    )
                    .with_secondary(cx.label(original.range).with_message("first set here"))
                    .with_help("remove one of the entries; only the last one takes effect"),
                );
            }
        }
    }
}
