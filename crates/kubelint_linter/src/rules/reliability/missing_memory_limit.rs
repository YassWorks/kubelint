//! KL3001 `missing-memory-limit`.

use kubelint_diagnostics::Severity;
use kubelint_k8s::ContainerKind;
use kubelint_source::TextRange;
use kubelint_yaml::Entry;

use crate::rules::{container_range, container_subject};
use crate::{Category, ObjectContext, Rule, RuleMeta};

/// Reports a regular or init container without `resources.limits.memory`.
///
/// The API server rejects `resources` on ephemeral containers
/// (`allowedEphemeralContainerFields` in `pkg/apis/core/validation/validation.go`), so
/// they are skipped. A `resources` or `limits` value that is neither a mapping nor null
/// is a schema problem and is not reported.
pub struct MissingMemoryLimit;

static META: RuleMeta = RuleMeta {
    code: "KL3001",
    name: "missing-memory-limit",
    category: Category::Reliability,
    default_severity: Severity::Warning,
    default_enabled: true,
    summary: "A container has no `resources.limits.memory`.",
    explanation: include_str!("missing_memory_limit.md"),
};

impl Rule for MissingMemoryLimit {
    fn meta(&self) -> &'static RuleMeta {
        &META
    }

    fn check_object(&self, cx: &mut ObjectContext<'_>) {
        let Some(pod) = cx.object().pod_spec() else {
            return;
        };
        if matches!(
            missing_limit(pod.node().get_entry("resources")),
            Missing::No
        ) {
            return;
        }
        let containers = pod
            .containers()
            .filter(|container| container.kind() != ContainerKind::Ephemeral);
        for container in containers {
            let Missing::Yes(range) = missing_limit(container.node().get_entry("resources")) else {
                continue;
            };
            cx.report(
                cx.diagnostic(
                    range.unwrap_or_else(|| container_range(container)),
                    format!("{} has no memory limit", container_subject(container)),
                )
                .with_help("set `resources.limits.memory`, for example `memory: 512Mi`"),
            );
        }
    }
}

enum Missing {
    /// The limit is missing. The range is the deepest key on the way to it, `limits`
    /// or `resources`, when there is one.
    Yes(Option<TextRange>),
    /// The limit is set, or a value on the way to it is a schema problem.
    No,
}

fn missing_limit(resources: Option<&Entry>) -> Missing {
    let mut found = None;
    let mut entry = resources;
    for key in ["limits", "memory"] {
        let Some(current) = entry.filter(|entry| !entry.value.is_kubernetes_null()) else {
            return Missing::Yes(found);
        };
        if current.value.as_mapping().is_none() {
            return Missing::No;
        }
        found = Some(current.key.range);
        entry = current.value.get_entry(key);
    }
    match entry {
        Some(memory) if !memory.value.is_kubernetes_null() => Missing::No,
        _ => Missing::Yes(found),
    }
}
